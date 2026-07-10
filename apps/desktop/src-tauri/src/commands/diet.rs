//! Diet commands: reads are thin wrappers over `core::db::queries::diet`;
//! the two write commands delegate to the same `queries::diet` helper
//! functions the `log_meal`/`set_diet_targets` agent tools use, so the
//! record-building logic isn't duplicated between the two paths. Unlike the
//! tools, these do NOT write a Quiet Feed row (that's reserved for auditing
//! autonomous agent mutations, not explicit user form input).

use tauri::State;

use crate::app_state::AppState;
use crate::core::db::queries::diet::{self, DietLog, DietTargets};

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

/// Logs a meal from the quick-add form.
#[tauri::command]
pub fn diet_log_meal(
    state: State<'_, AppState>,
    description: String,
    calories: Option<i64>,
    protein_g: Option<i64>,
    carbs_g: Option<i64>,
    fat_g: Option<i64>,
) -> Result<(), String> {
    diet::log_meal(&state.db, description, calories, protein_g, carbs_g, fat_g)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Sets today's diet targets from the quick-add form. At least one field
/// must be provided.
#[tauri::command]
pub fn diet_set_targets(
    state: State<'_, AppState>,
    calories: Option<i64>,
    protein_g: Option<i64>,
    carbs_g: Option<i64>,
    fat_g: Option<i64>,
) -> Result<(), String> {
    diet::set_diet_targets(&state.db, calories, protein_g, carbs_g, fat_g)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
