use anyhow::Result;
use chrono::Utc;
use rusqlite::params;
use uuid::Uuid;

use crate::core::db::Db;
use crate::core::notify::ntfy::NtfyMessage;

fn row_to_message(r: &rusqlite::Row) -> rusqlite::Result<NtfyMessage> {
    let priority: Option<i64> = r.get(1)?;
    let tags: Option<String> = r.get(2)?;
    Ok(NtfyMessage {
        title: r.get(0)?,
        body: r.get(3)?,
        priority: priority.map(|p| p as u8),
        tags: tags
            .filter(|t| !t.is_empty())
            .map(|t| t.split(',').map(|s| s.to_string()).collect())
            .unwrap_or_default(),
        click: r.get(4)?,
    })
}

impl Db {
    /// Persist a message to the batched-notification queue, to be flushed
    /// once quiet hours end.
    pub fn enqueue(&self, msg: &NtfyMessage) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let tags = msg.tags.join(",");
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO notification_queue (id, created_at, title, body, priority, tags, click) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    id,
                    created_at,
                    msg.title,
                    msg.body,
                    msg.priority.map(|p| p as i64),
                    tags,
                    msg.click,
                ],
            )?;
            Ok(())
        })
    }

    /// Fetch every queued message (ordered by enqueue time) and remove them
    /// from the queue, atomically.
    pub fn drain_queued(&self) -> Result<Vec<NtfyMessage>> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            let messages: Vec<NtfyMessage> = {
                let mut stmt = tx.prepare(
                    "SELECT title, priority, tags, body, click FROM notification_queue ORDER BY created_at ASC",
                )?;
                let rows = stmt.query_map([], row_to_message)?;
                let mut out = Vec::new();
                for row in rows {
                    out.push(row?);
                }
                out
            };
            tx.execute("DELETE FROM notification_queue", [])?;
            tx.commit()?;
            Ok(messages)
        })
    }

    /// Number of messages currently sitting in the batched-notification queue.
    pub fn queued_count(&self) -> Result<usize> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row("SELECT count(*) FROM notification_queue", [], |r| r.get(0))?;
            Ok(count as usize)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_drain_round_trip() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.queued_count().unwrap(), 0);

        let msg1 = NtfyMessage {
            title: "Hello".to_string(),
            body: "World".to_string(),
            priority: Some(3),
            tags: vec!["warning".to_string(), "tada".to_string()],
            click: Some("https://example.com".to_string()),
        };
        let msg2 = NtfyMessage {
            title: "Second".to_string(),
            body: "Message".to_string(),
            priority: None,
            tags: vec![],
            click: None,
        };

        db.enqueue(&msg1).unwrap();
        db.enqueue(&msg2).unwrap();
        assert_eq!(db.queued_count().unwrap(), 2);

        let drained = db.drain_queued().unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].title, "Hello");
        assert_eq!(drained[0].tags, vec!["warning".to_string(), "tada".to_string()]);
        assert_eq!(drained[0].click, Some("https://example.com".to_string()));
        assert_eq!(drained[1].title, "Second");
        assert_eq!(drained[1].priority, None);
        assert_eq!(drained[1].tags, Vec::<String>::new());

        // Draining empties the queue.
        assert_eq!(db.queued_count().unwrap(), 0);
        let empty = db.drain_queued().unwrap();
        assert!(empty.is_empty());
    }
}
