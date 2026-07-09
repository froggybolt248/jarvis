use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarEvent {
    pub id: String,
    pub google_id: Option<String>,
    pub calendar_id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start_at: Option<String>,
    pub end_at: Option<String>,
    pub all_day: bool,
    pub status: Option<String>,
    pub updated_at: String,
}

fn row_to_event(r: &rusqlite::Row) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: r.get(0)?,
        google_id: r.get(1)?,
        calendar_id: r.get(2)?,
        summary: r.get(3)?,
        description: r.get(4)?,
        location: r.get(5)?,
        start_at: r.get(6)?,
        end_at: r.get(7)?,
        all_day: r.get::<_, i64>(8)? != 0,
        status: r.get(9)?,
        updated_at: r.get(10)?,
    })
}

const SELECT_COLUMNS: &str = "id, google_id, calendar_id, summary, description, location, start_at, end_at, all_day, status, updated_at";

impl Db {
    #[allow(clippy::too_many_arguments)]
    pub fn upsert_event(&self, event: &CalendarEvent) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO calendar_events (id, google_id, calendar_id, summary, description, location, start_at, end_at, all_day, status, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                 ON CONFLICT(id) DO UPDATE SET \
                    google_id = excluded.google_id, \
                    calendar_id = excluded.calendar_id, \
                    summary = excluded.summary, \
                    description = excluded.description, \
                    location = excluded.location, \
                    start_at = excluded.start_at, \
                    end_at = excluded.end_at, \
                    all_day = excluded.all_day, \
                    status = excluded.status, \
                    updated_at = excluded.updated_at",
                params![
                    event.id,
                    event.google_id,
                    event.calendar_id,
                    event.summary,
                    event.description,
                    event.location,
                    event.start_at,
                    event.end_at,
                    event.all_day as i64,
                    event.status,
                    event.updated_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_events_between(&self, start: &str, end: &str) -> Result<Vec<CalendarEvent>> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {SELECT_COLUMNS} FROM calendar_events WHERE start_at >= ?1 AND start_at < ?2 ORDER BY start_at ASC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt
                .query_map(params![start, end], row_to_event)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
    }

    pub fn get_sync_token(&self, calendar_id: &str) -> Result<Option<String>> {
        self.with_conn(|conn| {
            let token = conn
                .query_row(
                    "SELECT sync_token FROM calendar_sync_state WHERE calendar_id = ?1",
                    params![calendar_id],
                    |r| r.get(0),
                )
                .optional()?;
            Ok(token.flatten())
        })
    }

    pub fn set_sync_token(
        &self,
        calendar_id: &str,
        sync_token: &str,
        last_synced_at: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO calendar_sync_state (calendar_id, sync_token, last_synced_at) \
                 VALUES (?1, ?2, ?3) \
                 ON CONFLICT(calendar_id) DO UPDATE SET \
                    sync_token = excluded.sync_token, \
                    last_synced_at = excluded.last_synced_at",
                params![calendar_id, sync_token, last_synced_at],
            )?;
            Ok(())
        })
    }
}
