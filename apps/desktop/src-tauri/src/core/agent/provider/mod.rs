//! LLM provider abstraction. `ChatProvider` is the trait the rest of the
//! agent loop programs against; `OllamaProvider` is the local-first
//! implementation backed by a local Ollama server.

pub mod ollama;
pub mod types;

pub use ollama::{pick_default_model, OllamaProvider};
pub use types::{ChatEvent, ChatMessage, ChatOptions, ToolCallData, ToolDef};

use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};

/// Health/status information about a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub version: String,
    pub models: Vec<String>,
}

/// A chat-capable LLM provider.
#[async_trait::async_trait]
pub trait ChatProvider: Send + Sync {
    /// Stream a chat completion for the given conversation.
    async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        opts: ChatOptions,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>>;

    /// Compute embeddings for a batch of texts.
    async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>>;

    /// Check provider availability and enumerate available models.
    async fn health(&self) -> anyhow::Result<ProviderHealth>;
}
