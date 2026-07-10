//! The agent loop: one user turn → retrieval-grounded, tool-capable, streamed
//! answer. Chat, the command palette, voice, and the scheduler all funnel
//! through here, so tool execution — and its Quiet Feed audit — happens in
//! exactly one place.

use futures_util::StreamExt;
use serde::Serialize;

use crate::core::agent::prompts::{self, PromptContext};
use crate::core::agent::provider::types::Role;
use crate::core::agent::provider::{
    ChatEvent, ChatMessage, ChatOptions, ChatProvider, ToolCallData,
};
use crate::core::agent::tools::{ToolContext, ToolRegistry};
use crate::core::db::queries::search::SearchHit;
use crate::core::db::Db;
use crate::core::memory::{core_memory, retriever, Vault};

/// How many vault chunks to inject as grounding context.
const RETRIEVE_K: usize = 6;
/// Safety cap on tool-call rounds before we force a final, tool-free answer.
const MAX_TOOL_ROUNDS: usize = 4;

/// A cited source backing an answer, surfaced to the UI as a citation chip.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Citation {
    pub index: usize,
    pub source_path: String,
    pub heading: Option<String>,
}

impl Citation {
    fn from_hits(hits: &[SearchHit]) -> Vec<Citation> {
        hits.iter()
            .enumerate()
            .map(|(i, h)| Citation {
                index: i + 1,
                source_path: h.source_path.clone(),
                heading: h.heading.clone(),
            })
            .collect()
    }
}

/// Streamed events from a single agent turn, forwarded to the frontend.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Citations available up front (from retrieval), before tokens stream.
    Citations { citations: Vec<Citation> },
    /// Incremental answer text.
    Token { text: String },
    /// A tool is about to run.
    ToolCall { name: String },
    /// A tool finished (`ok = false` means it returned an error the model sees).
    ToolResult { name: String, ok: bool },
    /// The turn finished successfully.
    Done,
    /// The turn failed.
    Error { message: String },
}

/// Immutable context for a turn: the core services plus per-request facts.
pub struct AgentContext<'a> {
    pub db: &'a Db,
    pub vault: &'a Vault,
    pub provider: &'a dyn ChatProvider,
    pub registry: &'a ToolRegistry,
    pub model: String,
    /// Human-readable current date, e.g. `"2026-07-09 (Thursday)"`.
    pub date: String,
    pub quiet_hours: bool,
}

/// Final result of a turn (also delivered incrementally via `on_event`).
#[derive(Debug, Clone, PartialEq)]
pub struct AgentOutcome {
    pub answer: String,
    pub citations: Vec<Citation>,
}

