// Typed bridge to the Rust core. Every `invoke` call in the app goes through
// here so command names and payload shapes live in exactly one place and stay
// in sync with `src-tauri/src/commands`.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface ProviderHealth {
  version: string;
  models: string[];
}

export interface QuietFeedItem {
  id: string;
  created_at: string;
  kind: string;
  title: string;
  body: string | null;
  deep_link: string | null;
  source: string | null;
}

/** First-run onboarding progress, mirrors `core::setup::onboarding`. */
export interface OnboardingState {
  complete: boolean;
  domains: string[];
}

/** Result of probing the local Ollama install/server. */
export interface OllamaStatus {
  installed: boolean;
  server_running: boolean;
  version: string | null;
  models: string[];
}

/** One model-pull progress update, emitted on the `ollama:pull-progress` event. */
export interface PullProgress {
  model: string;
  status: string;
  completed: number;
  total: number;
  percent: number;
}

/** Machine RAM + the model set recommended for it. */
export interface RecommendedModels {
  ram_gb: number;
  models: string[];
}

export interface CalendarListEntry {
  id: string;
  summary: string | null;
  primary: boolean | null;
}

/** Persisted ntfy phone-push configuration. */
export interface NtfyConfig {
  base_url: string;
  topic: string;
}

// ── Agent chat ──────────────────────────────────────────────────────────

/** A cited source backing an answer, mirrors `agent_loop::Citation`. */
export interface Citation {
  index: number;
  source_path: string;
  heading: string | null;
}

/**
 * Streamed events from a single agent turn, emitted one-by-one on the
 * `"agent:event"` window event. Mirrors `agent_loop::AgentEvent`
 * (`#[serde(tag = "type", rename_all = "snake_case")]`).
 */
export type AgentEvent =
  | { type: "citations"; citations: Citation[] }
  | { type: "token"; text: string }
  | { type: "tool_call"; name: string }
  | { type: "tool_result"; name: string; ok: boolean }
  | { type: "done" }
  | { type: "error"; message: string };

/** Diet log entry, mirrors `queries::diet::DietLog`. */
export interface DietLog {
  id: string;
  logged_at: string;
  description: string;
  calories: number | null;
  protein_g: number | null;
  carbs_g: number | null;
  fat_g: number | null;
  confidence: number | null;
}

/** Diet targets, mirrors `queries::diet::DietTargets`. */
export interface DietTargets {
  id: string;
  effective_date: string;
  calories: number | null;
  protein_g: number | null;
  carbs_g: number | null;
  fat_g: number | null;
  created_at: string;
}

/** Gym session, mirrors `queries::gym::GymSession`. */
export interface GymSession {
  id: string;
  program_id: string | null;
  started_at: string;
  ended_at: string | null;
  notes: string | null;
}

/** Gym set, mirrors `queries::gym::GymSet`. */
export interface GymSet {
  id: string;
  session_id: string;
  exercise: string;
  weight: number | null;
  reps: number | null;
  rpe: number | null;
  set_index: number | null;
}

/** Spaced-repetition study card, mirrors `queries::study::SrsCard`. */
export interface SrsCard {
  id: string;
  course_id: string | null;
  front: string;
  back: string;
  ease_factor: number;
  interval_days: number;
  repetitions: number;
  due_at: string;
  created_at: string;
}

/** Calendar event, mirrors `queries::calendar::CalendarEvent`. */
export interface CalendarEvent {
  id: string;
  google_id: string | null;
  calendar_id: string | null;
  summary: string | null;
  description: string | null;
  location: string | null;
  start_at: string | null;
  end_at: string | null;
  all_day: boolean;
  status: string | null;
  updated_at: string;
}

/** Vault note summary, mirrors `core::memory::NoteSummary`. */
export interface NoteSummary {
  path: string;
  title: string;
  modified: string;
}

