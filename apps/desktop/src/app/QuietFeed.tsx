import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { useUiStore } from "../state/ui";
import { ipc, type QuietFeedItem } from "../lib/ipc";
import { ChevronDownIcon, ClockIcon } from "../components/icons";

const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatFeedTime(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : timeFormat.format(d);
}

/**
 * Bottom strip: a collapsed single-line ticker of the latest Jarvis action,
 * expandable to a short panel listing recent entries. Never modal — it sits
 * quietly at the edge of the shell. Entries are the real Quiet Feed audit
 * rows every mutating tool writes, polled at a slow interval and refreshed
 * on expand.
 */
export function QuietFeed() {
  const expanded = useUiStore((s) => s.feedExpanded);
  const setExpanded = useUiStore((s) => s.setFeedExpanded);
  const [items, setItems] = useState<QuietFeedItem[]>([]);

  useEffect(() => {
    let cancelled = false;
    const fetchFeed = () => {
      ipc
        .recentFeed(30)
        .then((rows) => {
          if (!cancelled) setItems(rows);
        })
        .catch(() => {});
    };
    fetchFeed();
    const interval = setInterval(fetchFeed, 30_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [expanded]);

  const latest = items[0];

  return (
    <div className="shrink-0 border-t border-hairline bg-surface-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex h-9 w-full items-center gap-2 px-4 text-left text-xs text-ink-dim transition-colors duration-150 hover:text-ink"
      >
        <ClockIcon size={14} className="shrink-0 text-ink-faint" />
        <span className="min-w-0 flex-1 truncate">
          {latest ? (
            <>
              <span className="text-ink-faint">{formatFeedTime(latest.created_at)}</span>
              <span className="mx-1.5">·</span>
              {latest.title}
            </>
          ) : (
            <span className="text-ink-faint">No actions yet — everything Jarvis does shows up here.</span>
          )}
        </span>
        <motion.span
          animate={{ rotate: expanded ? 180 : 0 }}
          transition={{ type: "spring", stiffness: 420, damping: 32 }}
          className="shrink-0 text-ink-faint"
        >
          <ChevronDownIcon size={14} />
        </motion.span>
      </button>

      <motion.div
        initial={false}
        animate={{ height: expanded ? 200 : 0 }}
        transition={{ type: "spring", stiffness: 380, damping: 38 }}
        className="overflow-hidden"
      >
        <div className="max-h-[200px] overflow-y-auto border-t border-hairline px-4 py-2">
          {items.length === 0 ? (
            <p className="py-2 text-xs text-ink-faint">
              Nothing logged yet. When Jarvis takes an action — logs a meal, creates an event — it's recorded here.
            </p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {items.map((entry) => (
                <li key={entry.id} className="flex items-center gap-2 text-xs">
                  <span className="w-16 shrink-0 text-ink-faint">{formatFeedTime(entry.created_at)}</span>
                  <span className="min-w-0 truncate text-ink-dim">{entry.title}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </motion.div>
    </div>
  );
}
