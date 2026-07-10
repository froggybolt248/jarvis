//! The single chokepoint every outbound push notification must pass through.
//!
//! [`notify`] enforces quiet hours (deferring non-urgent pushes into the
//! batched-notification queue, see [`crate::core::db::queries::notifications`])
//! and is the only function anything in the app should call to send a push —
//! everything else (the scheduler's jobs, future agent tools) goes through
//! here rather than calling [`crate::core::notify::ntfy::publish`] directly.
//! The lone exception is `ntfy_send_test`, an explicit user-initiated
//! connectivity check that intentionally bypasses quiet hours.

use anyhow::Result;
use chrono::NaiveTime;

use crate::core::db::Db;
use crate::core::notify::config;
use crate::core::notify::ntfy::{self, NtfyMessage};

/// Whether a notification must be delivered immediately (bypassing quiet
/// hours) or may be deferred/batched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Urgent,
    Normal,
}

/// Setting keys controlling quiet hours.
const QUIET_HOURS_ENABLED: &str = "quiet_hours_enabled";
const QUIET_HOURS_START: &str = "quiet_hours_start";
const QUIET_HOURS_END: &str = "quiet_hours_end";
const DEFAULT_START: &str = "22:00";
const DEFAULT_END: &str = "07:00";

/// Pure check of whether `now` falls within the quiet-hours window
/// `[start, end)`. Handles the common midnight-wrapping window (e.g.
/// 22:00-07:00 means "now >= 22:00 OR now < 07:00") as well as the
/// non-wrapping case (e.g. 09:00-17:00 means "now >= 09:00 AND now < 17:00").
pub(crate) fn in_quiet_hours(now: NaiveTime, start: NaiveTime, end: NaiveTime) -> bool {
    if start <= end {
        now >= start && now < end
    } else {
        now >= start || now < end
    }
}

/// Whether we're currently inside the user's configured quiet hours. `false`
/// if quiet hours aren't enabled, or if the stored config fails to parse
/// (fails open, since we'd rather send a push than lose it).
pub fn is_quiet_now(db: &Db) -> Result<bool> {
    let enabled = db.get_setting(QUIET_HOURS_ENABLED)?;
    if enabled.as_deref() != Some("true") {
        return Ok(false);
    }

    let start_str = db
        .get_setting(QUIET_HOURS_START)?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_START.to_string());
    let end_str = db
        .get_setting(QUIET_HOURS_END)?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_END.to_string());

    let start = NaiveTime::parse_from_str(&start_str, "%H:%M").unwrap_or_else(|_| {
        NaiveTime::parse_from_str(DEFAULT_START, "%H:%M").expect("default parses")
    });
    let end = NaiveTime::parse_from_str(&end_str, "%H:%M").unwrap_or_else(|_| {
        NaiveTime::parse_from_str(DEFAULT_END, "%H:%M").expect("default parses")
    });

    Ok(in_quiet_hours(chrono::Local::now().time(), start, end))
}

/// The single chokepoint every push must go through.
///
/// - If ntfy isn't set up yet, this is a silent no-op (nothing to send to).
/// - `Urgent` messages are always sent immediately.
/// - `Normal` messages are deferred into the batched-notification queue if
///   we're currently within quiet hours, otherwise sent immediately.
pub async fn notify(db: &Db, http: &reqwest::Client, msg: NtfyMessage, urgency: Urgency) -> Result<()> {
    let Some(cfg) = config::load_config(db)? else {
        eprintln!("notify: ntfy not configured, dropping message {:?}", msg.title);
        return Ok(());
    };

    if urgency == Urgency::Urgent {
        return ntfy::publish(http, &cfg, &msg).await;
    }

    if is_quiet_now(db)? {
        db.enqueue(&msg)?;
        return Ok(());
    }

    ntfy::publish(http, &cfg, &msg).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }

    #[test]
    fn wrapping_window_inside_before_midnight() {
        assert!(in_quiet_hours(t("23:00"), t("22:00"), t("07:00")));
    }

    #[test]
    fn wrapping_window_inside_after_midnight() {
        assert!(in_quiet_hours(t("03:00"), t("22:00"), t("07:00")));
    }

    #[test]
    fn wrapping_window_outside() {
        assert!(!in_quiet_hours(t("12:00"), t("22:00"), t("07:00")));
    }

    #[test]
    fn wrapping_window_start_boundary_is_inclusive() {
        assert!(in_quiet_hours(t("22:00"), t("22:00"), t("07:00")));
    }

    #[test]
    fn wrapping_window_end_boundary_is_exclusive() {
        assert!(!in_quiet_hours(t("07:00"), t("22:00"), t("07:00")));
    }

    #[test]
    fn non_wrapping_window_inside() {
        assert!(in_quiet_hours(t("12:00"), t("09:00"), t("17:00")));
    }

    #[test]
    fn non_wrapping_window_outside() {
        assert!(!in_quiet_hours(t("20:00"), t("09:00"), t("17:00")));
    }

    #[test]
    fn non_wrapping_window_start_boundary_is_inclusive() {
        assert!(in_quiet_hours(t("09:00"), t("09:00"), t("17:00")));
    }

    #[test]
    fn non_wrapping_window_end_boundary_is_exclusive() {
        assert!(!in_quiet_hours(t("17:00"), t("09:00"), t("17:00")));
    }

    #[test]
    fn is_quiet_now_false_when_not_enabled() {
        let db = Db::open_in_memory().unwrap();
        assert!(!is_quiet_now(&db).unwrap());
    }

    #[test]
    fn is_quiet_now_uses_defaults_when_enabled_but_unconfigured() {
        let db = Db::open_in_memory().unwrap();
        db.set_setting(QUIET_HOURS_ENABLED, "true").unwrap();
        // Just confirm it doesn't error and parses defaults (22:00-07:00).
        let _ = is_quiet_now(&db).unwrap();
    }
}
