//! `ChatProvider` implementation backed by a local Ollama server.

use std::collections::VecDeque;

use futures_util::stream::{self, BoxStream, StreamExt};
use serde::Deserialize;
use serde_json::json;

use super::types::{parse_wire_tool_calls, ChatEvent, ChatMessage, ChatOptions};
use super::{ChatProvider, ProviderHealth};

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_EMBED_MODEL: &str = "nomic-embed-text";

/// A `ChatProvider` backed by a local Ollama server (default
/// `http://127.0.0.1:11434`).
pub struct OllamaProvider {
    base_url: String,
    embed_model: String,
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new provider pointed at `base_url` (e.g.
    /// `http://127.0.0.1:11434`). No trailing slash expected.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            embed_model: DEFAULT_EMBED_MODEL.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Override the model used for `embed()`. Defaults to `nomic-embed-text`.
    pub fn with_embed_model(mut self, model: impl Into<String>) -> Self {
        self.embed_model = model.into();
        self
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new(DEFAULT_BASE_URL)
    }
}

/// Build the JSON body for a `POST /api/chat` request. `think` is always
/// sent explicitly (defaults to `false` via `ChatOptions::default`) since
/// leaving Ollama's thinking mode on costs ~90s of extra latency on
/// CPU-only hardware.
fn build_chat_body(messages: &[ChatMessage], opts: &ChatOptions) -> serde_json::Value {
    let mut body = json!({
        "model": opts.model,
        "messages": messages,
        "stream": true,
        "think": opts.think,
    });

    let mut options = serde_json::Map::new();
    if let Some(temperature) = opts.temperature {
        options.insert("temperature".to_string(), json!(temperature));
    }
    if let Some(num_ctx) = opts.num_ctx {
        options.insert("num_ctx".to_string(), json!(num_ctx));
    }
    if !options.is_empty() {
        body["options"] = serde_json::Value::Object(options);
    }

    if !opts.tools.is_empty() {
        let tools: Vec<serde_json::Value> = opts.tools.iter().map(|t| t.to_wire()).collect();
        body["tools"] = serde_json::Value::Array(tools);
    }

    body
}

/// Parse a single NDJSON line from `/api/chat` into zero or more
/// `ChatEvent`s (a line can carry a content token, tool calls, and/or the
/// terminal `done` marker).
fn parse_chat_line(line: &str) -> anyhow::Result<Vec<ChatEvent>> {
    let value: serde_json::Value = serde_json::from_str(line)?;
    let mut events = Vec::new();

    if let Some(message) = value.get("message") {
        if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
            if !content.is_empty() {
                events.push(ChatEvent::Token(content.to_string()));
            }
        }
        if let Some(tool_calls) = message.get("tool_calls") {
            if tool_calls.as_array().is_some_and(|a| !a.is_empty()) {
                let calls = parse_wire_tool_calls(tool_calls);
                if !calls.is_empty() {
                    events.push(ChatEvent::ToolCalls(calls));
                }
            }
        }
    }

    if value.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
        let total_ms = value
            .get("total_duration")
            .and_then(|d| d.as_u64())
            .map(|ns| ns / 1_000_000);
        events.push(ChatEvent::Done { total_ms });
    }

    Ok(events)
}

/// Internal state threaded through `stream::unfold` while consuming the
/// chunked NDJSON response body.
struct LineStreamState {
    body: BoxStream<'static, reqwest::Result<String>>,
    /// Bytes received so far that haven't formed a complete `\n`-terminated
    /// line yet. A JSON object can be split across multiple TCP chunks, and
    /// a single chunk can contain multiple JSON objects, so we buffer raw
    /// text and only parse once we've found a newline.
    buffer: String,
    /// Events parsed from a line but not yet yielded (a single line can
    /// produce more than one `ChatEvent`).
    pending: VecDeque<ChatEvent>,
    finished: bool,
}

fn drain_complete_lines(state: &mut LineStreamState) {
    while let Some(pos) = state.buffer.find('\n') {
        let line: String = state.buffer.drain(..=pos).collect();
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match parse_chat_line(line) {
            Ok(events) => state.pending.extend(events),
            Err(err) => {
                tracing::warn!(%err, %line, "failed to parse Ollama NDJSON line");
            }
        }
    }
}

