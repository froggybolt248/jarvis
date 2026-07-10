//! Reusable loader for the persisted ntfy configuration, shared by the
//! `ntfy_*` commands and the scheduler/dispatch chokepoint.

use anyhow::Result;

use crate::core::db::Db;
use crate::core::notify::ntfy::NtfyConfig;

/// Setting keys for the persisted ntfy configuration.
const NTFY_BASE_URL: &str = "ntfy_base_url";
const NTFY_TOPIC: &str = "ntfy_topic";
const DEFAULT_BASE_URL: &str = "https://ntfy.sh";

/// Load the persisted ntfy config, or `None` if no topic has been set up yet.
pub fn load_config(db: &Db) -> Result<Option<NtfyConfig>> {
    let topic = db
        .get_setting(NTFY_TOPIC)?
        .filter(|t| !t.trim().is_empty());
    let Some(topic) = topic else {
        return Ok(None);
    };
    let base_url = db
        .get_setting(NTFY_BASE_URL)?
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    Ok(Some(NtfyConfig { base_url, topic }))
}
