// WP-Agent-Tools owns this file.

//! `create_calendar_event` (mutating) and `get_calendar_events` (read-only):
//! the agent-facing calendar tools. The create tool mutates both Google
//! Calendar (via the API) and the local SQLite cache, and logs a Quiet Feed
//! row as its audit record; the read tool only reads the local cache.

use anyhow::Result;
use chrono::Local;
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::quiet_feed::QuietFeedItem;
use crate::core::google;
use crate::core::google::calendar_client::{CalendarClient, CalendarEvent as ApiCalendarEvent, EventDateTime};
use crate::core::google::sync::api_event_to_db;

use super::{Tool, ToolContext};

const DEFAULT_DAYS_AHEAD: i64 = 1;
const MAX_DAYS_AHEAD: i64 = 31;

/// Extract a required string field from a JSON object.
fn required_str<'a>(args: &'a Value, field: &str) -> Result<&'a str> {
    args.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing or non-string required field '{field}'"))
}

/// Extract an optional string field from a JSON object.
fn optional_str(args: &Value, field: &str) -> Option<String> {
    args.get(field).and_then(Value::as_str).map(str::to_string)
}

/// Extract an optional non-negative integer field, clamped to `[1, max]`,
/// defaulting to `default` when absent.
fn optional_clamped_i64(args: &Value, field: &str, default: i64, max: i64) -> Result<i64> {
    match args.get(field) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let n = v
                .as_i64()
                .ok_or_else(|| anyhow::anyhow!("field '{field}' must be an integer"))?;
            Ok(n.clamp(1, max))
        }
    }
}

/// Creates a new event on the user's primary Google calendar.
pub struct CreateCalendarEvent;

#[async_trait::async_trait]
impl Tool for CreateCalendarEvent {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "create_calendar_event".to_string(),
            description: "Create a new event on the user's primary Google calendar. This \
                mutates the user's calendar, so only use it when the user has clearly asked to \
                schedule or create something."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "summary": {
                        "type": "string",
                        "description": "The event title."
                    },
                    "start": {
                        "type": "string",
                        "description": "Event start time, RFC3339 (e.g. 2026-07-10T09:00:00-07:00)."
                    },
                    "end": {
                        "type": "string",
                        "description": "Event end time, RFC3339."
                    },
                    "description": {
                        "type": "string",
                        "description": "Optional event description/notes."
                    },
                    "location": {
                        "type": "string",
                        "description": "Optional event location."
                    }
                },
                "required": ["summary", "start", "end"],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let summary = required_str(args, "summary")?;
        let start = required_str(args, "start")?;
        let end = required_str(args, "end")?;
        let description = optional_str(args, "description");
        let location = optional_str(args, "location");

        let token = google::valid_access_token().await?;

        let event = ApiCalendarEvent {
            id: None,
            summary: Some(summary.to_string()),
            description,
            location,
            start: EventDateTime {
                date_time: Some(start.to_string()),
                date: None,
                time_zone: None,
            },
            end: EventDateTime {
                date_time: Some(end.to_string()),
                date: None,
                time_zone: None,
            },
            status: None,
        };

        let created = CalendarClient::new(token)
            .insert_event("primary", &event)
            .await?;

        if let Some(db_event) = api_event_to_db(&created, "primary") {
            ctx.db.upsert_event(&db_event)?;
        }

        let item = QuietFeedItem {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            kind: "calendar".to_string(),
            title: format!("Created event '{summary}'"),
            body: Some(format!("{start} \u{2013} {end}")),
            deep_link: None,
            source: Some("create_calendar_event".to_string()),
        };
        ctx.db.insert_feed(&item)?;

        Ok(format!("Created event '{summary}' from {start} to {end}."))
    }
}

/// Read-only: lists events from the local calendar cache, from the start of
/// today (local time) through `days_ahead` days ahead.
pub struct GetCalendarEvents;

