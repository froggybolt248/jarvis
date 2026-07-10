//! Read-only diet commands: thin wrappers over `core::db::queries::diet`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::diet::{DietLog, DietTargets};

/// Diet logs for `date` (`YYYY-MM-DD`), oldest first.
#[tauri::command]
pub fn diet_logs_for_date(state: State<'_, AppState>, date: String) -> Result<Vec<DietLog>, String> {
    state.db.logs_for_date(&date).map_err(|e| e.to_string())
}

/// The most recently-effective diet targets, or `null` if none are set.
#[tauri::command]
pub fn diet_current_targets(state: State<'_, AppState>) -> Result<Option<DietTargets>, String> {
    state.db.current_targets().map_err(|e| e.to_string())
}