async fn next_chat_event(
    mut state: LineStreamState,
) -> Option<(anyhow::Result<ChatEvent>, LineStreamState)> {
    loop {
        if let Some(event) = state.pending.pop_front() {
            return Some((Ok(event), state));
        }
        if state.finished {
            return None;
        }
        match state.body.next().await {
            Some(Ok(chunk)) => {
                state.buffer.push_str(&chunk);
                drain_complete_lines(&mut state);
            }
            Some(Err(err)) => {
                state.finished = true;
                return Some((Err(err.into()), state));
            }
            None => {
                state.finished = true;
                // Flush any trailing partial line without a final newline.
                if !state.buffer.trim().is_empty() {
                    let remainder = std::mem::take(&mut state.buffer);
                    match parse_chat_line(remainder.trim()) {
                        Ok(events) => state.pending.extend(events),
                        Err(err) => {
                            tracing::warn!(%err, "failed to parse trailing Ollama NDJSON line");
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Debug, Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Debug, Deserialize)]
struct TagModel {
    name: String,
}

#[async_trait::async_trait]
impl ChatProvider for OllamaProvider {
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
        let body = build_chat_body(&messages, &opts);
        let url = format!("{}/api/chat", self.base_url);
        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;

        let byte_stream = resp
            .bytes_stream()
            .map(|chunk| chunk.map(|b| String::from_utf8_lossy(&b).into_owned()));

        let state = LineStreamState {
            body: Box::pin(byte_stream),
            buffer: String::new(),
            pending: VecDeque::new(),
            finished: false,
        };

        let stream = stream::unfold(state, next_chat_event);
        Ok(Box::pin(stream))
    }

    async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
        let url = format!("{}/api/embed", self.base_url);
        let body = json!({
            "model": self.embed_model,
            "input": texts,
        });
        let resp = self
            .client
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let parsed: EmbedResponse = resp.json().await?;
        Ok(parsed.embeddings)
    }

    async fn health(&self) -> anyhow::Result<ProviderHealth> {
        let version_url = format!("{}/api/version", self.base_url);
        let tags_url = format!("{}/api/tags", self.base_url);

        let version: VersionResponse = self
            .client
            .get(version_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let tags: TagsResponse = self
            .client
            .get(tags_url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(ProviderHealth {
            version: version.version,
            models: tags.models.into_iter().map(|m| m.name).collect(),
        })
    }
}

/// Pick a sensible default chat model from the set of models available
/// locally, given the machine's total RAM in GB.
///
/// - Prefers `qwen3:8b` when present and the machine has at least 24GB RAM.
/// - Falls back to `qwen3:4b` when present.
/// - Otherwise falls back to the first available model.
/// - If nothing is available, falls back to the literal `"qwen3:4b"`.
pub fn pick_default_model(available: &[String], ram_gb: u32) -> String {
    if ram_gb >= 24 && available.iter().any(|m| m == "qwen3:8b") {
        return "qwen3:8b".to_string();
    }
    if available.iter().any(|m| m == "qwen3:4b") {
        return "qwen3:4b".to_string();
    }
    if let Some(first) = available.first() {
        return first.clone();
    }
    "qwen3:4b".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::types::ToolDef;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn events_from_ndjson(body: &str) -> Vec<ChatEvent> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(body.as_bytes().to_vec(), "application/x-ndjson"),
            )
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let messages = vec![ChatMessage::user("hi")];
        let opts = ChatOptions::new("qwen3:4b");
        let stream = provider.chat_stream(messages, opts).await.unwrap();
        stream
            .map(|r| r.expect("chat event"))
            .collect::<Vec<_>>()
            .await
    }

    #[tokio::test]
    async fn chat_stream_parses_tokens_across_multiple_lines() {
        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\"lo\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true,\"total_duration\":5000000}\n",
        );
        let events = events_from_ndjson(body).await;
        assert_eq!(
            events,
            vec![
                ChatEvent::Token("Hel".to_string()),
                ChatEvent::Token("lo".to_string()),
                ChatEvent::Done { total_ms: Some(5) },
            ]
        );
    }

    #[tokio::test]
    async fn chat_stream_parses_tool_calls() {
        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"\",\"tool_calls\":",
            "[{\"function\":{\"name\":\"get_weather\",\"arguments\":{\"city\":\"NYC\"}}}]},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\"\"},\"done\":true}\n",
        );
        let events = events_from_ndjson(body).await;
        assert_eq!(events.len(), 2);
        match &events[0] {
            ChatEvent::ToolCalls(calls) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "get_weather");
                assert_eq!(calls[0].arguments, json!({"city": "NYC"}));
            }
            other => panic!("expected ToolCalls, got {other:?}"),
        }
        assert_eq!(events[1], ChatEvent::Done { total_ms: None });
    }

    #[tokio::test]
    async fn chat_stream_handles_two_objects_concatenated_in_one_chunk() {
        // Two full NDJSON lines served as a single body/chunk: proves the
        // line-buffer splits on `\n` even when everything arrives at once.
        let body = concat!(
            "{\"message\":{\"role\":\"assistant\",\"content\":\"A\"},\"done\":false}\n",
            "{\"message\":{\"role\":\"assistant\",\"content\":\"B\"},\"done\":true,\"total_duration\":2000000}\n",
        );
        let events = events_from_ndjson(body).await;
        assert_eq!(
            events,
            vec![
                ChatEvent::Token("A".to_string()),
                ChatEvent::Token("B".to_string()),
                ChatEvent::Done { total_ms: Some(2) },
            ]
        );
    }

    #[tokio::test]
    async fn chat_stream_handles_a_line_split_mid_json() {
        // wiremock serves the whole body as a single response, but the
        // parser must be robust to a line arriving as multiple chunks.
        // Simulate this by feeding the state machine directly with two
        // pushes of a single logical line split mid-JSON.
        let mut state = LineStreamState {
            body: Box::pin(stream::empty()),
            buffer: String::new(),
            pending: VecDeque::new(),
            finished: false,
        };
        state.buffer.push_str("{\"message\":{\"role\":\"assistant\",\"con");
        drain_complete_lines(&mut state);
        assert!(state.pending.is_empty(), "no complete line yet");

        state
            .buffer
            .push_str("tent\":\"hi\"},\"done\":true,\"total_duration\":1000000}\n");
        drain_complete_lines(&mut state);

        let events: Vec<_> = state.pending.into_iter().collect();
        assert_eq!(
            events,
            vec![
                ChatEvent::Token("hi".to_string()),
                ChatEvent::Done { total_ms: Some(1) },
            ]
        );
    }

    #[tokio::test]
    async fn embed_returns_vectors() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "embeddings": [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let result = provider
            .embed(vec!["a".to_string(), "b".to_string()])
            .await
            .unwrap();
        assert_eq!(result, vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6]]);
    }

    #[tokio::test]
    async fn health_combines_version_and_tags() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"version": "0.5.1"})))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/tags"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{"name": "qwen3:4b"}, {"name": "nomic-embed-text"}]
            })))
            .mount(&server)
            .await;

        let provider = OllamaProvider::new(server.uri());
        let health = provider.health().await.unwrap();
        assert_eq!(health.version, "0.5.1");
        assert_eq!(
            health.models,
            vec!["qwen3:4b".to_string(), "nomic-embed-text".to_string()]
        );
    }

    #[test]
    fn pick_default_model_table() {
        let both = vec!["qwen3:8b".to_string(), "qwen3:4b".to_string()];
        assert_eq!(pick_default_model(&both, 32), "qwen3:8b");
        assert_eq!(pick_default_model(&both, 16), "qwen3:4b");

        let only_4b = vec!["qwen3:4b".to_string()];
        assert_eq!(pick_default_model(&only_4b, 32), "qwen3:4b");

        let empty: Vec<String> = vec![];
        assert_eq!(pick_default_model(&empty, 32), "qwen3:4b");

        let other_only = vec!["llama3:8b".to_string()];
        assert_eq!(pick_default_model(&other_only, 8), "llama3:8b");
    }

    #[test]
    fn chat_body_sends_think_false_by_default() {
        let messages = vec![ChatMessage::user("hello")];
        let opts = ChatOptions::new("qwen3:4b");
        let body = build_chat_body(&messages, &opts);
        assert_eq!(body["think"], json!(false));
        assert_eq!(body["stream"], json!(true));
        assert_eq!(body["model"], json!("qwen3:4b"));
        assert!(body.get("tools").is_none());
    }

    #[test]
    fn chat_body_includes_tools_when_present() {
        let messages = vec![ChatMessage::user("hello")];
        let mut opts = ChatOptions::new("qwen3:4b");
        opts.tools.push(ToolDef {
            name: "get_weather".to_string(),
            description: "Get the weather".to_string(),
            parameters: json!({"type": "object", "properties": {}}),
        });
        let body = build_chat_body(&messages, &opts);
        assert_eq!(body["tools"][0]["type"], json!("function"));
        assert_eq!(body["tools"][0]["function"]["name"], json!("get_weather"));
    }

    /// Live smoke test against a real local Ollama server. Not run by
    /// default — normal `cargo test` must not depend on a live server.
    /// Run manually with:
    ///   cargo test --lib core::agent::provider -- --ignored
    #[tokio::test]
    #[ignore]
    async fn live_smoke_test_says_hi() {
        let provider = OllamaProvider::default();
        let messages = vec![ChatMessage::user("Say hi in one word")];
        let opts = ChatOptions::new("qwen3:4b");
        let mut stream = provider.chat_stream(messages, opts).await.unwrap();
        let mut got_token = false;
        while let Some(event) = stream.next().await {
            if let ChatEvent::Token(_) = event.unwrap() {
                got_token = true;
                break;
            }
        }
        assert!(got_token, "expected at least one Token event");
    }
}
