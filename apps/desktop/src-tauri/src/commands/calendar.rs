//! Calendar commands: read from the local SQLite cache, trigger a sync from
//! Google, and create new events (writing through to both Google and the
//! local cache).

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::calendar::CalendarEvent;
use crate::core::google;
use crate::core::google::calendar_client::{CalendarClient, CalendarEvent as ApiCalendarEvent, EventDateTime};
use crate::core::google::sync::{api_event_to_db, sync_calendar};

/// Calendar events starting in `[start, end)` (RFC3339 timestamps), ascending.
#[tauri::command]
pub fn calendar_events_between(
    state: State<'_, AppState>,
    start: String,
    end: String,
) -> Result<Vec<CalendarEvent>, String> {
    state
        .db
        .list_events_between(&start, &end)
        .map_err(|e| e.to_string())
}

/// Sync the primary calendar from Google into the local cache now. Returns
/// the number of events upserted.
#[tauri::command]
pub async fn calendar_sync_now(state: State<'_, AppState>) -> Result<usize, String> {
    sync_calendar(&state.db, "primary")
        .await
        .map_err(|e| e.to_string())
}

/// Create a new (timed) event on the primary Google calendar, then upsert it
/// into the local cache so the UI reflects it immediately.
#[tauri::command]
pub async fn calendar_create_event(
    state: State<'_, AppState>,
    summary: String,
    start_rfc3339: String,
    end_rfc3339: String,
    description: Option<String>,
    location: Option<String>,
) -> Result<(), String> {
    let token = google::valid_access_token().await.map_err(|e| e.to_string())?;

    let event = ApiCalendarEvent {
        id: None,
        summary: Some(summary),
        description,
        location,
        start: EventDateTime {
            date_time: Some(start_rfc3339),
            date: None,
            time_zone: None,
        },
        end: EventDateTime {
            date_time: Some(end_rfc3339),
            date: None,
            time_zone: None,
        },
        status: None,
    };

    let created = CalendarClient::new(token)
        .insert_event("primary", &event)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(db_event) = api_event_to_db(&created, "primary") {
        state.db.upsert_event(&db_event).map_err(|e| e.to_string())?;
    }

    Ok(())
}
