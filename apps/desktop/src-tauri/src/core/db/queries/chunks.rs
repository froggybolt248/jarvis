use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

use super::search::serialize_f32;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chunk {
    pub rowid: i64,
    pub id: String,
    pub source_path: String,
    pub heading: Option<String>,
    pub content: String,
    pub updated_at: String,
}

impl Db {
    /// Insert or replace a chunk by id. Returns the sqlite rowid of the
    /// chunk, which is what `chunks_vec.chunk_rowid` should reference.
    pub fn upsert_chunk(
        &self,
        id: &str,
        source_path: &str,
        heading: Option<&str>,
        content: &str,
        updated_at: &str,
    ) -> Result<i64> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO chunks (id, source_path, heading, content, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(id) DO UPDATE SET \
                    source_path = excluded.source_path, \
                    heading = excluded.heading, \
                    content = excluded.content, \
                    updated_at = excluded.updated_at",
                params![id, source_path, heading, content, updated_at],
            )?;
            let rowid: i64 = conn.query_row(
                "SELECT rowid FROM chunks WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )?;
            Ok(rowid)
        })
    }

    /// Delete all chunks (and their FTS/vector rows via triggers/manual
    /// cleanup) belonging to a given source path.
    pub fn delete_chunks_for_source(&self, source_path: &str) -> Result<usize> {
        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM chunks_vec WHERE chunk_rowid IN (SELECT rowid FROM chunks WHERE source_path = ?1)",
                params![source_path],
            )?;
            let n = conn.execute(
                "DELETE FROM chunks WHERE source_path = ?1",
                params![source_path],
            )?;
            Ok(n)
        })
    }

    /// Set (or replace) the embedding for a given chunk rowid.
    pub fn set_chunk_embedding(&self, chunk_rowid: i64, embedding: &[f32]) -> Result<()> {
        let bytes = serialize_f32(embedding);
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO chunks_vec (chunk_rowid, embedding) VALUES (?1, ?2)",
                params![chunk_rowid, bytes],
            )?;
            Ok(())
        })
    }

    pub fn get_chunk_by_rowid(&self, rowid: i64) -> Result<Option<Chunk>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT rowid, id, source_path, heading, content, updated_at FROM chunks WHERE rowid = ?1",
                params![rowid],
                |r| {
                    Ok(Chunk {
                        rowid: r.get(0)?,
                        id: r.get(1)?,
                        source_path: r.get(2)?,
                        heading: r.get(3)?,
                        content: r.get(4)?,
                        updated_at: r.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
        })
    }
}
