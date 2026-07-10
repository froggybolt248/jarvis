// WP-Agent-Tools owns this file.

//! Built-in vault tools: `vault_search` (hybrid retrieval), `vault_read`
//! (raw note contents), and `vault_append` (the sole mutating tool, which
//! also logs a Quiet Feed audit row).

use anyhow::{Context, Result};
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::quiet_feed::QuietFeedItem;

use super::{Tool, ToolContext};

const DEFAULT_K: usize = 6;
const MAX_K: usize = 20;
const MAX_READ_CHARS: usize = 8000;
const MAX_FEED_SUMMARY_CHARS: usize = 200;

/// Extract a required string field from a JSON object.
fn required_str<'a>(args: &'a Value, field: &str) -> Result<&'a str> {
    args.get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("missing or non-string required field '{field}'"))
}

/// Extract an optional non-negative integer field, clamped to `[1, max]`,
/// defaulting to `default` when absent.
fn optional_k(args: &Value, field: &str, default: usize, max: usize) -> Result<usize> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let n = v
                .as_u64()
                .with_context(|| format!("field '{field}' must be a non-negative integer"))?;
            Ok((n as usize).clamp(1, max))
        }
    }
}

/// Read-only semantic + full-text search over the vault's indexed chunks.
pub struct VaultSearch;

#[async_trait::async_trait]
impl Tool for VaultSearch {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "vault_search".to_string(),
            description: "Search the user's personal vault (notes) using hybrid full-text \
                and semantic search. Use this before answering from memory so answers can be \
                grounded and cited."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query."
                    },
                    "k": {
                        "type": "integer",
                        "description": "Max number of results to return (default 6, max 20).",
                        "minimum": 1,
                        "maximum": MAX_K
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let query = required_str(args, "query")?;
        let k = optional_k(args, "k", DEFAULT_K, MAX_K)?;

        let mut embeddings = ctx.provider.embed(vec![query.to_string()]).await?;
        let embedding = if embeddings.is_empty() {
            anyhow::bail!("embedding provider returned no vectors");
        } else {
            embeddings.remove(0)
        };

        let hits = ctx.db.hybrid_search(query, &embedding, k)?;
        if hits.is_empty() {
            return Ok("No matching notes found.".to_string());
        }

        let mut out = String::new();
        for (i, hit) in hits.iter().enumerate() {
            let heading_suffix = hit
                .heading
                .as_ref()
                .map(|h| format!(" \u{203a} {h}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "[{}] {}{}\n{}\n\n",
                i + 1,
                hit.source_path,
                heading_suffix,
                hit.content
            ));
        }
        Ok(out.trim_end().to_string())
    }
}

/// Read-only raw contents of a single vault note.
pub struct VaultRead;

#[async_trait::async_trait]
impl Tool for VaultRead {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "vault_read".to_string(),
            description: "Read the full raw contents of a note in the user's vault, given its \
                path relative to the vault root."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the note, relative to the vault root."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let path = required_str(args, "path")?;
        let content = ctx.vault.read(path)?;
        if content.len() > MAX_READ_CHARS {
            let mut truncated = content.chars().take(MAX_READ_CHARS).collect::<String>();
            truncated.push_str("\u{2026}(truncated)");
            Ok(truncated)
        } else {
            Ok(content)
        }
    }
}

/// The sole mutating tool: appends a section to a vault note and logs a
/// Quiet Feed row as the audit record of the mutation.
pub struct VaultAppend;

