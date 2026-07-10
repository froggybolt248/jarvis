//! ntfy phone-push commands. The private topic is generated once and persisted
//! in `settings` (the topic's secrecy is what provides privacy; it is also
//! shown to the user as a QR code to subscribe, so it is not a hard secret).

use tauri::State;

use crate::app_state::AppState;
use crate::core::notify::config;
use crate::core::notify::ntfy::{self, NtfyConfig};

/// Setting keys for the persisted ntfy configuration.
const NTFY_BASE_URL: &str = "ntfy_base_url";
const NTFY_TOPIC: &str = "ntfy_topic";
const DEFAULT_BASE_URL: &str = "https://ntfy.sh";

fn load_config(state: &AppState) -> Result<Option<NtfyConfig>, String> {
    config::load_config(&state.db).map_err(|e| e.to_string())
}

/// Return the persisted ntfy config, or `null` if not set up yet.
#[tauri::command]
pub fn ntfy_get_config(state: State<'_, AppState>) -> Result<Option<NtfyConfig>, String> {
    load_config(&state)
}

/// Ensure a private topic exists (generating one on first call) and return the
/// resulting config. An optional `base_url` overrides the default/self-hosted
/// server; the topic is preserved across calls.
#[tauri::command]
pub fn ntfy_setup(
    state: State<'_, AppState>,
    base_url: Option<String>,
) -> Result<NtfyConfig, String> {
    let existing = load_config(&state)?;
    let topic = existing
        .as_ref()
        .map(|c| c.topic.clone())
        .unwrap_or_else(ntfy::generate_topic);
    let base = base_url
        .filter(|b| !b.trim().is_empty())
        .or_else(|| existing.as_ref().map(|c| c.base_url.clone()))
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    state
        .db
        .set_setting(NTFY_TOPIC, &topic)
        .map_err(|e| e.to_string())?;
    state
        .db
        .set_setting(NTFY_BASE_URL, &base)
        .map_err(|e| e.to_string())?;
    Ok(NtfyConfig {
        base_url: base,
        topic,
    })
}

/// Publish the canned "connected" push so the user can confirm their phone is
/// subscribed to the topic.
#[tauri::command]
pub async fn ntfy_send_test(state: State<'_, AppState>) -> Result<(), String> {
    let cfg = load_config(&state)?.ok_or_else(|| "ntfy is not set up yet".to_string())?;
    let http = state.http.clone();
    ntfy::send_test(&http, &cfg).await.map_err(|e| e.to_string())
}
