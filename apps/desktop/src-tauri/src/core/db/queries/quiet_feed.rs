use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuietFeedItem {
    pub id: String,
    pub created_at: String,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub deep_link: Option<String>,
    pub source: Option<String>,
}

fn row_to_item(r: &rusqlite::Row) -> rusqlite::Result<QuietFeedItem> {
    Ok(QuietFeedItem {
        id: r.get(0)?,
        created_at: r.get(1)?,
        kind: r.get(2)?,
        title: r.get(3)?,
        body: r.get(4)?,
        deep_link: r.get(5)?,
        source: r.get(6)?,
    })
}

impl Db {
    pub fn insert_feed(&self, item: &QuietFeedItem) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO quiet_feed (id, created_at, kind, title, body, deep_link, source) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item.id,
                    item.created_at,
                    item.kind,
                    item.title,
                    item.body,
                    item.deep_link,
                    item.source,
                ],
            )?;
            Ok(())
        })
    }

    pub fn recent_feed(&self, limit: usize) -> Result<Vec<QuietFeedItem>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, created_at, kind, title, body, deep_link, source FROM quiet_feed ORDER BY created_at DESC LIMIT ?1",
            )?;
            let rows = stmt
                .query_map(params![limit as i64], row_to_item)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn quiet_feed_round_trip() {
        let db = Db::open_in_memory().unwrap();
        let item = QuietFeedItem {
            id: Uuid::new_v4().to_string(),
            created_at: "2026-07-09T08:00:00Z".into(),
            kind: "digest".into(),
            title: "Morning summary".into(),
            body: Some("Nothing urgent today.".into()),
            deep_link: Some("jarvis://feed/1".into()),
            source: Some("scheduler".into()),
        };
        db.insert_feed(&item).unwrap();

        let recent = db.recent_feed(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0], item);

        let none = db.recent_feed(0).unwrap();
        assert!(none.is_empty());
    }
}
