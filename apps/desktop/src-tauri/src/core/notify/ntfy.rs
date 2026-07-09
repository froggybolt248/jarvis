// WP-Ntfy owns this file.
//! ntfy.sh phone push integration.
//!
//! ntfy (<https://ntfy.sh>) is a pub-sub push notification service. Publishing is a
//! plain `POST {base_url}/{topic}` with the notification body as the request body and
//! metadata carried in headers. Anyone subscribed to `{topic}` (e.g. via the ntfy phone
//! app) receives the push. The privacy model relies entirely on `{topic}` being an
//! unguessable secret, so topics are generated with high entropy (see
//! [`generate_topic`]).

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Configuration identifying where to publish/subscribe for phone push notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NtfyConfig {
    /// Base URL of the ntfy server, e.g. `https://ntfy.sh` or a self-hosted
    /// `https://push.example.com`. No trailing slash.
    pub base_url: String,
    /// Private topic name acting as the shared secret between this app and the
    /// subscribing phone.
    pub topic: String,
}

impl NtfyConfig {
    /// Build a config pointing at the publicly hosted `https://ntfy.sh` service.
    pub fn default_hosted(topic: String) -> Self {
        Self {
            base_url: "https://ntfy.sh".to_string(),
            topic,
        }
    }

    /// URL a browser/phone can use to view or subscribe to this topic over HTTP(S).
    pub fn subscribe_url(&self) -> String {
        format!("{}/{}", self.base_url, self.topic)
    }

    /// `ntfy://` deep link for the ntfy phone app (used for QR codes / one-tap
    /// subscribe). Strips the scheme from `base_url` to obtain the host.
    pub fn app_deep_link(&self) -> String {
        let host = strip_scheme(&self.base_url);
        format!("ntfy://{}/{}", host, self.topic)
    }
}

fn strip_scheme(base_url: &str) -> &str {
    base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
        .unwrap_or(base_url)
}

/// A single push notification to publish to a topic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NtfyMessage {
    pub title: String,
    pub body: String,
    /// ntfy priority, 1 (min) ..= 5 (max). `None` uses the server default (3).
    pub priority: Option<u8>,
    /// Emoji-mappable tags, comma-joined when sent (e.g. `["white_check_mark"]`).
    pub tags: Vec<String>,
    /// URL opened when the notification is tapped.
    pub click: Option<String>,
}

/// Generate an unguessable private topic name suitable for ntfy.
///
/// No `rand` crate is available in this workspace, so entropy is sourced from
/// [`uuid::Uuid::new_v4`], whose v4 variant is generated from a CSPRNG. Two v4 UUIDs
/// are rendered without dashes (32 hex chars each = 64 hex chars total, well above the
/// 24 chars we need) and truncated to 24 characters. Hex digits (`[0-9a-f]`) are a
/// subset of ntfy's allowed topic charset (`[A-Za-z0-9_-]`), so the result is always
/// valid without further sanitization.
pub fn generate_topic() -> String {
    let a = Uuid::new_v4().simple().to_string();
    let b = Uuid::new_v4().simple().to_string();
    let combined: String = a.chars().chain(b.chars()).take(24).collect();
    format!("jarvis-{combined}")
}

