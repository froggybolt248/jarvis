//! Vault note listing for the Knowledge screen.

use tauri::State;

use crate::app_state::AppState;
use crate::core::memory::NoteSummary;

/// Summaries of every note in the vault, newest-modified first.
#[tauri::command]
pub fn vault_list_notes(state: State<'_, AppState>) -> Result<Vec<NoteSummary>, String> {
    let vault = state.vault.read().expect("vault lock poisoned");
    vault.list_notes().map_err(|e| e.to_string())
}
