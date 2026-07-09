//! Thin `#[tauri::command]` wrappers exposing core services to the frontend.
//!
//! Commands are deliberately minimal: they translate arguments, call into a
//! core service on [`AppState`], and map errors to `String` (the wire error
//! type). No business logic lives here — that belongs in `core`.
//!
//! Feature work packages (onboarding, calendar, diet, ...) add their own
//! submodules here and register handlers in `lib.rs`.

use tauri::State;

use crate::app_state::AppState;
use crate::core::agent::provider::{ChatProvider, ProviderHealth};
use crate::core::db::queries::quiet_feed::QuietFeedItem;

/// Report Ollama availability and the locally installed models.
#[tauri::command]
pub async fn ollama_health(state: State<'_, AppState>) -> Result<ProviderHealth, String> {
    state.provider.health().await.map_err(|e| e.to_string())
}

/// Read a persisted setting value, or `null` if unset.
#[tauri::command]
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    state.db.get_setting(&key).map_err(|e| e.to_string())
}

/// Write (upsert) a persisted setting value.
#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    state.db.set_setting(&key, &value).map_err(|e| e.to_string())
}

/// Most recent Quiet Feed items, newest first.
#[tauri::command]
pub fn recent_feed(state: State<'_, AppState>, limit: usize) -> Result<Vec<QuietFeedItem>, String> {
    state.db.recent_feed(limit).map_err(|e| e.to_string())
}
