//! Process-wide application state, initialized once at startup and shared
//! with every Tauri command via `tauri::State`.
//!
//! Owns the three long-lived core services:
//! - [`Db`]: the SQLite index (chunks/FTS/vec + all structured domains).
//! - [`Vault`]: the markdown source-of-truth directory. Held behind an
//!   `RwLock` so onboarding can relocate it after first run.
//! - [`OllamaProvider`]: the local LLM provider.

use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::{Context, Result};

use crate::core::agent::provider::OllamaProvider;
use crate::core::agent::tools::ToolRegistry;
use crate::core::db::Db;
use crate::core::memory::Vault;

/// Settings key holding the user-chosen vault directory (absolute path).
pub const VAULT_PATH_SETTING: &str = "vault_path";

pub struct AppState {
    pub db: Db,
    pub provider: OllamaProvider,
    pub vault: RwLock<Vault>,
    pub app_data_dir: PathBuf,
    /// Shared HTTP client for outbound calls that aren't the LLM provider's
    /// own (ntfy push, Google Calendar). Cloning is cheap (Arc inside).
    pub http: reqwest::Client,
    /// The tools exposed to the agent loop. Stateless/cheap to build; shared
    /// across every turn.
    pub registry: ToolRegistry,
}

impl AppState {
    /// Initialize all core services under `app_data_dir` (typically
    /// `%APPDATA%/Jarvis`). The SQLite database lives directly in this dir;
    /// the vault lives wherever the persisted `vault_path` setting points, or
    /// at the default location (`%USERPROFILE%/JarvisVault`) on first run.
    pub fn bootstrap(app_data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)
            .with_context(|| format!("creating app data dir {}", app_data_dir.display()))?;

        let db = Db::open(&app_data_dir.join("jarvis.db"))?;

        let vault_path = match db.get_setting(VAULT_PATH_SETTING)? {
            Some(p) if !p.trim().is_empty() => PathBuf::from(p),
            _ => default_vault_path()?,
        };
        let vault = Vault::open(&vault_path)?;

        let provider = OllamaProvider::default();

        Ok(Self {
            db,
            provider,
            vault: RwLock::new(vault),
            app_data_dir,
            http: reqwest::Client::new(),
            registry: ToolRegistry::with_defaults(),
        })
    }

    /// Relocate the vault to `new_root`, opening (and seeding) it there and
    /// persisting the choice. Used by onboarding and settings.
    pub fn set_vault_path(&self, new_root: &Path) -> Result<()> {
        let vault = Vault::open(new_root)?;
        self.db
            .set_setting(VAULT_PATH_SETTING, &new_root.to_string_lossy())?;
        *self.vault.write().expect("vault lock poisoned") = vault;
        Ok(())
    }
}

/// Default vault location on first run: `%USERPROFILE%/JarvisVault`
/// (falls back to `$HOME` on non-Windows dev machines).
pub fn default_vault_path() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .context("no home directory (USERPROFILE/HOME unset)")?;
    Ok(PathBuf::from(home).join("JarvisVault"))
}
