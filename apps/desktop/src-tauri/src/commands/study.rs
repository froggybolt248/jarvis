//! Read-only study commands: thin wrappers over `core::db::queries::study`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::study::SrsCard;

/// SRS cards due at or before `now` (an RFC3339 timestamp), soonest first.
#[tauri::command]
pub fn study_due_cards(state: State<'_, AppState>, now: String) -> Result<Vec<SrsCard>, String> {
    state.db.due_cards(&now).map_err(|e| e.to_string())
}
