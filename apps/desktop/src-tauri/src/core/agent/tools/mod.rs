// WP-Agent-Tools owns this file.

//! Tool registry: the set of callable tools exposed to the model, plus the
//! plumbing to dispatch a model-issued `ToolCallData` to the right `Tool`
//! implementation and turn its result (or failure) into a string that can be
//! fed back into the conversation as a `Role::Tool` message.

pub mod vault;

use crate::core::agent::provider::{ChatProvider, ToolCallData, ToolDef};
use crate::core::db::Db;
use crate::core::memory::Vault;

/// Everything a tool needs for one execution. Borrowed for the call's duration.
pub struct ToolContext<'a> {
    pub db: &'a Db,
    pub vault: &'a Vault,
    pub provider: &'a dyn ChatProvider,
}

/// A single callable tool: its wire-format definition, plus the logic to run
/// it given a `ToolContext` and the model-supplied JSON arguments.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn def(&self) -> ToolDef;
    async fn execute(&self, ctx: &ToolContext<'_>, args: &serde_json::Value) -> anyhow::Result<String>;
}

/// Registry of all tools exposed to the model.
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Registry with the three built-in vault tools registered.
    pub fn with_defaults() -> Self {
        ToolRegistry {
            tools: vec![
                Box::new(vault::VaultSearch),
                Box::new(vault::VaultRead),
                Box::new(vault::VaultAppend),
            ],
        }
    }

    /// Definitions for every registered tool, in the wire-ready format the
    /// model expects.
    pub fn defs(&self) -> Vec<ToolDef> {
        self.tools.iter().map(|t| t.def()).collect()
    }

    /// Look up a registered tool by name.
    pub fn find(&self, name: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.def().name == name)
            .map(|t| t.as_ref())
    }

    /// Execute a model-issued tool call. Returns a result string to feed back
    /// as a `Role::Tool` message. On unknown tool name OR argument-validation
    /// OR execution error, returns a concise human-readable error string
    /// (prefixed "ERROR:") rather than panicking or bubbling the error up —
    /// this lets the agent loop retry with feedback.
    pub async fn execute(&self, ctx: &ToolContext<'_>, call: &ToolCallData) -> String {
        match self.find(&call.name) {
            Some(tool) => match tool.execute(ctx, &call.arguments).await {
                Ok(result) => result,
                Err(err) => format!("ERROR: {err:#}"),
            },
            None => format!("ERROR: unknown tool '{}'", call.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::provider::{ChatEvent, ChatMessage, ChatOptions, ProviderHealth};
    use futures_util::stream::BoxStream;

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

        async fn embed(&self, texts: Vec<String>) -> anyhow::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .map(|t| {
                    let seed = t.bytes().map(|b| b as f32).sum::<f32>();
                    (0..768).map(|i| seed + i as f32 * 0.001).collect()
                })
                .collect())
        }

        async fn health(&self) -> anyhow::Result<ProviderHealth> {
            anyhow::bail!("not implemented in stub")
        }
    }

    #[test]
    fn with_defaults_registers_exactly_three_tools() {
        let registry = ToolRegistry::with_defaults();
        let defs = registry.defs();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"vault_search"));
        assert!(names.contains(&"vault_read"));
        assert!(names.contains(&"vault_append"));

        assert!(registry.find("vault_search").is_some());
        assert!(registry.find("vault_read").is_some());
        assert!(registry.find("vault_append").is_some());
        assert!(registry.find("nonexistent").is_none());
    }

    #[tokio::test]
    async fn execute_on_unknown_tool_returns_error_string() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open_in_memory().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        let provider = StubProvider;
        let ctx = ToolContext {
            db: &db,
            vault: &vault,
            provider: &provider,
        };
        let registry = ToolRegistry::with_defaults();
        let call = ToolCallData {
            name: "nope".to_string(),
            arguments: serde_json::json!({}),
        };
        let result = registry.execute(&ctx, &call).await;
        assert!(result.starts_with("ERROR"), "got: {result}");
    }
}