export const ipc = {
  /** Ollama server version + locally installed models. Rejects if unreachable. */
  ollamaHealth: () => invoke<ProviderHealth>("ollama_health"),

  /** Read a persisted setting, or `null` if unset. */
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),

  /** Upsert a persisted setting. */
  setSetting: (key: string, value: string) =>
    invoke<void>("set_setting", { key, value }),

  /** Most recent Quiet Feed items, newest first. */
  recentFeed: (limit: number) => invoke<QuietFeedItem[]>("recent_feed", { limit }),

  // ── Onboarding + vault ────────────────────────────────────────────────
  /** Whether onboarding is complete + which domains are enabled. */
  getOnboardingState: () => invoke<OnboardingState>("get_onboarding_state"),

  /** Persist the domain selection without finishing onboarding. */
  setOnboardingDomains: (domains: string[]) =>
    invoke<void>("set_onboarding_domains", { domains }),

  /** Finalize onboarding: marks complete and enables autostart. */
  completeOnboarding: (domains: string[]) =>
    invoke<void>("complete_onboarding", { domains }),

  /** Default vault path (`%USERPROFILE%/JarvisVault`). */
  getDefaultVaultDir: () => invoke<string>("get_default_vault_dir"),

  /** Point the vault at `path`, seeding it there and persisting the choice. */
  setVaultDir: (path: string) => invoke<void>("set_vault_dir", { path }),

  // ── Ollama setup automation ───────────────────────────────────────────
  /** Detect install/server/model state. */
  ollamaDetect: () => invoke<OllamaStatus>("ollama_detect"),

  /** RAM (GiB) + recommended models for this machine. */
  ollamaRecommendedModels: () =>
    invoke<RecommendedModels>("ollama_recommended_models"),

  /** Install Ollama via winget (long-running). */
  ollamaInstall: () => invoke<void>("ollama_install"),

  /** Ensure `ollama serve` is running. */
  ollamaEnsureRunning: () => invoke<void>("ollama_ensure_running"),

  /** Pull a model; subscribe with {@link onOllamaPullProgress} for progress. */
  ollamaPull: (model: string) => invoke<void>("ollama_pull", { model }),

  /** Subscribe to model-pull progress events. Returns an unlisten fn. */
  onOllamaPullProgress: (cb: (p: PullProgress) => void): Promise<UnlistenFn> =>
    listen<PullProgress>("ollama:pull-progress", (e) => cb(e.payload)),

  // ── Google Calendar ───────────────────────────────────────────────────
  /** Run OAuth loopback flow with the user's own client credentials. */
  googleConnect: (clientId: string, clientSecret: string) =>
    invoke<void>("google_connect", { clientId, clientSecret }),

  /** Whether Google is currently connected. */
  googleStatus: () => invoke<boolean>("google_status"),

  /** Forget stored Google tokens. */
  googleDisconnect: () => invoke<void>("google_disconnect"),

  /** Smoke-test: list the user's calendars. */
  googleListCalendars: () =>
    invoke<CalendarListEntry[]>("google_list_calendars"),

  // ── ntfy phone push ───────────────────────────────────────────────────
  /** Current ntfy config, or `null` if not set up. */
  ntfyGetConfig: () => invoke<NtfyConfig | null>("ntfy_get_config"),

  /** Ensure a private topic exists (optionally override base URL) and return it. */
  ntfySetup: (baseUrl?: string) =>
    invoke<NtfyConfig>("ntfy_setup", { baseUrl: baseUrl ?? null }),

  /** Send the canned "connected" test push. */
  ntfySendTest: () => invoke<void>("ntfy_send_test"),

  // ── Agent chat ────────────────────────────────────────────────────────
  /**
   * Run one agent turn for `message`, invoking `onEvent` for each streamed
   * `AgentEvent` (citations, tokens, tool calls/results, done/error). The
   * listener is wired up before the command is invoked so no early events
   * are missed. Returns a disposer that unsubscribes the listener; call it
   * once the turn is done (e.g. after an `AgentEvent` with `type: "done"`
   * or `"error"`) or on unmount.
   */
  chat: async (message: string, onEvent: (e: AgentEvent) => void): Promise<() => void> => {
    const unlisten = await listen<AgentEvent>("agent:event", (e) => onEvent(e.payload));
    try {
      await invoke<void>("chat", { message });
    } catch (err) {
      unlisten();
      throw err;
    }
    return unlisten;
  },

  // ── Domain reads (diet, gym, study, calendar) ────────────────────────
  /** Diet logs for `date` (`YYYY-MM-DD`), oldest first. */
  dietLogsForDate: (date: string) => invoke<DietLog[]>("diet_logs_for_date", { date }),

  /** The most recently-effective diet targets, or `null` if none are set. */
  dietCurrentTargets: () => invoke<DietTargets | null>("diet_current_targets"),

  /** Most recent gym sessions, newest first. */
  gymRecentSessions: (limit: number) => invoke<GymSession[]>("gym_recent_sessions", { limit }),

  /** Most recent sets logged for `exercise`, newest session first. */
  gymSetsForExercise: (exercise: string, limit: number) =>
    invoke<GymSet[]>("gym_sets_for_exercise", { exercise, limit }),

  /** SRS cards due at or before `now` (RFC3339 timestamp), soonest first. */
  studyDueCards: (now: string) => invoke<SrsCard[]>("study_due_cards", { now }),

  /** Calendar events starting in `[start, end)` (RFC3339 timestamps), ascending. */
  calendarEventsBetween: (start: string, end: string) =>
    invoke<CalendarEvent[]>("calendar_events_between", { start, end }),

  /** Sync the primary Google calendar into the local cache now. Returns the number of events upserted. */
  calendarSyncNow: () => invoke<number>("calendar_sync_now"),

  /** Create a new event on the primary Google calendar (and the local cache). */
  createCalendarEvent: (args: {
    summary: string;
    startRfc3339: string;
    endRfc3339: string;
    description?: string;
    location?: string;
  }) => invoke<void>("calendar_create_event", args),

  // ── Knowledge (vault notes) ──────────────────────────────────────────
  /** Summaries of every note in the vault, newest-modified first. */
  vaultListNotes: () => invoke<NoteSummary[]>("vault_list_notes"),
};
