# Jarvis — Agent Conventions

Local-first personal life assistant. Tauri v2 (Rust core) + React. The Rust core owns all
state, tool execution, scheduling, and network I/O; the React frontend is a thin reactive view.

Workspace: `apps/desktop` (frontend) + `apps/desktop/src-tauri` (Rust core).

## Invariants (never violate — reject changes that break these)

1. **The vault is the source of truth.** Everything Jarvis knows lives as plain markdown in
   an Obsidian-compatible folder the user owns. SQLite (`core/db`) is only an index/cache —
   deleting it must lose nothing.
2. **No silent actions.** Every *mutating* tool call writes a Quiet Feed row
   (`QuietFeedItem` via `db.insert_feed`) at the point of execution. See
   `core/agent/tools/vault.rs::VaultAppend` for the canonical pattern.
3. **Tools never execute in the frontend.** All tool/side-effect logic lives in Rust. The
   frontend only invokes commands and renders events.
4. **All notifications pass the quiet-hours chokepoint.** No code path sends an ntfy/native
   notification without going through the single quiet-hours gate (`core/scheduler/quiet_hours.rs`).
5. **Local-first.** LLM, embeddings, STT/TTS, and search run on-device. The only outbound
   network calls are the user's own connected accounts (Google, ntfy, etc.).
6. **Secrets only in the OS credential store.** OAuth tokens and client secrets live in
   Windows Credential Manager via the `keyring` crate (`core/google/token_store.rs`) — never
   in SQLite plaintext, never in the vault.

## Architecture map

- `core/agent/` — one agent loop (`agent_loop.rs`) for chat, palette, voice, scheduler
  triggers; tool-calling over Ollama. `tools/` is the tool registry; `provider/` the LLM
  provider; `prompts/` the system-prompt builder.
- `core/memory/` — markdown vault (source of truth) → chunk → embed → SQLite (FTS5 +
  sqlite-vec) hybrid retrieval; core-memory block.
- `core/db/` — SQLite schema + typed queries per domain (`queries/`).
- `core/google/` — OAuth (PKCE + loopback, `oauth.rs`), token store (`token_store.rs`),
  typed API clients (`calendar_client.rs`, ...).
- `core/notify/` — ntfy push + native toast fallback.
- `core/scheduler/` — cron jobs (briefings, nudges, sync) + quiet-hours chokepoint.
- `commands/` — thin `#[tauri::command]` wrappers; no business logic. Register handlers in
  `lib.rs::invoke_handler!`.
- `src/` (frontend) — feature folders under `src/features/`; typed IPC in `src/lib/ipc.ts`;
  per-feature `use*.ts` hooks.

## Patterns to copy, not reinvent

- OAuth engine: `core/google/oauth.rs`
- API client + `wiremock` tests: `core/google/calendar_client.rs`
- Mutating tool + Quiet Feed audit: `core/agent/tools/vault.rs`
- Typed IPC wrapper: `src/lib/ipc.ts`
- Feature hook: `src/features/*/use*.ts`
- Secrets: `core/google/token_store.rs`

## Adding an agent tool

1. New struct in `core/agent/tools/<domain>.rs` implementing `Tool` (`def()` + `execute()`).
2. If mutating, write a `QuietFeedItem` in `execute()`.
3. Register it in `core/agent/tools/mod.rs::ToolRegistry::with_defaults()`.
4. Add unit tests using the in-file `StubProvider`/`Db::open_in_memory()` pattern.

## Verification

- Rust: `cd apps/desktop/src-tauri && cargo test`.
- Frontend: `cd apps/desktop && npm run build` (tsc typecheck + Vite).
- End-to-end: `cd apps/desktop && npm run tauri dev`, then drive the real app.
- Prereqs: Node, Rust, WebView2 (present on Win11), Ollama (onboarding self-installs).

## Platform note

Currently Windows-only in practice (winget, `LOCALAPPDATA`, Credential Manager,
`cmd /C start`). Keep a `#[cfg(not(windows))]` best-effort branch where cheap, but Windows
is the supported target.