#[async_trait::async_trait]
impl Tool for GetCalendarEvents {
    fn def(&self) -> ToolDef {
        ToolDef {
            name: "get_calendar_events".to_string(),
            description: "Get the user's calendar events from the start of today through N days \
                ahead. Use this before answering any question about the user's schedule, \
                upcoming events, or what's on the calendar."
                .to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "days_ahead": {
                        "type": "integer",
                        "description": "How many days ahead to look, including today (default 1, max 31).",
                        "minimum": 1,
                        "maximum": MAX_DAYS_AHEAD
                    }
                },
                "required": [],
                "additionalProperties": false
            }),
        }
    }

    async fn execute(&self, ctx: &ToolContext<'_>, args: &Value) -> Result<String> {
        let days_ahead = optional_clamped_i64(args, "days_ahead", DEFAULT_DAYS_AHEAD, MAX_DAYS_AHEAD)?;

        let today = Local::now().format("%Y-%m-%d").to_string();
        let start = format!("{today}T00:00:00");
        let end_date = (Local::now() + chrono::Duration::days(days_ahead))
            .format("%Y-%m-%d")
            .to_string();
        let end = format!("{end_date}T00:00:00");

        let events = ctx.db.list_events_between(&start, &end)?;
        if events.is_empty() {
            return Ok("No events in that range.".to_string());
        }

        let mut lines = Vec::with_capacity(events.len());
        for event in &events {
            let date = event
                .start_at
                .as_deref()
                .and_then(|s| s.split('T').next())
                .unwrap_or("?");
            let time = if event.all_day {
                "all day".to_string()
            } else {
                event
                    .start_at
                    .as_deref()
                    .and_then(|s| s.split('T').nth(1))
                    .map(|t| t.chars().take(5).collect::<String>())
                    .unwrap_or_else(|| "?".to_string())
            };
            let summary = event.summary.as_deref().unwrap_or("(untitled)");
            let location = event
                .location
                .as_deref()
                .map(|l| format!(" @ {l}"))
                .unwrap_or_default();
            lines.push(format!("- {date} {time} {summary}{location}"));
        }

        Ok(format!("{} event(s):\n{}", events.len(), lines.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ChatProvider, ProviderHealth};
    use crate::core::db::queries::calendar::CalendarEvent as DbCalendarEvent;
    use crate::core::db::Db;
    use crate::core::memory::Vault;
    use futures_util::stream::BoxStream;
    use tempfile::tempdir;

    struct StubProvider;

    #[async_trait::async_trait]
    impl ChatProvider for StubProvider {
        async fn chat_stream(
            &self,
            _messages: Vec<ChatMessage>,
            _opts: ChatOptions,
        ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
            anyhow::bail!("not implemented in stub")
        }

        async fn embed(&self, _texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            anyhow::bail!("not implemented in stub")
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("not implemented in stub")
        }
    }

    fn ctx_parts() -> (Db, tempfile::TempDir, Vault, StubProvider) {
        let db = Db::open_in_memory().unwrap();
        let dir = tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (db, dir, vault, StubProvider)
    }

    #[tokio::test]
    async fn get_calendar_events_returns_events_today() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let today = Local::now().format("%Y-%m-%d").to_string();
        db.upsert_event(&DbCalendarEvent {
            id: "evt-1".to_string(),
            google_id: None,
            calendar_id: None,
            summary: Some("Standup".to_string()),
            description: None,
            location: Some("Room 2".to_string()),
            start_at: Some(format!("{today}T09:00:00")),
            end_at: Some(format!("{today}T09:30:00")),
            all_day: false,
            status: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .unwrap();

        let tool = GetCalendarEvents;
        let args = serde_json::json!({});
        let result = tool.execute(&ctx, &args).await.unwrap();

        assert!(result.contains("1 event(s)"), "got: {result}");
        assert!(result.contains("Standup"), "got: {result}");
        assert!(result.contains("09:00"), "got: {result}");
        assert!(result.contains("Room 2"), "got: {result}");
    }

    #[tokio::test]
    async fn get_calendar_events_returns_message_when_empty() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let tool = GetCalendarEvents;
        let args = serde_json::json!({});
        let result = tool.execute(&ctx, &args).await.unwrap();
        assert_eq!(result, "No events in that range.");
    }

    #[tokio::test]
    async fn get_calendar_events_respects_days_ahead() {
        let (db, _dir, vault, provider) = ctx_parts();
        let ctx = ToolContext { db: &db, vault: &vault, provider: &provider };

        let future = (Local::now() + chrono::Duration::days(5))
            .format("%Y-%m-%d")
            .to_string();
        db.upsert_event(&DbCalendarEvent {
            id: "evt-2".to_string(),
            google_id: None,
            calendar_id: None,
            summary: Some("Far out event".to_string()),
            description: None,
            location: None,
            start_at: Some(format!("{future}T12:00:00")),
            end_at: None,
            all_day: false,
            status: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        })
        .unwrap();

        let tool = GetCalendarEvents;
        let default_args = serde_json::json!({});
        let default_result = tool.execute(&ctx, &default_args).await.unwrap();
        assert_eq!(default_result, "No events in that range.");

        let wide_args = serde_json::json!({"days_ahead": 10});
        let wide_result = tool.execute(&ctx, &wide_args).await.unwrap();
        assert!(wide_result.contains("Far out event"), "got: {wide_result}");
    }
}
