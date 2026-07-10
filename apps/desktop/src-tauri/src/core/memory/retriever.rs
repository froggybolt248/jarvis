// WP-Memory-RAG owns this file.
//! Query-time retrieval: embed the query, run hybrid search, and render the
//! results into a citable context block for the system prompt.

use anyhow::Result;

use crate::core::agent::provider::ChatProvider;
use crate::core::db::queries::search::SearchHit;
use crate::core::db::Db;

/// Embed the query (single embed call) and run hybrid search for the top k
/// hits.
pub async fn retrieve(
    db: &Db,
    provider: &dyn ChatProvider,
    query: &str,
    k: usize,
) -> Result<Vec<SearchHit>> {
    let mut embeddings = provider.embed(vec![query.to_string()]).await?;
    let embedding = embeddings.pop().unwrap_or_default();
    db.hybrid_search(query, &embedding, k)
}

/// Render hits into a numbered, citable context block for a system prompt,
/// e.g. `"[1] Knowledge/welcome.md › Getting started\n<content>\n\n[2] ..."`.
/// Returns an empty string for no hits. Citation numbers are 1-based and
/// match hit order.
pub fn render_context(hits: &[SearchHit]) -> String {
    if hits.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for (i, hit) in hits.iter().enumerate() {
        let n = i + 1;
        match &hit.heading {
            Some(heading) if !heading.is_empty() => {
                out.push_str(&format!("[{n}] {} \u{203a} {}\n", hit.source_path, heading));
            }
            _ => {
                out.push_str(&format!("[{n}] {}\n", hit.source_path));
            }
        }
        out.push_str(&hit.content);
        out.push_str("\n\n");
    }
    // Trim the trailing double newline so callers get a clean block.
    out.truncate(out.trim_end_matches('\n').len());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ProviderHealth};
    use crate::core::memory::embedder::index_source;
    use futures_util::stream::BoxStream;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    struct StubProvider;

    fn embed_one(text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; 768];
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();
        for (i, slot) in v.iter_mut().enumerate().take(8) {
            let shifted = seed.rotate_left((i * 8) as u32);
            *slot = ((shifted & 0xFFFF) as f32) / 65535.0;
        }
        v
    }

    #[async_trait::async_trait]
    impl ChatProvider for StubProvider {
        async fn chat_stream(
            &self,
            _messages: Vec<ChatMessage>,
            _opts: ChatOptions,
        ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
            anyhow::bail!("unused in tests")
        }

        async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|t| embed_one(t)).collect())
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("unused in tests")
        }
    }

    #[tokio::test]
    async fn retrieve_ranks_relevant_chunk_first_and_context_cites_source() {
        let db = crate::core::db::Db::open_in_memory().unwrap();
        let provider = StubProvider;

        index_source(
            &db,
            &provider,
            "Knowledge/welcome.md",
            "# Getting started\n\nJarvis is a local personal assistant that helps you organize your notes.\n",
            "2026-01-01T00:00:00Z",
        )
        .await
        .unwrap();

        index_source(
            &db,
            &provider,
            "Knowledge/unrelated.md",
            "# Unrelated\n\nThis document discusses gardening tips for tomatoes.\n",
            "2026-01-01T00:00:00Z",
        )
        .await
        .unwrap();

        let hits = retrieve(&db, &provider, "local personal assistant notes", 5)
            .await
            .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].source_path, "Knowledge/welcome.md");

        let context = render_context(&hits);
        assert!(context.contains("Knowledge/welcome.md"));
        assert!(context.starts_with("[1]"));
    }

    #[test]
    fn render_context_empty_for_no_hits() {
        assert_eq!(render_context(&[]), String::new());
    }

    #[test]
    fn render_context_numbers_hits_and_includes_heading() {
        let hits = vec![
            SearchHit {
                chunk_id: "a".into(),
                source_path: "Knowledge/welcome.md".into(),
                heading: Some("Getting started".into()),
                content: "Hello world.".into(),
                score: 1.0,
            },
            SearchHit {
                chunk_id: "b".into(),
                source_path: "Knowledge/other.md".into(),
                heading: None,
                content: "Second chunk.".into(),
                score: 0.5,
            },
        ];
        let rendered = render_context(&hits);
        assert!(rendered.contains("[1] Knowledge/welcome.md"));
        assert!(rendered.contains("Getting started"));
        assert!(rendered.contains("Hello world."));
        assert!(rendered.contains("[2] Knowledge/other.md"));
        assert!(rendered.contains("Second chunk."));
    }
}
