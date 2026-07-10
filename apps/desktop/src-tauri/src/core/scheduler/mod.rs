//! Background scheduler: periodic jobs for calendar sync, the batched
//! notification flush, and the morning briefing.
//!
//! Uses `tokio-cron-scheduler`'s `JobScheduler`, which internally spawns its
//! own Tokio task to tick pending jobs (see `JobScheduler::start`). We keep
//! the `JobScheduler` value alive for the lifetime of the app by
//! `std::mem::forget`-ing it after `start()`: the scheduler is meant to run
//! for as long as the process does, there is exactly one instance, and
//! leaking it is simpler and just as correct as threading a `OnceLock`/global
//! through `AppState` only to never actually drop it before exit.

use anyhow::{Context, Result};
use chrono::Local;
use tauri::{AppHandle, Manager};
use tokio_cron_scheduler::{Job, JobScheduler};

use crate::app_state::AppState;
use crate::core::db::queries::calendar::CalendarEvent;
use crate::core::db::queries::diet::DietTargets;
use crate::core::db::Db;
use crate::core::google;
use crate::core::notify::dispatch::{self, Urgency};
use crate::core::notify::ntfy::{self, NtfyMessage};

/// Start the app-lifetime background scheduler: calendar sync every 15
/// minutes, the batched-notification flush every 5 minutes, and the morning
/// briefing daily at 07:00 local time.
pub async fn start(app: AppHandle) -> Result<()> {
    let sched = JobScheduler::new().await.context("creating JobScheduler")?;

    // --- calendar_sync: every 15 minutes ---
    {
        let app = app.clone();
        sched
            .add(
                Job::new_async("0 */15 * * * *", move |_uuid, _lock| {
                    let app = app.clone();
                    Box::pin(async move {
                        let state = app.state::<AppState>();
                        if let Err(err) = google::sync::sync_calendar(&state.db, "primary").await {
                            eprintln!("scheduler: calendar_sync failed: {err:#}");
                        }
                    })
                })
                .context("building calendar_sync job")?,
            )
            .await
            .context("adding calendar_sync job")?;
    }

    // --- flush_notifications: every 5 minutes ---
    {
        let app = app.clone();
        sched
            .add(
                Job::new_async("0 */5 * * * *", move |_uuid, _lock| {
                    let app = app.clone();
                    Box::pin(async move {
                        let state = app.state::<AppState>();
                        if let Err(err) = flush_notifications(&state.db, &state.http).await {
                            eprintln!("scheduler: flush_notifications failed: {err:#}");
                        }
                    })
                })
                .context("building flush_notifications job")?,
            )
            .await
            .context("adding flush_notifications job")?;
    }

    // --- morning_briefing: daily at 07:00 local ---
    {
        let app = app.clone();
        sched
            .add(
                Job::new_async("0 0 7 * * *", move |_uuid, _lock| {
                    let app = app.clone();
                    Box::pin(async move {
                        let state = app.state::<AppState>();
                        if let Err(err) = run_morning_briefing(&state.db, &state.http).await {
                            eprintln!("scheduler: morning_briefing failed: {err:#}");
                        }
                    })
                })
                .context("building morning_briefing job")?,
            )
            .await
            .context("adding morning_briefing job")?;
    }

    // Note: a `missed_event_watch` job (nudging about calendar events about
    // to start with no acknowledgement) was scoped as optional/skip-if-not-
    // trivial in the brief. Skipped here as a deliberate follow-up: it needs
    // a notion of "already notified for event X" to avoid re-notifying every
    // tick, which is more than trivial state to bolt on in this slice.

    sched.start().await.context("starting JobScheduler")?;

    // Keep the scheduler alive for the lifetime of the app (see module doc).
    std::mem::forget(sched);

    Ok(())
}

