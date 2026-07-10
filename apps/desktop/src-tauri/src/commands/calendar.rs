//! Read-only calendar commands: thin wrappers over `core::db::queries::calendar`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::calendar::CalendarEvent;

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
