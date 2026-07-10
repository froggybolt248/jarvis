// WP-Agent-Tools owns this file.

//! `create_calendar_event`: the agent-facing calendar tool. Mutates both
//! Google Calendar (via the API) and the local SQLite cache, and logs a
//! Quiet Feed row as its audit record.

use anyhow::Result;
use serde_json::Value;

use crate::core::agent::provider::ToolDef;
use crate::core::db::queries::quiet_feed::QuietFeedItem;
use crate::core::google;
use crate::core::google::calendar_client::{CalendarClient, CalendarEvent as ApiCalendarEvent, EventDateTime};
use crate::core::google::sync::api_event_to_db;

use super::{Tool, ToolContext};

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