/// Run one full agent turn. Retrieval grounds the system prompt, the model
/// streams an answer, and any tool calls are executed (looping their results
/// back) up to [`MAX_TOOL_ROUNDS`]. `on_event` receives streamed events; the
/// final answer is also returned.
pub async fn run_turn(
    ctx: &AgentContext<'_>,
    user_message: &str,
    mut on_event: impl FnMut(AgentEvent),
) -> anyhow::Result<AgentOutcome> {
    // 1. Retrieve grounding context. Degrade gracefully: a retrieval failure
    //    (e.g. embeddings model not pulled yet) should not sink the whole turn.
    let hits = retriever::retrieve(ctx.db, ctx.provider, user_message, RETRIEVE_K)
        .await
        .unwrap_or_default();
    let citations = Citation::from_hits(&hits);
    if !citations.is_empty() {
        on_event(AgentEvent::Citations {
            citations: citations.clone(),
        });
    }

    // 2. Assemble the system prompt from base instruction + dynamic context.
    let core_mem = core_memory::load_and_render(ctx.db).unwrap_or_default();
    let retrieved = retriever::render_context(&hits);
    let system = prompts::build_system_prompt(&PromptContext {
        date: &ctx.date,
        core_memory: &core_mem,
        retrieved: &retrieved,
        quiet_hours: ctx.quiet_hours,
    });

    // 3. Seed the conversation and run the tool-calling loop.
    let mut messages = vec![ChatMessage::system(system), ChatMessage::user(user_message)];
    let base_opts = ChatOptions {
        model: ctx.model.clone(),
        think: false,
        temperature: None,
        num_ctx: None,
        tools: ctx.registry.defs(),
    };
    let tool_ctx = ToolContext {
        db: ctx.db,
        vault: ctx.vault,
        provider: ctx.provider,
    };

    let mut final_answer = String::new();

    for round in 0..MAX_TOOL_ROUNDS {
        // On the last permitted round, withhold tools so the model is forced to
        // produce a natural-language answer instead of calling forever.
        let mut round_opts = base_opts.clone();
        if round + 1 == MAX_TOOL_ROUNDS {
            round_opts.tools = Vec::new();
        }

        let mut stream = ctx.provider.chat_stream(messages.clone(), round_opts).await?;
        let mut round_content = String::new();
        let mut tool_calls: Vec<ToolCallData> = Vec::new();

        while let Some(ev) = stream.next().await {
            match ev? {
                ChatEvent::Token(t) => {
                    round_content.push_str(&t);
                    on_event(AgentEvent::Token { text: t });
                }
                ChatEvent::ToolCalls(calls) => tool_calls.extend(calls),
                ChatEvent::Done { .. } => break,
            }
        }

        if tool_calls.is_empty() {
            final_answer = round_content;
            break;
        }

        // Echo the assistant turn (content + tool_calls), then feed each result
        // back as a tool message for the next round.
        messages.push(ChatMessage {
            role: Role::Assistant,
            content: round_content,
            tool_calls: Some(tool_calls.clone()),
            tool_name: None,
        });
        for call in &tool_calls {
            on_event(AgentEvent::ToolCall {
                name: call.name.clone(),
            });
            let result = ctx.registry.execute(&tool_ctx, call).await;
            let ok = !result.starts_with("ERROR");
            on_event(AgentEvent::ToolResult {
                name: call.name.clone(),
                ok,
            });
            messages.push(ChatMessage {
                role: Role::Tool,
                content: result,
                tool_calls: None,
                tool_name: Some(call.name.clone()),
            });
        }
    }

    on_event(AgentEvent::Done);
    Ok(AgentOutcome {
        answer: final_answer,
        citations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::ProviderHealth;
    use futures_util::stream::{self, BoxStream};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn embed_one(text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; 768];
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();
        for (i, slot) in v.iter_mut().take(8).enumerate() {
            *slot = ((seed.rotate_left((i * 8) as u32) & 0xFFFF) as f32) / 65535.0;
        }
        v
    }

    /// A provider that first requests a `vault_search` tool call, then answers.
    struct ScriptedProvider {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl ChatProvider for ScriptedProvider {
        async fn chat_stream(
            &self,
            _messages: Vec<ChatMessage>,
            _opts: ChatOptions,
        ) -> anyhow::Result<BoxStream<'static, anyhow::Result<ChatEvent>>> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            let events: Vec<anyhow::Result<ChatEvent>> = if n == 0 {
                vec![
                    Ok(ChatEvent::ToolCalls(vec![ToolCallData {
                        name: "vault_search".to_string(),
                        arguments: serde_json::json!({ "query": "jarvis" }),
                    }])),
                    Ok(ChatEvent::Done { total_ms: None }),
                ]
            } else {
                vec![
                    Ok(ChatEvent::Token("Jarvis is local-first [1]".to_string())),
                    Ok(ChatEvent::Done { total_ms: None }),
                ]
            };
            Ok(Box::pin(stream::iter(events)))
        }

        async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|t| embed_one(t)).collect())
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("unused in tests")
        }
    }

    #[tokio::test]
    async fn run_turn_retrieves_calls_tool_and_streams_answer() {
        let db = Db::open_in_memory().unwrap();
        let provider = ScriptedProvider {
            calls: AtomicUsize::new(0),
        };
        // Seed one chunk so retrieval yields a citation.
        let rowid = db
            .upsert_chunk(
                "c1",
                "Knowledge/welcome.md",
                Some("Welcome"),
                "Jarvis is a local-first personal assistant.",
                "2026-01-01T00:00:00Z",
            )
            .unwrap();
        db.set_chunk_embedding(rowid, &embed_one("Jarvis is a local-first personal assistant."))
            .unwrap();

        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let registry = ToolRegistry::with_defaults();

        let ctx = AgentContext {
            db: &db,
            vault: &vault,
            provider: &provider,
            registry: &registry,
            model: "test-model".to_string(),
            date: "2026-07-09 (Thursday)".to_string(),
            quiet_hours: false,
        };

        let mut events = Vec::new();
        let outcome = run_turn(&ctx, "what is jarvis?", |e| events.push(e))
            .await
            .unwrap();

        assert!(outcome.answer.contains("Jarvis is local-first"));
        assert!(!outcome.citations.is_empty());
        assert_eq!(outcome.citations[0].source_path, "Knowledge/welcome.md");

        // A tool round happened and the final Done fired.
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::ToolCall { name } if name == "vault_search")));
        assert!(events
            .iter()
            .any(|e| matches!(e, AgentEvent::Token { .. })));
        assert_eq!(events.last(), Some(&AgentEvent::Done));
    }
}
