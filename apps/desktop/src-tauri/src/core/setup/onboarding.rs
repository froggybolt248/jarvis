//! First-run onboarding state, persisted in the `settings` table.
//!
//! The onboarding wizard runs exactly once. Completion is recorded as a
//! durable setting so that (a) the frontend can decide whether to show the
//! wizard or the dashboard on launch, and (b) side effects that must only
//! happen after a real first run — most importantly registering the app for
//! Windows autostart — are gated behind [`OnboardingState::complete`].
//!
//! State is intentionally minimal and derived entirely from `settings`; there
//! is no separate table to keep in sync.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::core::db::Db;

/// Setting key holding `"true"` once onboarding has finished.
pub const ONBOARDING_COMPLETE_SETTING: &str = "onboarding_complete";
/// Setting key holding the comma-separated list of domains the user enabled
/// on the welcome screen (e.g. `"calendar,diet,study"`).
pub const DOMAINS_SETTING: &str = "enabled_domains";

/// Snapshot of onboarding progress, surfaced to the frontend on launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingState {
    /// Whether the user has finished the wizard at least once.
    pub complete: bool,
    /// Domains the user chose to enable (may be empty until the welcome step).
    pub domains: Vec<String>,
}

/// Read the current onboarding state from persisted settings.
pub fn get_state(db: &Db) -> Result<OnboardingState> {
    let complete = db.get_setting(ONBOARDING_COMPLETE_SETTING)?.as_deref() == Some("true");
    let domains = db
        .get_setting(DOMAINS_SETTING)?
        .map(|s| parse_domains(&s))
        .unwrap_or_default();
    Ok(OnboardingState { complete, domains })
}

/// Persist the user's chosen domains without marking onboarding complete.
/// Safe to call repeatedly as the user toggles choices on the welcome screen.
pub fn set_domains(db: &Db, domains: &[String]) -> Result<()> {
    db.set_setting(DOMAINS_SETTING, &domains.join(","))?;
    Ok(())
}

/// Mark onboarding finished, persisting the final domain selection. After this
/// returns `Ok`, [`get_state`] reports `complete == true` and the caller may
/// perform first-run-only side effects (e.g. registering autostart).
pub fn complete(db: &Db, domains: &[String]) -> Result<()> {
    set_domains(db, domains)?;
    db.set_setting(ONBOARDING_COMPLETE_SETTING, "true")?;
    Ok(())
}

/// Clear the completion flag so the wizard runs again (used by "reset" in
/// Settings and by tests). Domain selections are left intact.
pub fn reset(db: &Db) -> Result<()> {
    db.set_setting(ONBOARDING_COMPLETE_SETTING, "false")?;
    Ok(())
}

fn parse_domains(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Db {
        Db::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn fresh_db_is_not_complete_and_has_no_domains() {
        let db = mem_db();
        let state = get_state(&db).unwrap();
        assert!(!state.complete);
        assert!(state.domains.is_empty());
    }

    #[test]
    fn set_domains_roundtrips_without_completing() {
        let db = mem_db();
        set_domains(&db, &["calendar".into(), "diet".into()]).unwrap();
        let state = get_state(&db).unwrap();
        assert!(!state.complete);
        assert_eq!(state.domains, vec!["calendar", "diet"]);
    }

    #[test]
    fn complete_sets_flag_and_domains() {
        let db = mem_db();
        complete(&db, &["study".into()]).unwrap();
        let state = get_state(&db).unwrap();
        assert!(state.complete);
        assert_eq!(state.domains, vec!["study"]);
    }

    #[test]
    fn reset_clears_completion_but_keeps_domains() {
        let db = mem_db();
        complete(&db, &["gym".into()]).unwrap();
        reset(&db).unwrap();
        let state = get_state(&db).unwrap();
        assert!(!state.complete);
        assert_eq!(state.domains, vec!["gym"]);
    }

    #[test]
    fn parse_domains_trims_and_drops_empties() {
        assert_eq!(parse_domains(" calendar , , diet ,"), vec!["calendar", "diet"]);
        assert!(parse_domains("").is_empty());
    }
}