#[async_trait::async_trait]
impl Tool for VaultAppend {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "vault_append".to_string(),
            description: "Append a new section (or add to an existing section) in a vault \
                note. This mutates the user's vault, so only use it when the user has clearly \
                asked to record something."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the note, relative to the vault root."
                    },
                    "heading": {
                        "type": "string",
                        "description": "The '## heading' section to append under."
                    },
                    "body": {
                        "type": "string",
                        "description": "The content to append under the heading."
                    }
                },
                "required": ["path", "heading", "body"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let path = required_str(args, "path")?;
        let heading = required_str(args, "heading")?;
        let body = required_str(args, "body")?;

        ctx.vault.append_section(path, heading, body)?;

        let summary: String = if body.len() > MAX_FEED_SUMMARY_CHARS {
            let mut s = body.chars().take(MAX_FEED_SUMMARY_CHARS).collect::<String>();
            s.push('\u{2026}');
            s
        } else {
            body.to_string()
        };

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "note".to_string(),
            title: format!("Noted in {path}"),
            body: Some(summary),
            deep_link: None,
            source: Some("vault_append".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!("Appended to '{path}' under '## {heading}'."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ChatProvider, ProviderHealth};
    use crate::core::db::Db;
    use crate::core::memory::Vault;
    use futures_util::stream::BoxStream;
    use tempfile::tempdir;

    struct StubProvider;

    #[async_trait::async_trait]
    impl ChatProvider for StubProvider {
        async fn chat_stream(
            &self,
            _messages: Vec<ChatMessage>,
            _opts: ChatOptions,
        ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
            anyhow::bail!("not implemented in stub")
        }

        async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .map(|t| {
                    // Deterministic, distinct-per-text 768-dim vector.
                    let seed = t.bytes().map(|b| b as f32).sum::<f32>();
                    (0..768).map(|i| seed + i as f32 * 0.001).collect()
                })
                .collect())
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("not implemented in stub")
        }
    }

    #[tokio::test]
    async fn vault_search_returns_expected_source_path() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;

        let rowid_a = db
            .upsert_chunk(
                "a",
                "Knowledge/thermo.md",
                Some("Entropy"),
                "entropy is a measure of disorder in a thermodynamic system",
                "2026-01-01T00:00:00Z",
            )
            .unwrap();
        let rowid_b = db
            .upsert_chunk(
                "b",
                "Knowledge/unrelated.md",
                None,
                "the weather today is sunny with a light breeze",
                "2026-01-01T00:00:00Z",
            )
            .unwrap();

        // Embeddings must be distinct per text: reuse the stub's own scheme.
        let emb_a: Vec<f32> = {
            let seed = "entropy is a measure of disorder in a thermodynamic system"
                .bytes()
                .map(|b| b as f32)
                .sum::<f32>();
            (0..768).map(|i| seed + i as f32 * 0.001).collect()
        };
        let emb_b: Vec<f32> = {
            let seed = "the weather today is sunny with a light breeze"
                .bytes()
                .map(|b| b as f32)
                .sum::<f32>();
            (0..768).map(|i| seed + i as f32 * 0.001).collect()
        };
        db.set_chunk_embedding(rowid_a, &emb_a).unwrap();
        db.set_chunk_embedding(rowid_b, &emb_b).unwrap();

        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let tool = VaultSearch;
        let args = serde_json::json!({"query": "entropy is a measure of disorder in a thermodynamic system"});
        let result = tool.execute(&ctx, &args).await.unwrap();

        assert!(result.contains("Knowledge/thermo.md"), "got: {result}");
        assert!(result.contains("Entropy"), "got: {result}");
    }

    #[tokio::test]
    async fn vault_search_returns_message_when_no_hits() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let tool = VaultSearch;
        let args = serde_json::json!({"query": "anything"});
        let result = tool.execute(&ctx, &args).await.unwrap();
        assert_eq!(result, "No matching notes found.");
    }

    #[tokio::test]
    async fn vault_search_missing_query_errors() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let tool = VaultSearch;
        let args = serde_json::json!({});
        assert!(tool.execute(&ctx, &args).await.is_err());
    }

    #[tokio::test]
    async fn vault_read_and_append_round_trip_with_feed_audit() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let append = VaultAppend;
        let args = serde_json::json!({
            "path": "Knowledge/scratch.md",
            "heading": "Ideas",
            "body": "- Try a new study technique"
        });
        let confirmation = append.execute(&ctx, &args).await.unwrap();
        assert!(confirmation.contains("Knowledge/scratch.md"));

        let read = VaultRead;
        let read_args = serde_json::json!({"path": "Knowledge/scratch.md"});
        let content = read.execute(&ctx, &read_args).await.unwrap();
        assert!(content.contains("## Ideas"));
        assert!(content.contains("Try a new study technique"));

        let feed = db.recent_feed(10).unwrap();
        assert_eq!(feed.len(), 1);
        assert_eq!(feed[0].source.as_deref(), Some("vault_append"));
        assert!(feed[0].title.contains("Knowledge/scratch.md"));
    }

    #[tokio::test]
    async fn vault_read_truncates_long_files() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let long_content = "x".repeat(MAX_READ_CHARS + 500);
        vault.write("Knowledge/long.md", &long_content).unwrap();

        let read = VaultRead;
        let args = serde_json::json!({"path": "Knowledge/long.md"});
        let result = read.execute(&ctx, &args).await.unwrap();
        assert!(result.ends_with("\u{2026}(truncated)"));
        assert!(result.len() < long_content.len());
    }

    #[tokio::test]
    async fn vault_append_missing_field_errors() {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };

        let append = VaultAppend;
        let args = serde_json::json!({"path": "Knowledge/scratch.md", "heading": "Ideas"});
        assert!(append.execute(&ctx, &args).await.is_err());
    }
}
