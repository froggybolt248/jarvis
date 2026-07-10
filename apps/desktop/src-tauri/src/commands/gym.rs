//! Read-only gym commands: thin wrappers over `core::db::queries::gym`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::gym::{GymSession, GymSet};

/// Most recent gym sessions, newest first.
#[tauri::command]
pub fn gym_recent_sessions(state: State<'_, AppState>, limit: usize) -> Result<Vec<GymSession>, String> {
    state.db.recent_sessions(limit).map_err(|e| e.to_string())
}

/// Most recent sets logged for `exercise`, newest session first.
#[tauri::command]
pub fn gym_sets_for_exercise(
    state: State<'_, AppState>,
    exercise: String,
    limit: usize,
) -> Result<Vec<GymSet>, String> {
    state
        .db
        .sets_for_exercise(&exercise, limit)
        .map_err(|e| e.to_string())
}
