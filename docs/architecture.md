# Jarvis Architecture

Jarvis is a Tauri v2 desktop app. The Rust core owns all state, tool execution,
scheduling, and network I/O; the React frontend is a thin reactive view.

## Layers

```
React UI (bento dashboard, command palette, voice HUD)
    │  invoke() / events
Rust core
    ├─ agent/      one agent loop for chat, palette, voice, and scheduler triggers;
    │              tool-calling against Ollama with strict JSON-schema validation
    ├─ memory/     markdown vault (source of truth) → chunk → embed (Ollama) →
    │              SQLite (FTS5 + sqlite-vec) hybrid retrieval; core-memory block
    ├─ scheduler/  cron jobs (briefings, nudges, missed-event watch) — every
    │              notification passes through quiet-hours batching, no bypass
    ├─ google/     Calendar API v3, per-user OAuth desktop client (PKCE + loopback)
    ├─ voice/      whisper.cpp + Piper sidecars, push-to-talk global hotkey
    └─ notify/     ntfy phone push + native toast fallback
```

## Principles

1. **The vault is the source of truth.** Everything Jarvis knows lives as plain
   markdown in an Obsidian-compatible folder the user owns. SQLite is only an
   index and cache — deleting it loses nothing.
2. **No silent actions.** Every mutating tool call writes a Quiet Feed row at
   the point of execution. Tools never execute in the frontend.
3. **Local-first.** LLM, STT, TTS, embeddings, and search run on-device. The
   only network calls are Google Calendar (the user's own OAuth client) and
   ntfy (the user's own private topic, self-hostable).
4. **Calm proactivity.** Non-urgent nudges are batched; quiet hours and study
   blocks are enforced in one chokepoint (`scheduler/quiet_hours.rs`).

Decision records live in `docs/adr/`.
