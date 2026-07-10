//! The chat command: bridges the frontend to the M3 agent loop
//! ([`crate::core::agent::agent_loop`]), streaming every [`AgentEvent`] to the
//! webview as it happens rather than waiting for the whole turn to finish.

use tauri::{Emitter, State};

use crate::app_state::AppState;
use crate::core::agent::agent_loop::{self, AgentContext};
use crate::core::agent::provider::{pick_default_model, ChatProvider};
use crate::core::memory::Vault;

/// Settings key for the user-chosen chat model (falls back to an
/// auto-detected default when unset).
const CHAT_MODEL_SETTING: &str = "chat_model";

/// Run one agent turn for `message`, emitting each [`AgentEvent`] on the
/// `"agent:event"` window event as it's produced. The final answer/citations
/// are also available via the returned outcome, but the frontend is expected
/// to reconstruct the transcript from the streamed events.
#[tauri::command]
pub async fn chat(
    window: tauri::Window,
    state: State<'_, AppState>,
    message: String,
) -> Result<(), String> {
    let model = resolve_model(&state).await;
    let date = chrono::Local::now().format("%Y-%m-%d (%A)").to_string();

    // `AgentContext` borrows `&Vault` for the whole (awaited) turn, but a std
    // `RwLockReadGuard` is `!Send` and can't live across an await point in a
    // future the async runtime must be able to move between threads. Instead
    // of holding the guard, copy out the vault root (guard dropped
    // immediately after) and open a fresh, owned `Vault` for this turn —
    // `Vault::open` is idempotent and never overwrites existing files, so
    // this is just as cheap and correct as borrowing the shared instance.
    let vault_root = state.vault.read().expect("vault lock poisoned").root().to_path_buf();
    let vault = Vault::open(&vault_root).map_err(|e| e.to_string())?;

    let ctx = AgentContext {
        db: &state.db,
        vault: &vault,
        provider: &state.provider,
        registry: &state.registry,
        model,
        date,
        quiet_hours: crate::core::notify::dispatch::is_quiet_now(&state.db).unwrap_or(false),
    };

    agent_loop::run_turn(&ctx, &message, |event| {
        let _ = window.emit("agent:event", &event);
    })
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Pick the model for a turn: the persisted `chat_model` setting if set,
/// otherwise the best model the provider currently reports as installed
/// (falling back to `"qwen3:4b"` if the provider is unreachable).
async fn resolve_model(state: &AppState) -> String {
    if let Ok(Some(model)) = state.db.get_setting(CHAT_MODEL_SETTING) {
        if !model.trim().is_empty() {
            return model;
        }
    }
    match state.provider.health().await {
        Ok(health) => pick_default_model(&health.models, 32),
        Err(_) => "qwen3:4b".to_string(),
    }
}
