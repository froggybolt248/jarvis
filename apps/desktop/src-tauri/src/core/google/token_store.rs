// WP-Google owns this file.
//
// Persists Google OAuth tokens in the OS-native credential store via the
// `keyring` crate (v4.1.4, `v1` feature — the default). On Windows this
// backs onto the Windows Credential Manager.
//
// `keyring::Entry` (v1 API) exposes `new`, `set_secret`/`get_secret`
// (raw bytes) and `set_password`/`get_password` (UTF-8 strings), plus
// `delete_credential`. We use the raw-bytes secret API since we store a
// JSON blob and want to control encoding explicitly.
//
// We deliberately store `client_id`/`client_secret` alongside the tokens
// (see `StoredTokens`) so that a silent, headless refresh
// (`oauth::refresh_access_token`) can run later without re-prompting the
// user for their Google Cloud OAuth client credentials.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use keyring::Entry;
use serde::{Deserialize, Serialize};

const SERVICE: &str = "org.openlumen.jarvis";
const USERNAME: &str = "google-oauth";

/// Tokens plus the OAuth client credentials needed to refresh them,
/// persisted as a single JSON secret in the OS credential store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredTokens {
    pub refresh_token: String,
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    /// Stored so `oauth::refresh_access_token` can run without prompting the
    /// user again for their Google Cloud Desktop-app OAuth client.
    pub client_id: String,
    pub client_secret: String,
}

fn entry() -> Result<Entry> {
    Entry::new(SERVICE, USERNAME).context("failed to open credential store entry")
}

/// Serializes `tokens` to JSON and writes it as the secret for the
/// well-known Jarvis Google OAuth credential entry, overwriting any
/// previously stored value.
pub fn save(tokens: &StoredTokens) -> Result<()> {
    let json = serde_json::to_vec(tokens).context("failed to serialize tokens")?;
    entry()?
        .set_secret(&json)
        .context("failed to write tokens to credential store")?;
    Ok(())
}

/// Loads and deserializes the stored tokens, or `Ok(None)` if no entry has
/// been saved yet (or it was previously cleared).
pub fn load() -> Result<Option<StoredTokens>> {
    match entry()?.get_secret() {
        Ok(bytes) => {
            let tokens = serde_json::from_slice(&bytes)
                .context("failed to deserialize stored tokens")?;
            Ok(Some(tokens))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err).context("failed to read tokens from credential store"),
    }
}

/// Deletes the stored credential entry, if any. Treats "no entry" as
/// success since the end state (no stored tokens) is already achieved.
pub fn clear() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err).context("failed to delete tokens from credential store"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> StoredTokens {
        StoredTokens {
            refresh_token: "refresh-abc".to_string(),
            access_token: "access-xyz".to_string(),
            expires_at: Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
            client_id: "client-id-123".to_string(),
            client_secret: "client-secret-456".to_string(),
        }
    }

    #[test]
    fn stored_tokens_json_round_trip() {
        let original = sample();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: StoredTokens = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn stored_tokens_json_uses_expected_keys() {
        let json = serde_json::to_value(sample()).expect("serialize to value");
        let obj = json.as_object().expect("object");
        for key in [
            "refresh_token",
            "access_token",
            "expires_at",
            "client_id",
            "client_secret",
        ] {
            assert!(obj.contains_key(key), "missing key: {key}");
        }
    }

    // Exercises the real Windows Credential Manager. Ignored by default so
    // that `cargo test` never touches the machine's credential store; run
    // explicitly with `cargo test -- --ignored` to verify manually. Cleans
    // up after itself via `clear()`.
    #[test]
    #[ignore]
    fn save_load_clear_round_trip_against_real_credential_store() {
        let tokens = sample();
        save(&tokens).expect("save");
        let loaded = load().expect("load").expect("entry should exist");
        assert_eq!(loaded, tokens);
        clear().expect("clear");
        assert!(load().expect("load after clear").is_none());
    }
}
