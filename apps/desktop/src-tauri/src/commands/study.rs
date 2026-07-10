//! Study commands: reads are thin wrappers over `core::db::queries::study`;
//! the two write commands delegate to the same `queries::study` helper
//! functions (and the pure `apply_sm2` scheduler) the `create_study_card`/
//! `review_study_card` agent tools use, so the scheduling logic isn't
//! duplicated between the two paths. Unlike the tools, these do NOT write a
//! Quiet Feed row (that's reserved for auditing autonomous agent mutations,
//! not explicit user form input).

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::study::{self, SrsCard};

/// SRS cards due at or before `now` (an RFC3339 timestamp), soonest first.
#[tauri::command]
pub fn study_due_cards(state: State<'_, AppState>, now: String) -> Result<Vec<SrsCard>, String> {
    state.db.due_cards(&now).map_err(|e| e.to_string())
}

/// Creates a new study card from the quick-add form. Returns the new card's id.
#[tauri::command]
pub fn study_create_card(
    state: State<'_, AppState>,
    front: String,
    back: String,
    course_id: Option<String>,
) -> Result<String, String> {
    study::create_study_card(&state.db, front, back, course_id)
        .map(|card| card.id)
        .map_err(|e| e.to_string())
}

/// Records a review of `id` with the given `quality` (0-5), rescheduling it
/// via SM-2.
#[tauri::command]
pub fn study_review_card(state: State<'_, AppState>, id: String, quality: u8) -> Result<(), String> {
    study::review_study_card(&state.db, &id, quality)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
