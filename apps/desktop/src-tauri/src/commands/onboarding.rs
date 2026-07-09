//! Onboarding-flow commands: read/advance the wizard state, choose the vault
//! location, and finalize (which is the ONLY place Windows autostart is
//! enabled — the app must never register itself before a real first run).

use std::path::PathBuf;

use tauri::State;
use tauri_plugin_autostart::ManagerExt;

use crate::app_state::{default_vault_path, AppState};
use crate::core::setup::onboarding::{self, OnboardingState};

/// Current onboarding state (completion flag + chosen domains). The frontend
/// calls this on launch to decide between the wizard and the dashboard.
#[tauri::command]
pub fn get_onboarding_state(state: State<'_, AppState>) -> Result<OnboardingState, String> {
    onboarding::get_state(&state.db).map_err(|e| e.to_string())
}

/// Persist the domain selection without finishing onboarding.
#[tauri::command]
pub fn set_onboarding_domains(
    state: State<'_, AppState>,
    domains: Vec<String>,
) -> Result<(), String> {
    onboarding::set_domains(&state.db, &domains).map_err(|e| e.to_string())
}

/// Finalize onboarding: persist the final domain selection, mark it complete,
/// and register the app for autostart. A failure to enable autostart is logged
/// but does not fail onboarding (the app still works, just won't auto-launch).
#[tauri::command]
pub fn complete_onboarding(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    domains: Vec<String>,
) -> Result<(), String> {
    onboarding::complete(&state.db, &domains).map_err(|e| e.to_string())?;
    if let Err(e) = app.autolaunch().enable() {
        tracing::warn!("failed to enable autostart after onboarding: {e}");
    }
    Ok(())
}

/// Default vault location (`%USERPROFILE%/JarvisVault`), pre-filled in the UI.
#[tauri::command]
pub fn get_default_vault_dir() -> Result<String, String> {
    Ok(default_vault_path()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned())
}

/// Point the vault at `path`, opening (and seeding) it there and persisting the
/// choice. The frontend picks the directory via the dialog plugin and passes
/// the resulting path here.
#[tauri::command]
pub fn set_vault_dir(state: State<'_, AppState>, path: String) -> Result<(), String> {
    state
        .set_vault_path(&PathBuf::from(path))
        .map_err(|e| e.to_string())
}
