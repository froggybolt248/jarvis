// WP-Google owns this file.
//
// Minimal Google Calendar API v3 client. Uses this workspace's own
// `reqwest` dependency (0.13.4) directly — this is unrelated to (and a
// different crate instance than) the `oauth2::reqwest` re-export used in
// `oauth.rs` for the token exchange.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

/// A single entry from the user's calendar list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalendarListEntry {
    pub id: String,
    pub summary: Option<String>,
    pub primary: Option<bool>,
}

/// A start/end timestamp on an event: either an all-day `date` or a
/// zoned `dateTime`, per the Google Calendar API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EventDateTime {
    pub date_time: Option<String>,
    pub date: Option<String>,
    pub time_zone: Option<String>,
}

/// A Google Calendar event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: Option<String>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub start: EventDateTime,
    pub end: EventDateTime,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CalendarListResponse {
    #[serde(default)]
    items: Vec<CalendarListEntry>,
}

#[derive(Debug, Deserialize)]
struct EventsListResponse {
    #[serde(default)]
    items: Vec<CalendarEvent>,
}

/// Minimal typed client for the Google Calendar API v3.
pub struct CalendarClient {
    access_token: String,
    http: reqwest::Client,
    base_url: String,
}

impl CalendarClient {
    /// Creates a client using the real Google Calendar API base URL.
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            http: reqwest::Client::new(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Creates a client pointed at a custom base URL (e.g. a `wiremock`
    /// mock server in tests). `base_url` should have no trailing slash.
    pub fn with_base_url(access_token: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            access_token: access_token.into(),
            http: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    /// Lists the calendars on the authenticated user's calendar list.
    pub async fn list_calendars(&self) -> Result<Vec<CalendarListEntry>> {
        let url = format!("{}/users/me/calendarList", self.base_url);
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        let response = Self::ensure_success(response).await?;
        let parsed: CalendarListResponse = response.json().await?;
        Ok(parsed.items)
    }

    /// Lists single (non-recurring-expanded... expanded, actually) events
    /// on `calendar_id` between `time_min` and `time_max` (RFC3339
    /// timestamps), ordered by start time.
    pub async fn list_events(
        &self,
        calendar_id: &str,
        time_min: &str,
        time_max: &str,
    ) -> Result<Vec<CalendarEvent>> {
        // Note: the `query()` builder method requires reqwest's optional
        // "query" feature, which is not enabled in this workspace, so the
        // query string is built manually here.
        let url = format!(
            "{}/calendars/{}/events?timeMin={}&timeMax={}&singleEvents=true&orderBy=startTime",
            self.base_url,
            urlencode(calendar_id),
            urlencode(time_min),
            urlencode(time_max),
        );
        let response = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        let response = Self::ensure_success(response).await?;
        let parsed: EventsListResponse = response.json().await?;
        Ok(parsed.items)
    }

    /// Inserts a new event on `calendar_id`, returning the event as created
    /// (including its server-assigned `id`).
    pub async fn insert_event(
        &self,
        calendar_id: &str,
        event: &CalendarEvent,
    ) -> Result<CalendarEvent> {
        let url = format!(
            "{}/calendars/{}/events",
            self.base_url,
            urlencode(calendar_id)
        );
        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(event)
            .send()
            .await?;
        let response = Self::ensure_success(response).await?;
        let created: CalendarEvent = response.json().await?;
        Ok(created)
    }

    /// Returns `Ok(response)` for 2xx responses, otherwise reads the body
    /// and returns an `Err` including Google's error payload.
    async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            Ok(response)
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<failed to read body>".to_string());
            Err(anyhow!(
                "Google Calendar API request failed: {status} - {body}"
            ))
        }
    }
}

/// URL-encodes a single path segment (e.g. a calendar id, which may be an
/// email address containing `@`).
fn urlencode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn urlencode_escapes_special_chars() {
        assert_eq!(urlencode("primary"), "primary");
        assert_eq!(urlencode("a@b.com"), "a%40b.com");
    }