/// Publish a message to the configured ntfy topic.
///
/// Returns `Err` if the request fails to send or the server responds with a non-2xx
/// status (the response body is included in the error for diagnostics).
pub async fn publish(client: &reqwest::Client, cfg: &NtfyConfig, msg: &NtfyMessage) -> Result<()> {
    let url = format!("{}/{}", cfg.base_url, cfg.topic);
    let mut req = client.post(&url).body(msg.body.clone());

    req = req.header("Title", &msg.title);
    if let Some(priority) = msg.priority {
        req = req.header("Priority", priority.to_string());
    }
    if !msg.tags.is_empty() {
        req = req.header("Tags", msg.tags.join(","));
    }
    if let Some(click) = &msg.click {
        req = req.header("Click", click);
    }

    let resp = req
        .send()
        .await
        .with_context(|| format!("failed to send ntfy publish request to {url}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("ntfy publish to {url} failed with status {status}: {body}");
    }

    Ok(())
}

/// Publish a canned "connection established" notification, used to let the user
/// verify their phone is correctly subscribed to their topic.
pub async fn send_test(client: &reqwest::Client, cfg: &NtfyConfig) -> Result<()> {
    let msg = NtfyMessage {
        title: "Jarvis".to_string(),
        body: "✅ Jarvis is connected to your phone".to_string(),
        priority: Some(3),
        tags: vec!["white_check_mark".to_string()],
        click: None,
    };
    publish(client, cfg, &msg).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_string, header, headers, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn generate_topic_has_expected_prefix() {
        let topic = generate_topic();
        assert!(
            topic.starts_with("jarvis-"),
            "expected prefix 'jarvis-', got {topic}"
        );
    }

    #[test]
    fn generate_topic_has_expected_length() {
        let topic = generate_topic();
        // "jarvis-" (7) + 24 random chars = 31.
        assert_eq!(topic.len(), 31, "unexpected topic length: {topic}");
    }

    #[test]
    fn generate_topic_charset_is_ntfy_safe() {
        let topic = generate_topic();
        assert!(
            topic
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
            "topic contains unsafe characters: {topic}"
        );
        assert!(
            topic.chars().all(|c| !c.is_ascii_uppercase()),
            "topic should be lowercase: {topic}"
        );
    }

    #[test]
    fn generate_topic_is_unique_across_calls() {
        let a = generate_topic();
        let b = generate_topic();
        assert_ne!(a, b, "two generated topics should not collide");
    }

    #[test]
    fn subscribe_url_hosted() {
        let cfg = NtfyConfig::default_hosted("jarvis-abc123".to_string());
        assert_eq!(cfg.subscribe_url(), "https://ntfy.sh/jarvis-abc123");
    }

    #[test]
    fn app_deep_link_hosted() {
        let cfg = NtfyConfig::default_hosted("jarvis-abc123".to_string());
        assert_eq!(cfg.app_deep_link(), "ntfy://ntfy.sh/jarvis-abc123");
    }

    #[test]
    fn subscribe_url_self_hosted() {
        let cfg = NtfyConfig {
            base_url: "https://push.example.com".to_string(),
            topic: "jarvis-xyz789".to_string(),
        };
        assert_eq!(cfg.subscribe_url(), "https://push.example.com/jarvis-xyz789");
    }

    #[test]
    fn app_deep_link_self_hosted() {
        let cfg = NtfyConfig {
            base_url: "https://push.example.com".to_string(),
            topic: "jarvis-xyz789".to_string(),
        };
        assert_eq!(cfg.app_deep_link(), "ntfy://push.example.com/jarvis-xyz789");
    }

    #[tokio::test]
    async fn publish_sends_expected_request_and_succeeds_on_2xx() {
        let server = MockServer::start().await;
        let topic = "jarvis-test-topic";

        Mock::given(method("POST"))
            .and(path(format!("/{topic}")))
            .and(header("Title", "Hello"))
            .and(body_string("World"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let cfg = NtfyConfig {
            base_url: server.uri(),
            topic: topic.to_string(),
        };
        let msg = NtfyMessage {
            title: "Hello".to_string(),
            body: "World".to_string(),
            priority: None,
            tags: vec![],
            click: None,
        };

        let client = reqwest::Client::new();
        let result = publish(&client, &cfg, &msg).await;
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[tokio::test]
    async fn publish_sets_priority_tags_and_click_headers() {
        let server = MockServer::start().await;
        let topic = "jarvis-test-topic2";

        Mock::given(method("POST"))
            .and(path(format!("/{topic}")))
            .and(header("Priority", "5"))
            .and(headers("Tags", vec!["warning", "rotating_light"]))
            .and(header("Click", "https://example.com"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let cfg = NtfyConfig {
            base_url: server.uri(),
            topic: topic.to_string(),
        };
        let msg = NtfyMessage {
            title: "Alert".to_string(),
            body: "Something happened".to_string(),
            priority: Some(5),
            tags: vec!["warning".to_string(), "rotating_light".to_string()],
            click: Some("https://example.com".to_string()),
        };

        let client = reqwest::Client::new();
        let result = publish(&client, &cfg, &msg).await;
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }

    #[tokio::test]
    async fn publish_errors_on_4xx() {
        let server = MockServer::start().await;
        let topic = "jarvis-test-topic3";

        Mock::given(method("POST"))
            .and(path(format!("/{topic}")))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&server)
            .await;

        let cfg = NtfyConfig {
            base_url: server.uri(),
            topic: topic.to_string(),
        };
        let msg = NtfyMessage {
            title: "Hello".to_string(),
            body: "World".to_string(),
            priority: None,
            tags: vec![],
            click: None,
        };

        let client = reqwest::Client::new();
        let result = publish(&client, &cfg, &msg).await;
        assert!(result.is_err(), "expected Err, got {result:?}");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("403"), "error should mention status: {err_msg}");
        assert!(
            err_msg.contains("forbidden"),
            "error should include response body: {err_msg}"
        );
    }

    #[tokio::test]
    async fn send_test_publishes_canned_message() {
        let server = MockServer::start().await;
        let topic = "jarvis-test-topic4";

        Mock::given(method("POST"))
            .and(path(format!("/{topic}")))
            .and(header("Title", "Jarvis"))
            .and(header("Priority", "3"))
            .and(header("Tags", "white_check_mark"))
            .and(body_string("✅ Jarvis is connected to your phone"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let cfg = NtfyConfig {
            base_url: server.uri(),
            topic: topic.to_string(),
        };

        let client = reqwest::Client::new();
        let result = send_test(&client, &cfg).await;
        assert!(result.is_ok(), "expected Ok, got {result:?}");
    }
}
