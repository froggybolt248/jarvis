//! Gym commands: reads are thin wrappers over `core::db::queries::gym`; the
//! write command delegates to the same `queries::gym::log_workout` helper
//! the `log_workout` agent tool uses, so the session/set construction logic
//! isn't duplicated between the two paths. Unlike the tool, this does NOT
//! write a Quiet Feed row (that's reserved for auditing autonomous agent
//! mutations, not explicit user form input).

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::gym::{self, GymSession, GymSet, SetInput};

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

/// Logs a completed workout (one session + its sets) from the quick-add
/// form. Returns the new session's id.
#[tauri::command]
pub fn gym_log_workout(
    state: State<'_, AppState>,
    notes: Option<String>,
    sets: Vec<SetInput>,
) -> Result<String, String> {
    gym::log_workout(&state.db, notes, sets)
        .map(|session| session.id)
        .map_err(|e| e.to_string())
}
