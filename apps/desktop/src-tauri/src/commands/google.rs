//! Google Calendar connection commands. OAuth tokens (and the user's own
//! client id/secret) are persisted in Windows Credential Manager via
//! [`token_store`]; the access token is transparently refreshed when stale.

use tauri::State;

use crate::app_state::AppState;
use crate::core::google::calendar_client::{CalendarClient, CalendarListEntry};
use crate::core::google::sync::sync_calendar;
use crate::core::google::{oauth, token_store};

/// Run the loopback OAuth flow (opens the system browser), then persist the
/// resulting tokens alongside the user's client credentials. On success,
/// kicks off a best-effort calendar sync so events show up immediately
/// without waiting for the next scheduled/background sync; a sync failure
/// here must not fail the connect itself.
#[tauri::command]
pub async fn google_connect(
    state: State<'_, AppState>,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let tokens = oauth::run_loopback_flow(&client_id, &client_secret)
        .await
        .map_err(|e| e.to_string())?;
    let refresh_token = tokens.refresh_token.ok_or_else(|| {
        "Google did not return a refresh token. Remove Jarvis under your Google \
         Account → Security → Third-party access, then reconnect."
            .to_string()
    })?;
    token_store::save(&token_store::StoredTokens {
        refresh_token,
        access_token: tokens.access_token,
        expires_at: tokens.expires_at,
        client_id,
        client_secret,
    })
    .map_err(|e| e.to_string())?;

    if let Err(err) = sync_calendar(&state.db, "primary").await {
        eprintln!("post-connect calendar sync failed: {err:#}");
    }

    Ok(())
}

/// Whether Google is currently connected (tokens present).
#[tauri::command]
pub fn google_status() -> Result<bool, String> {
    Ok(token_store::load().map_err(|e| e.to_string())?.is_some())
}

/// Forget stored Google tokens/credentials.
#[tauri::command]
pub fn google_disconnect() -> Result<(), String> {
    token_store::clear().map_err(|e| e.to_string())
}

/// Smoke-test the connection: list the user's calendars.
#[tauri::command]
pub async fn google_list_calendars() -> Result<Vec<CalendarListEntry>, String> {
    let token = crate::core::google::valid_access_token()
        .await
        .map_err(|e| e.to_string())?;
    CalendarClient::new(token)
        .list_calendars()
        .await
        .map_err(|e| e.to_string())
}
