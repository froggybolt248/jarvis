// WP-Memory-RAG owns this file.
//! Indexing pipeline: chunk a markdown source, embed the chunks in a single
//! batch call to the provider, and persist both the chunk rows and their
//! embeddings.

use anyhow::{bail, Result};

use crate::core::agent::provider::ChatProvider;
use crate::core::db::Db;
use crate::core::memory::chunker::chunk_markdown;
use crate::core::memory::Vault;

/// Re-index one source file: delete its existing chunks, re-chunk, upsert
/// each, batch-embed all chunk contents in a single provider.embed call, and
/// store each vector. Returns the number of chunks indexed.
pub async fn index_source(
    db: &Db,
    provider: &dyn ChatProvider,
    source_path: &str,
    content: &str,
    updated_at: &str,
) -> Result<usize> {
    db.delete_chunks_for_source(source_path)?;

    let chunks = chunk_markdown(source_path, content);
    if chunks.is_empty() {
        return Ok(0);
    }

    let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
    let embeddings = provider.embed(texts).await?;

    if embeddings.len() != chunks.len() {
        bail!(
            "embedding count mismatch for {source_path}: got {} embeddings for {} chunks",
            embeddings.len(),
            chunks.len()
        );
    }

    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        let rowid = db.upsert_chunk(
            &chunk.id,
            source_path,
            chunk.heading.as_deref(),
            &chunk.content,
            updated_at,
        )?;
        db.set_chunk_embedding(rowid, embedding)?;
    }

    Ok(chunks.len())
}

/// Re-index every markdown file in the vault (reads each via `vault.read` on
/// its relative path). Returns total chunks indexed.
pub async fn index_vault(db: &Db, provider: &dyn ChatProvider, vault: &Vault) -> Result<usize> {
    let paths = vault.list_markdown()?;
    let updated_at = chrono::Utc::now().to_rfc3339();

    let mut total = 0usize;
    for path in paths {
        let rel = path.to_string_lossy().replace('\\', "/");
        let content = vault.read(&rel)?;
        total += index_source(db, provider, &rel, &content, &updated_at).await?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ProviderHealth};
    use futures_util::stream::BoxStream;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use tempfile::tempdir;

    /// Deterministic, distinguishable 768-dim embeddings derived from a hash
    /// of the input text. Only used in tests.
    struct StubProvider;

    fn embed_one(text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; 768];
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();
        // Seed the first several dims from the hash so distinct texts get
        // distinguishable vectors; leave the rest zero.
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
    async fn index_source_stores_chunks_and_embeddings() {
        let db = Db::open_in_memory().unwrap();
        let provider = StubProvider;
        let content = "# Title\n\nIntro.\n\n## Section\n\nBody content.\n";
        let n = index_source(&db, &provider, "notes.md", content, "2026-01-01T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(n, 2);

        let hits = db
            .hybrid_search("Body content", &embed_one("Body content."), 5)
            .unwrap();
        assert!(!hits.is_empty());
    }

    #[tokio::test]
    async fn index_source_skips_embed_when_no_chunks() {
        let db = Db::open_in_memory().unwrap();
        let provider = StubProvider;
        let n = index_source(&db, &provider, "empty.md", "   \n\n  ", "2026-01-01T00:00:00Z")
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn index_source_replaces_old_chunks_on_reindex() {
        let db = Db::open_in_memory().unwrap();
        let provider = StubProvider;
        index_source(
            &db,
            &provider,
            "notes.md",
            "# Old\n\nOld content.\n",
            "2026-01-01T00:00:00Z",
        )
        .await
        .unwrap();

        let n = index_source(
            &db,
            &provider,
            "notes.md",
            "# New\n\nNew content.\n",
            "2026-01-02T00:00:00Z",
        )
        .await
        .unwrap();
        assert_eq!(n, 1);

        let hits = db
            .hybrid_search("New content", &embed_one("New content."), 5)
            .unwrap();
        assert!(hits.iter().any(|h| h.content.contains("New content")));
        assert!(!hits.iter().any(|h| h.content.contains("Old content")));
    }

    #[tokio::test]
    async fn index_vault_indexes_all_markdown_files() {
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        vault
            .write("Knowledge/a.md", "# A\n\nContent about apples.\n")
            .unwrap();
        vault
            .write("Knowledge/b.md", "# B\n\nContent about bananas.\n")
            .unwrap();

        let db = Db::open_in_memory().unwrap();
        let provider = StubProvider;
        let total = index_vault(&db, &provider, &vault).await.unwrap();
        // At least the two files we wrote plus the seeded template files.
        assert!(total >= 2);
    }
}
