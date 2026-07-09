// Typed bridge to the Rust core. Every `invoke` call in the app goes through
// here so command names and payload shapes live in exactly one place and stay
// in sync with `src-tauri/src/commands`.
import { invoke } from "@tauri-apps/api/core";

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
};
