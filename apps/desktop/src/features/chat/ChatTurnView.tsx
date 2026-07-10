import { cn } from "../../lib/cn";
import { useChatStore } from "../../state/chat";
import { Button } from "../../components/ui/Button";
import { Kbd } from "../../components/ui/Kbd";
import { CitationChip } from "./CitationChip";

/** Small spinning ring, matches the one used in onboarding's OllamaStep. */
function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-block h-3.5 w-3.5 animate-spin rounded-full border-[1.5px] border-hairline-strong border-t-accent",
        className,
      )}
      aria-hidden="true"
    />
  );
}

export interface ChatTurnViewProps {
  /** Unsubscribe + return the palette to search mode. */
  onBack: () => void;
}

/**
 * The body of the command palette once it has morphed into "ask" mode:
 * streaming answer text, a subtle tool-call indicator, numbered citation
 * chips, and a readable error state. Rendered in place of `Command.List`.
 */
export function ChatTurnView({ onBack }: ChatTurnViewProps) {
  const status = useChatStore((s) => s.status);
  const answer = useChatStore((s) => s.answer);
  const citations = useChatStore((s) => s.citations);
  const toolActivity = useChatStore((s) => s.toolActivity);
  const error = useChatStore((s) => s.error);
  const ask = useChatStore((s) => s.ask);
  const question = useChatStore((s) => s.question);

  const awaitingFirstToken = status === "streaming" && answer.length === 0 && !toolActivity;

  return (
    <div className="flex flex-col">
      <div className="flex max-h-96 flex-col gap-3 overflow-y-auto p-4">
        {awaitingFirstToken ? (
          <div className="flex items-center gap-2.5 text-sm text-ink-dim">
            <Spinner />
            Thinking…
          </div>
        ) : null}

        {answer ? (
          <p className="whitespace-pre-wrap break-words text-sm leading-relaxed text-ink">{answer}</p>
        ) : null}

        {status === "running_tool" && toolActivity ? (
          <div className="flex items-center gap-2 text-xs text-ink-faint">
            <Spinner />
            using {toolActivity.name}…
          </div>
        ) : null}

        {status === "error" ? (
          <div className="flex flex-col gap-2 rounded-tile border border-hairline bg-surface-2 px-3 py-2.5">
            <p className="text-sm text-caution">{error ?? "Something went wrong."}</p>
          </div>
        ) : null}

        {citations.length > 0 ? (
          <div className="flex flex-wrap gap-1.5 pt-1">
            {citations.map((c) => (
              <CitationChip key={c.index} citation={c} />
            ))}
          </div>
        ) : null}
      </div>

      <div className="flex items-center justify-between gap-2 border-t border-hairline px-4 py-2.5">
        <div>
          {status === "error" ? (
            <Button variant="ghost" onClick={() => void ask(question)}>
              Try again
            </Button>
          ) : null}
        </div>
        <div className="flex items-center gap-3 text-xs text-ink-faint">
          {status === "done" || status === "error" ? (
            <Button variant="ghost" onClick={onBack}>
              New question
            </Button>
          ) : null}
          <span className="flex items-center gap-1.5">
            <Kbd>esc</Kbd>
            to close
          </span>
        </div>
      </div>
    </div>
  );
}
