//! Provider-agnostic chat/embedding types shared by the `ChatProvider` trait
//! and its implementations (e.g. `OllamaProvider`).

use serde::{Deserialize, Serialize};

/// Chat message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A single chat message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallData>>,
    /// Name of the tool this message is a result for (only relevant for
    /// `Role::Tool` messages).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_name: None,
        }
    }
}

/// Definition of a tool exposed to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDef {
    /// Serialize this tool definition into Ollama's wire format:
    /// `{"type":"function","function":{"name":..,"description":..,"parameters":..}}`
    pub fn to_wire(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// A tool call requested by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallData {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Wire-format wrapper Ollama uses for tool calls: `{"function":{"name":..,"arguments":..}}`.
#[derive(Debug, Clone, Deserialize)]
struct WireToolCall {
    function: WireToolCallFunction,
}

#[derive(Debug, Clone, Deserialize)]
struct WireToolCallFunction {
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

impl From<WireToolCall> for ToolCallData {
    fn from(w: WireToolCall) -> Self {
        Self {
            name: w.function.name,
            arguments: w.function.arguments,
        }
    }
}

/// Deserialize a list of tool calls from Ollama's wire format.
pub(crate) fn parse_wire_tool_calls(value: &serde_json::Value) -> Vec<ToolCallData> {
    match serde_json::from_value::<Vec<WireToolCall>>(value.clone()) {
        Ok(calls) => calls.into_iter().map(ToolCallData::from).collect(),
        Err(_) => Vec::new(),
    }
}

/// Options controlling a chat request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatOptions {
    pub model: String,
    /// Whether to enable "thinking" mode. MUST default to `false` — on this
    /// CPU-only hardware, thinking mode adds ~90s of latency per request.
    pub think: bool,
    pub temperature: Option<f32>,
    pub num_ctx: Option<u32>,
    pub tools: Vec<ToolDef>,
}

impl ChatOptions {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }
}

/// A streamed event produced while a chat completion is in progress.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChatEvent {
    /// Incremental assistant content.
    Token(String),
    /// The model requested one or more tool calls.
    ToolCalls(Vec<ToolCallData>),
    /// The stream has finished.
    Done { total_ms: Option<u64> },
}
