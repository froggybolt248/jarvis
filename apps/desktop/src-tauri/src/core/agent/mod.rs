// Owned by WP-Provider (provider/) and later the agent loop.
pub mod provider;

// M3 (WP-Agent-Tools): tool registry + built-in tools, and prompt assembly.
pub mod prompts;
pub mod tools;

// M3 orchestrator: the retrieval-grounded, tool-calling agent loop.
pub mod agent_loop;