/// If we're outside quiet hours and there's anything queued, drain the
/// batched-notification queue and send it as a single digest push. This is
/// itself the post-quiet-hours drain, so it publishes directly rather than
/// going through `dispatch::notify` (which would just re-defer it).
async fn flush_notifications(db: &Db, http: &reqwest::Client) -> Result<()> {
    if dispatch::is_quiet_now(db)? {
        return Ok(());
    }
    if db.queued_count()? == 0 {
        return Ok(());
    }

    let queued = db.drain_queued()?;
    if queued.is_empty() {
        return Ok(());
    }

    let Some(cfg) = crate::core::notify::config::load_config(db)? else {
        return Ok(());
    };

    let digest = NtfyMessage {
        title: format!("Jarvis: {} updates", queued.len()),
        body: queued
            .iter()
            .map(|m| m.title.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        priority: Some(3),
        tags: vec![],
        click: None,
    };

    ntfy::publish(http, &cfg, &digest).await
}

async fn run_morning_briefing(db: &Db, http: &reqwest::Client) -> Result<()> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let start = format!("{today}T00:00:00");
    let end_date = (Local::now() + chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let end = format!("{end_date}T00:00:00");

    let events = db.list_events_between(&start, &end)?;
    let due = db.due_cards(&chrono::Utc::now().to_rfc3339())?;
    let targets = db.current_targets()?;

    let msg = compose_briefing(&events, due.len(), targets.as_ref());
    dispatch::notify(db, http, msg, Urgency::Normal).await
}

/// Pure: compose the morning briefing message from today's events, the
/// number of due study cards, and (if set) the current diet calorie target.
pub(crate) fn compose_briefing(
    events: &[CalendarEvent],
    due_card_count: usize,
    targets: Option<&DietTargets>,
) -> NtfyMessage {
    let mut lines = Vec::new();

    if events.is_empty() {
        lines.push("No events today.".to_string());
    } else {
        for event in events {
            let summary = event.summary.as_deref().unwrap_or("(untitled)");
            if event.all_day {
                lines.push(format!("- {summary} (all day)"));
            } else {
                let time = event
                    .start_at
                    .as_deref()
                    .and_then(|s| s.split('T').nth(1))
                    .map(|t| t.chars().take(5).collect::<String>())
                    .unwrap_or_else(|| "?".to_string());
                lines.push(format!("- {time} {summary}"));
            }
        }
    }

    lines.push(String::new());
    lines.push(format!("{due_card_count} study cards due today."));

    if let Some(targets) = targets {
        if let Some(calories) = targets.calories {
            lines.push(format!("Calorie target: {calories} kcal."));
        }
    }

    NtfyMessage {
        title: "Jarvis: morning briefing".to_string(),
        body: lines.join("\n"),
        priority: Some(3),
        tags: vec!["sunrise".to_string()],
        click: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(summary: &str, start_at: &str, all_day: bool) -> CalendarEvent {
        CalendarEvent {
            id: "id".to_string(),
            google_id: None,
            calendar_id: None,
            summary: Some(summary.to_string()),
            description: None,
            location: None,
            start_at: Some(start_at.to_string()),
            end_at: None,
            all_day,
            status: None,
            updated_at: "2026-07-10T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn compose_briefing_no_events() {
        let msg = compose_briefing(&[], 0, None);
        assert!(msg.body.contains("No events today."));
        assert!(msg.body.contains("0 study cards due today."));
    }

    #[test]
    fn compose_briefing_includes_events_and_due_cards() {
        let events = vec![
            event("Standup", "2026-07-10T09:00:00", false),
            event("Team offsite", "2026-07-10T00:00:00", true),
        ];
        let msg = compose_briefing(&events, 5, None);
        assert!(msg.body.contains("09:00 Standup"));
        assert!(msg.body.contains("Team offsite (all day)"));
        assert!(msg.body.contains("5 study cards due today."));
    }

    #[test]
    fn compose_briefing_includes_calorie_target_when_set() {
        let targets = DietTargets {
            id: "t1".to_string(),
            effective_date: "2026-07-01".to_string(),
            calories: Some(2200),
            protein_g: None,
            carbs_g: None,
            fat_g: None,
            created_at: "2026-07-01T00:00:00Z".to_string(),
        };
        let msg = compose_briefing(&[], 0, Some(&targets));
        assert!(msg.body.contains("Calorie target: 2200 kcal."));
    }

    #[test]
    fn compose_briefing_omits_calorie_line_when_no_target_set() {
        let targets = DietTargets {
            id: "t1".to_string(),
            effective_date: "2026-07-01".to_string(),
            calories: None,
            protein_g: None,
            carbs_g: None,
            fat_g: None,
            created_at: "2026-07-01T00:00:00Z".to_string(),
        };
        let msg = compose_briefing(&[], 0, Some(&targets));
        assert!(!msg.body.contains("Calorie target"));
    }
}
