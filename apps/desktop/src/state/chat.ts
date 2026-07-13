import { create } from "zustand";
import { ipc, type AgentEvent, type Citation } from "../lib/ipc";

export type ChatStatus = "idle" | "streaming" | "running_tool" | "done" | "error";

/** The most recent tool call in the current turn, and whether it has resolved. */
export interface ToolActivity {
  name: string;
  ok: boolean | null;
}

interface ChatState {
  status: ChatStatus;
  question: string;
  answer: string;
  citations: Citation[];
  toolActivity: ToolActivity | null;
  error: string | null;
  /** Unlisten fn for the in-flight turn's event stream, or `null` between turns. */
  unlisten: (() => void) | null;

  /** Start a turn for `message`. No-op if a turn is already in flight. */
  ask: (message: string) => Promise<void>;
  /** Unsubscribe (if listening) and return to the idle/search state. */
  reset: () => void;
}

const initial = {
  status: "idle" as ChatStatus,
  question: "",
  answer: "",
  citations: [] as Citation[],
  toolActivity: null as ToolActivity | null,
  error: null as string | null,
  unlisten: null as (() => void) | null,
};

export const useChatStore = create<ChatState>((set, get) => ({
  ...initial,

  ask: async (message: string) => {
    const current = get().status;
    if (current === "streaming" || current === "running_tool") return;

    // Defensive: unsubscribe any stray listener from a previous turn.
    get().unlisten?.();

    set({
      ...initial,
      status: "streaming",
      question: message,
    });

    const handleEvent = (e: AgentEvent) => {
      // A stray event delivered after this turn already ended (or was reset)
      // must not resurrect state; the listener is disposed synchronously
      // below on every terminal path, but events can still be in flight.
      if (get().status !== "streaming" && get().status !== "running_tool") return;
      switch (e.type) {
        case "citations":
          set({ citations: e.citations });
          break;
        case "token":
          set((s) => ({ answer: s.answer + e.text }));
          break;
        case "tool_call":
          set({ status: "running_tool", toolActivity: { name: e.name, ok: null } });
          break;
        case "tool_result":
          set((s) => ({
            status: "streaming",
            toolActivity:
              s.toolActivity && s.toolActivity.name === e.name
                ? { name: e.name, ok: e.ok }
                : s.toolActivity,
          }));
          break;
        case "done": {
          set({ status: "done", toolActivity: null });
          const fn = get().unlisten;
          set({ unlisten: null });
          fn?.();
          break;
        }
        case "error": {
          set({ status: "error", error: e.message, toolActivity: null });
          const fn = get().unlisten;
          set({ unlisten: null });
          fn?.();
          break;
        }
      }
    };

    // Register the listener — and stash its disposer in the store — before
    // invoking the command. `invoke("chat")` doesn't resolve until the whole
    // turn is done, so storing the disposer only after `await`ing it (the
    // previous bug) left every terminal handler's `get().unlisten?.()` a
    // no-op for the entire duration of the turn.
    let disposer: (() => void) | null = null;
    try {
      disposer = await ipc.listenAgentEvents(handleEvent);
    } catch (err) {
      set({ status: "error", error: String(err), unlisten: null });
      return;
    }

    // reset() (or another race) may have already moved us out of this turn
    // while we were awaiting listener registration — don't clobber it.
    if (get().status !== "streaming" && get().status !== "running_tool") {
      disposer();
      return;
    }
    set({ unlisten: disposer });

    try {
      await ipc.invokeChat(message);
    } catch (err) {
      // Only surface the failure if this turn is still live — a reset() (Esc,
      // palette close) may have already returned us to idle, and a stale
      // rejection must not resurrect an error state into a fresh palette.
      if (get().status !== "streaming" && get().status !== "running_tool") return;
      const fn = get().unlisten;
      set({ status: "error", error: String(err), unlisten: null });
      fn?.();
    }
  },

  reset: () => {
    get().unlisten?.();
    set({ ...initial });
  },
}));
