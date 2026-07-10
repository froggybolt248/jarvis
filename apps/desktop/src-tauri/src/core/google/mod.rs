// Owned by WP-Google: OAuth (PKCE+loopback), token store, calendar client.
pub mod calendar_client;
pub mod oauth;
pub mod sync;
pub mod token_store;

/// Return a usable access token, refreshing (and re-persisting) it if it is
/// within 60s of expiry. Errors if Google isn't connected. Shared by both the
/// `commands::google` wrappers and the agent tool (`tools::calendar`), which
/// need a fresh token but must not depend on the `commands` layer.
pub async fn valid_access_token() -> anyhow::Result<String> {
    let stored = token_store::load()?.ok_or_else(|| anyhow::anyhow!("Google is not connected"))?;

    if stored.expires_at > chrono::Utc::now() + chrono::Duration::seconds(60) {
        return Ok(stored.access_token);
    }

    let refreshed =
        oauth::refresh_access_token(&stored.client_id, &stored.client_secret, &stored.refresh_token)
            .await?;
    let updated = token_store::StoredTokens {
        refresh_token: refreshed.refresh_token.unwrap_or(stored.refresh_token),
        access_token: refreshed.access_token,
        expires_at: refreshed.expires_at,
        client_id: stored.client_id,
        client_secret: stored.client_secret,
    };
    token_store::save(&updated)?;
    Ok(updated.access_token)
}
