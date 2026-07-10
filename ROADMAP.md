# Jarvis Roadmap

Status legend: ✅ done · 🔨 in progress · ⬜ planned · 🚫 deferred/skipped

## Shipped

- **M0 — Shell & design system** ✅ App shell, tokens, icons, command palette.
- **M1 — Core infra** ✅ SQLite + sqlite-vec + FTS5, markdown vault, Ollama provider.
- **M2 — Foundation** ✅ Managed `AppState`, commands bridge, Ollama setup automation,
  Google OAuth + Calendar client, ntfy push, 4-step onboarding wizard + boot gate.
- **M3 — Agent & memory** ✅ Agent loop (Ollama tool-calling, streamed events), RAG memory
  pipeline (chunker/embedder/retriever/core-memory), chat UI, per-domain **read** commands
  (diet/gym/study/calendar/knowledge). _Note: read-only — write paths land in M4._

## In progress

### M4 — Finish the core (make what exists actually work) 🔨
The M3 dashboards render but can't show real data yet. M4 closes that.
- **Calendar sync** — `core/google/sync.rs` incremental sync (wires existing `list_events`
  + `upsert_event`/`set_sync_token`); `calendar_sync_now` command; `create_calendar_event`
  command + agent tool.
  - _Accept:_ after Google connect, the Calendar screen and Today tile show real events;
    the agent can create an event (with a Quiet Feed row).
- **Diet/gym/study write paths** — agent tools `log_meal`, `set_diet_targets`,
  `log_workout`, `create_study_card`, `review_study_card` (SM-2); thin quick-add forms.
  - _Accept:_ logging a meal/workout/card via chat or form makes the corresponding tile and
    screen show it; every write leaves a Quiet Feed row.
- **Scheduler + quiet hours** — `core/scheduler/` (tokio-cron-scheduler): calendar sync
  (15 min), morning briefing, missed-event watch, nudge batching; `quiet_hours.rs` single
  notification chokepoint replacing the hardcoded `quiet_hours: false`.
  - _Accept:_ a morning briefing arrives via ntfy; notifications inside quiet hours are
    suppressed/batched; nothing bypasses the chokepoint.
- **Exit criteria:** every dashboard tile shows real data; briefing arrives daily;
  `cargo test` green.

## Planned

### M5 — Connections Hub + Gmail ⬜
- **Connections Hub** — `core/connections/` with three auth rails (OAuth PKCE + loopback,
  token paste, QR/phone-code); `src/features/connections/` provider-card grid; connect any
  account in <30s; `list_connections` agent tool. Migrate Google + ntfy into it.
- **Gmail (flagship)** — `core/google/gmail_client.rs`; importance triage → ntfy (via
  chokepoint) + Quiet Feed; propose-then-approve cleanup (archive/label/unsubscribe, no
  silent deletes); daily digest in the briefing. Tools: `search_email`, `summarize_thread`,
  `archive_emails`, `label_emails`, `unsubscribe_sender`, `draft_reply` (draft only).
- **Telegram (flagship messaging)** — TDLib user account, QR login; unified triage +
  unread digest. _Crate choice after a spike._
- **Later in M5:** Discord, Slack, Notion, Todoist, Spotify, YouTube.
- **Exit criteria:** Gmail connects in <30s; an important email pushes to phone within
  15 min; a cleanup batch archives ≥50 promos with one approval; Telegram unread digest in
  the briefing.

### M6a — Project Jumpstart ⬜
Idea → interview → scaffold folder (`CLAUDE.md` + `.claude/kickoff.md`) → launch headless
Claude Code (`claude -p --bare --model … --append-system-prompt-file …`, streamed into
Jarvis) with model recommendation (Sonnet 5 / Opus 4.8) → resumable in Windows Terminal.

### M6b — Life-boost layer ⬜
Richer briefings (conflicts, travel time, email/Telegram digests, streaks); weekly
`suggest_improvements` review written to the vault; propose-then-approve auto time-blocking;
batched nudges.

## Deferred / skipped

- 🚫 **WhatsApp** — no safe personal-account path (unofficial libs = ToS violation + ban
  risk; official Business API can't read a personal inbox). Revisit only if Meta opens a
  personal API.
- 🚫 **Instagram** — API is Business/Creator-only, DMs reply-only; drafting-only scope not
  worth the account conversion.
- 🚫 **Voice layer** (whisper.cpp + Piper) — deferred to a later milestone.
- 🚫 **X/Twitter** — no free tier since Feb 2026; add later behind a user-supplied key.
- 🚫 **Banking/Plaid** — highest sensitivity + real cost; its own milestone if ever.
