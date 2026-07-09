use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

/// Reciprocal Rank Fusion constant, as recommended by the original RRF paper.
const RRF_K: f64 = 60.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchHit {
    pub chunk_id: String,
    pub source_path: String,
    pub heading: Option<String>,
    pub content: String,
    pub score: f64,
}

/// Serialize an f32 embedding into the raw little-endian byte layout that
/// sqlite-vec's vec0 virtual table expects for a `FLOAT[N]` column.
pub fn serialize_f32(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for f in v {
        bytes.extend_from_slice(&f.to_le_bytes());
    }
    bytes
}

impl Db {
    /// Hybrid search over `chunks`: fuses FTS5 BM25 full-text results with
    /// vec0 KNN vector results using Reciprocal Rank Fusion (k=60).
    pub fn hybrid_search(
        &self,
        query_text: &str,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<SearchHit>> {
        self.with_conn(|conn| {
            // Full-text ranked list (by rowid, best match first).
            let mut fts_stmt = conn.prepare(
                "SELECT rowid FROM chunks_fts WHERE chunks_fts MATCH ?1 ORDER BY bm25(chunks_fts) LIMIT ?2",
            )?;
            let fts_rowids: Vec<i64> = fts_stmt
                .query_map(params![query_text, k as i64], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()
                .unwrap_or_default();

            // Vector KNN ranked list (by chunk_rowid, best match first).
            let query_bytes = serialize_f32(query_embedding);
            let mut vec_stmt = conn.prepare(
                "SELECT chunk_rowid FROM chunks_vec WHERE embedding MATCH ?1 AND k = ?2 ORDER BY distance",
            )?;
            let vec_rowids: Vec<i64> = vec_stmt
                .query_map(params![query_bytes, k as i64], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()
                .unwrap_or_default();

            // Reciprocal Rank Fusion.
            let mut scores: HashMap<i64, f64> = HashMap::new();
            for (rank, rowid) in fts_rowids.into_iter().enumerate() {
                *scores.entry(rowid).or_insert(0.0) += 1.0 / (RRF_K + (rank + 1) as f64);
            }
            for (rank, rowid) in vec_rowids.into_iter().enumerate() {
                *scores.entry(rowid).or_insert(0.0) += 1.0 / (RRF_K + (rank + 1) as f64);
            }

            let mut ranked: Vec<(i64, f64)> = scores.into_iter().collect();
            ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            ranked.truncate(k);

            let mut hits = Vec::with_capacity(ranked.len());
            for (rowid, score) in ranked {
                let hit = conn
                    .query_row(
                        "SELECT id, source_path, heading, content FROM chunks WHERE rowid = ?1",
                        params![rowid],
                        |r| {
                            Ok(SearchHit {
                                chunk_id: r.get(0)?,
                                source_path: r.get(1)?,
                                heading: r.get(2)?,
                                content: r.get(3)?,
                                score,
                            })
                        },
                    )
                    .ok();
                if let Some(hit) = hit {
                    hits.push(hit);
                }
            }

            Ok(hits)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedding_near(base: f32, len: usize) -> Vec<f32> {
        (0..len).map(|i| base + i as f32 * 0.0001).collect()
    }

    #[test]
    fn hybrid_search_ranks_dual_matches_above_single_matches() {
        let db = Db::open_in_memory().unwrap();

        let vec_a = embedding_near(1.0, 768); // "both" chunk's embedding
        let vec_b = embedding_near(50.0, 768); // far away, distinct embedding
        let vec_c = embedding_near(1.0001, 768); // very close to vec_a

        let rowid_both = db
            .upsert_chunk("both", "notes.md", None, "the quick brown fox jumps", "2026-01-01T00:00:00Z")
            .unwrap();
        let rowid_text_only = db
            .upsert_chunk("text_only", "notes.md", None, "quick brown fox reference", "2026-01-01T00:00:00Z")
            .unwrap();
        let rowid_vec_only = db
            .upsert_chunk("vec_only", "notes.md", None, "totally unrelated content", "2026-01-01T00:00:00Z")
            .unwrap();

        db.set_chunk_embedding(rowid_both, &vec_a).unwrap();
        db.set_chunk_embedding(rowid_text_only, &vec_b).unwrap();
        db.set_chunk_embedding(rowid_vec_only, &vec_c).unwrap();

        // Query text matches "both" and "text_only" (both contain "quick brown fox").
        // Query embedding is close to vec_a ("both") and vec_c ("vec_only").
        let query_embedding = embedding_near(1.00005, 768);
        let hits = db.hybrid_search("quick brown fox", &query_embedding, 10).unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].chunk_id, "both", "chunk matching both text and vector should rank first");

        let ids: Vec<&str> = hits.iter().map(|h| h.chunk_id.as_str()).collect();
        assert!(ids.contains(&"text_only"));
        assert!(ids.contains(&"vec_only"));
    }
}
