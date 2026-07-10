//! Calendar sync: pulls a rolling window of events from Google Calendar and
//! upserts them into the local SQLite cache.
//!
//! This is a full-refresh sync over `[now - 7 days, now + 35 days]`,
//! deliberately NOT using Google's `syncToken` incremental-sync mechanism:
//! Google rejects `syncToken` when combined with the `orderBy` parameter that
//! `CalendarClient::list_events` always sends, so true incremental sync isn't
//! available with the current client. A rolling-window refresh is simpler and
//! sufficient for the MVP; the sync-token DB methods
//! (`get_sync_token`/`set_sync_token`) are left untouched for potential
//! future use but are not called here.

use anyhow::Result;
use chrono::{Duration, Utc};

use crate::core::db::queries::calendar::CalendarEvent as DbCalendarEvent;
use crate::core::db::Db;

use super::calendar_client::{self, CalendarClient};

const WINDOW_PAST_DAYS: i64 = 7;
const WINDOW_FUTURE_DAYS: i64 = 35;

/// Pure mapping from a Google Calendar API event to the local DB row shape.
/// Returns `None` for events with no `id` (nothing stable to key the upsert
/// on, so they're skipped).
pub(crate) fn api_event_to_db(
    api: &calendar_client::CalendarEvent,
    calendar_id: &str,
) -> Option<DbCalendarEvent> {
    let id = api.id.clone()?;
    let all_day = api.start.date.is_some();
    let start_at = api.start.date_time.clone().or_else(|| api.start.date.clone());
    let end_at = api.end.date_time.clone().or_else(|| api.end.date.clone());

    Some(DbCalendarEvent {
        id: id.clone(),
        google_id: Some(id),
        calendar_id: Some(calendar_id.to_string()),
        summary: api.summary.clone(),
        description: api.description.clone(),
        location: api.location.clone(),
        start_at,
        end_at,
        all_day,
        status: api.status.clone(),
        updated_at: Utc::now().to_rfc3339(),
    })
}

/// Syncs `calendar_id` for a rolling window around "now", upserting every
/// event returned by Google into the local cache. Returns the number of
/// events upserted.
pub async fn sync_calendar(db: &Db, calendar_id: &str) -> Result<usize> {
    let token = super::valid_access_token().await?;

    let now = Utc::now();
    let time_min = (now - Duration::days(WINDOW_PAST_DAYS)).to_rfc3339();
    let time_max = (now + Duration::days(WINDOW_FUTURE_DAYS)).to_rfc3339();

    let client = CalendarClient::new(token);
    let events = client.list_events(calendar_id, &time_min, &time_max).await?;

    let mut count = 0;
    for api_event in &events {
        if let Some(db_event) = api_event_to_db(api_event, calendar_id) {
            db.upsert_event(&db_event)?;
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::google::calendar_client::EventDateTime;

    fn timed_event(id: &str) -> calendar_client::CalendarEvent {
        calendar_client::CalendarEvent {
            id: Some(id.to_string()),
            summary: Some("Standup".to_string()),
            description: Some("Daily sync".to_string()),
            location: Some("Room 1".to_string()),
            start: EventDateTime {
                date_time: Some("2026-01-01T09:00:00-08:00".to_string()),
                time_zone: Some("America/Los_Angeles".to_string()),
                date: None,
            },
            end: EventDateTime {
                date_time: Some("2026-01-01T09:30:00-08:00".to_string()),
                time_zone: Some("America/Los_Angeles".to_string()),
                date: None,
            },
            status: Some("confirmed".to_string()),
        }
    }

    fn all_day_event(id: &str) -> calendar_client::CalendarEvent {
        calendar_client::CalendarEvent {
            id: Some(id.to_string()),
            summary: Some("Offsite".to_string()),
            description: None,
            location: None,
            start: EventDateTime {
                date: Some("2026-01-02".to_string()),
                ..Default::default()
            },
            end: EventDateTime {
                date: Some("2026-01-03".to_string()),
                ..Default::default()
            },
            status: None,
        }
    }

    #[test]
    fn maps_timed_event() {
        let api = timed_event("evt1");
        let db = api_event_to_db(&api, "primary").expect("should map");

        assert_eq!(db.id, "evt1");
        assert_eq!(db.google_id.as_deref(), Some("evt1"));
        assert_eq!(db.calendar_id.as_deref(), Some("primary"));
        assert_eq!(db.summary.as_deref(), Some("Standup"));
        assert_eq!(db.description.as_deref(), Some("Daily sync"));
        assert_eq!(db.location.as_deref(), Some("Room 1"));
        assert_eq!(db.start_at.as_deref(), Some("2026-01-01T09:00:00-08:00"));
        assert_eq!(db.end_at.as_deref(), Some("2026-01-01T09:30:00-08:00"));
        assert!(!db.all_day);
        assert_eq!(db.status.as_deref(), Some("confirmed"));
        assert!(!db.updated_at.is_empty());
    }

    #[test]
    fn maps_all_day_event() {
        let api = all_day_event("evt2");
        let db = api_event_to_db(&api, "primary").expect("should map");

        assert_eq!(db.id, "evt2");
        assert_eq!(db.google_id.as_deref(), Some("evt2"));
        assert_eq!(db.start_at.as_deref(), Some("2026-01-02"));
        assert_eq!(db.end_at.as_deref(), Some("2026-01-03"));
        assert!(db.all_day);
        assert_eq!(db.status, None);
        assert_eq!(db.description, None);
        assert_eq!(db.location, None);
    }

    #[test]
    fn missing_id_is_skipped() {
        let mut api = timed_event("ignored");
        api.id = None;
        assert!(api_event_to_db(&api, "primary").is_none());
    }

    #[test]
    fn uses_secondary_calendar_id() {
        let api = timed_event("evt3");
        let db = api_event_to_db(&api, "work@group.calendar.google.com").expect("should map");
        assert_eq!(
            db.calendar_id.as_deref(),
            Some("work@group.calendar.google.com")
        );
    }
}
