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
};
