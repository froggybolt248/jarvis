use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoreMemoryEntry {
    pub id: String,
    pub label: String,
    pub content: String,
    pub pinned: bool,
    pub updated_at: String,
}

impl Db {
    pub fn list_core_memory(&self) -> Result<Vec<CoreMemoryEntry>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, label, content, pinned, updated_at FROM core_memory ORDER BY pinned DESC, updated_at DESC",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(CoreMemoryEntry {
                        id: r.get(0)?,
                        label: r.get(1)?,
                        content: r.get(2)?,
                        pinned: r.get::<_, i64>(3)? != 0,
                        updated_at: r.get(4)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    pub fn upsert_core_memory(
        &self,
        id: &str,
        label: &str,
        content: &str,
        pinned: bool,
        updated_at: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO core_memory (id, label, content, pinned, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5) \
                 ON CONFLICT(id) DO UPDATE SET \
                    label = excluded.label, \
                    content = excluded.content, \
                    pinned = excluded.pinned, \
                    updated_at = excluded.updated_at",
                params![id, label, content, pinned as i64, updated_at],
            )?;
            Ok(())
        })
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE core_memory SET pinned = ?2 WHERE id = ?1",
                params![id, pinned as i64],
            )?;
            Ok(())
        })
    }
}
