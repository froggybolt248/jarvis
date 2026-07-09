//! Ollama setup commands used by the onboarding wizard: detect an existing
//! install, recommend models for the machine's RAM, install via winget, start
//! the server, and pull a model while streaming progress to the frontend.
//!
//! The blocking helpers in [`core::setup::ollama`] (winget spawn, server
//! polling, RAM probe via PowerShell) are dispatched on the blocking pool so
//! they never stall an async worker thread.

use serde::Serialize;
use tauri::{Emitter, State};

use crate::app_state::AppState;
use crate::core::setup::ollama;

/// Detect whether Ollama is installed, whether its server is up, and which
/// models are already pulled — all against the provider's configured URL.
#[tauri::command]
pub async fn ollama_detect(state: State<'_, AppState>) -> Result<ollama::OllamaStatus, String> {
    Ok(ollama::detect(state.provider.base_url()).await)
}

/// Machine RAM (GiB) plus the model set recommended for it (8B on ≥24 GB,
/// otherwise 4B), so the wizard can preselect the right download.
#[derive(Debug, Serialize)]
pub struct RecommendedModels {
    pub ram_gb: u32,
    pub models: Vec<String>,
}

#[tauri::command]
pub async fn ollama_recommended_models() -> Result<RecommendedModels, String> {
    let ram_gb = tokio::task::spawn_blocking(ollama::total_ram_gb)
        .await
        .map_err(|e| e.to_string())?;
    let models = ollama::recommended_models(ram_gb);
    Ok(RecommendedModels { ram_gb, models })
}

/// Install Ollama via `winget`. Long-running; the frontend shows a spinner.
#[tauri::command]
pub async fn ollama_install() -> Result<(), String> {
    tokio::task::spawn_blocking(ollama::install_via_winget)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Ensure `ollama serve` is running (spawns it detached if not), polling until
/// the server answers.
#[tauri::command]
pub async fn ollama_ensure_running(state: State<'_, AppState>) -> Result<(), String> {
    let base = state.provider.base_url().to_string();
    tokio::task::spawn_blocking(move || ollama::ensure_server_running(&base))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Pull a model, emitting an `ollama:pull-progress` event per progress update
/// (payload: [`ollama::PullProgress`]) so the wizard can render a live bar.
#[tauri::command]
pub async fn ollama_pull(
    window: tauri::Window,
    state: State<'_, AppState>,
    model: String,
) -> Result<(), String> {
    let base = state.provider.base_url().to_string();
    ollama::pull_model(&base, &model, |progress| {
        let _ = window.emit("ollama:pull-progress", &progress);
    })
    .await
    .map_err(|e| e.to_string())
}
