//! Google Calendar connection commands. OAuth tokens (and the user's own
//! client id/secret) are persisted in Windows Credential Manager via
//! [`token_store`]; the access token is transparently refreshed when stale.

use crate::core::google::calendar_client::{CalendarClient, CalendarListEntry};
use crate::core::google::{oauth, token_store};

/// Run the loopback OAuth flow (opens the system browser), then persist the
/// resulting tokens alongside the user's client credentials.
#[tauri::command]
pub async fn google_connect(client_id: String, client_secret: String) -> Result<(), String> {
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
    .map_err(|e| e.to_string())
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
    let token = valid_access_token().await?;
    CalendarClient::new(token)
        .list_calendars()
        .await
        .map_err(|e| e.to_string())
}

/// Return a usable access token, refreshing (and re-persisting) it if it is
/// within 60s of expiry. Errors if Google isn't connected.
async fn valid_access_token() -> Result<String, String> {
    let stored = token_store::load()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Google is not connected".to_string())?;

    if stored.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
        return Ok(stored.access_token);
    }

    let refreshed =
        oauth::refresh_access_token(&stored.client_id, &stored.client_secret, &stored.refresh_token)
            .await
            .map_err(|e| e.to_string())?;
    let updated = token_store::StoredTokens {
        refresh_token: refreshed.refresh_token.unwrap_or(stored.refresh_token),
        access_token: refreshed.access_token,
        expires_at: refreshed.expires_at,
        client_id: stored.client_id,
        client_secret: stored.client_secret,
    };
    token_store::save(&updated).map_err(|e| e.to_string())?;
    Ok(updated.access_token)
}