    #[tokio::test]
    async fn list_calendars_parses_typed_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {"id": "primary", "summary": "Main Calendar", "primary": true},
                    {"id": "work@group.calendar.google.com", "summary": "Work"}
                ]
            })))
            .mount(&server)
            .await;

        let client = CalendarClient::with_base_url("test-token", server.uri());
        let calendars = client.list_calendars().await.expect("request succeeds");

        assert_eq!(calendars.len(), 2);
        assert_eq!(calendars[0].id, "primary");
        assert_eq!(calendars[0].summary.as_deref(), Some("Main Calendar"));
        assert_eq!(calendars[0].primary, Some(true));
        assert_eq!(calendars[1].id, "work@group.calendar.google.com");
        assert_eq!(calendars[1].primary, None);
    }

    #[tokio::test]
    async fn list_events_parses_camel_case_datetimes() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/calendars/primary/events"))
            .and(query_param("timeMin", "2026-01-01T00:00:00Z"))
            .and(query_param("timeMax", "2026-01-02T00:00:00Z"))
            .and(query_param("singleEvents", "true"))
            .and(query_param("orderBy", "startTime"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "id": "evt1",
                        "summary": "Standup",
                        "start": {"dateTime": "2026-01-01T09:00:00-08:00", "timeZone": "America/Los_Angeles"},
                        "end": {"dateTime": "2026-01-01T09:30:00-08:00", "timeZone": "America/Los_Angeles"},
                        "status": "confirmed"
                    },
                    {
                        "id": "evt2",
                        "summary": "All-day offsite",
                        "start": {"date": "2026-01-02"},
                        "end": {"date": "2026-01-03"}
                    }
                ]
            })))
            .mount(&server)
            .await;

        let client = CalendarClient::with_base_url("test-token", server.uri());
        let events = client
            .list_events("primary", "2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z")
            .await
            .expect("request succeeds");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].summary.as_deref(), Some("Standup"));
        assert_eq!(
            events[0].start.date_time.as_deref(),
            Some("2026-01-01T09:00:00-08:00")
        );
        assert_eq!(
            events[0].start.time_zone.as_deref(),
            Some("America/Los_Angeles")
        );
        assert_eq!(events[1].start.date.as_deref(), Some("2026-01-02"));
        assert_eq!(events[1].start.date_time, None);
    }

    #[tokio::test]
    async fn insert_event_posts_and_parses_created_event() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/calendars/primary/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "new-event-id",
                "summary": "Created Event",
                "start": {"dateTime": "2026-03-01T10:00:00Z"},
                "end": {"dateTime": "2026-03-01T11:00:00Z"}
            })))
            .mount(&server)
            .await;

        let client = CalendarClient::with_base_url("test-token", server.uri());
        let new_event = CalendarEvent {
            summary: Some("Created Event".to_string()),
            start: EventDateTime {
                date_time: Some("2026-03-01T10:00:00Z".to_string()),
                ..Default::default()
            },
            end: EventDateTime {
                date_time: Some("2026-03-01T11:00:00Z".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let created = client
            .insert_event("primary", &new_event)
            .await
            .expect("request succeeds");

        assert_eq!(created.id.as_deref(), Some("new-event-id"));
        assert_eq!(created.summary.as_deref(), Some("Created Event"));
    }

    #[tokio::test]
    async fn non_success_response_bubbles_up_google_error_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/users/me/calendarList"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {"code": 401, "message": "Invalid Credentials"}
            })))
            .mount(&server)
            .await;

        let client = CalendarClient::with_base_url("test-token", server.uri());
        let err = client
            .list_calendars()
            .await
            .expect_err("should fail on 401");

        let message = err.to_string();
        assert!(message.contains("401"));
        assert!(message.contains("Invalid Credentials"));
    }
}
