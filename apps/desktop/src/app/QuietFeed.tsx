import { motion } from "motion/react";
import { useUiStore } from "../state/ui";
import { feedEntries } from "../mock/feed";
import { ChevronDownIcon, ClockIcon } from "../components/icons";

/**
 * Bottom strip: a collapsed single-line ticker of the latest Jarvis action,
 * expandable to a short panel listing recent entries. Never modal — it sits
 * quietly at the edge of the shell.
 */
export function QuietFeed() {
  const expanded = useUiStore((s) => s.feedExpanded);
  const setExpanded = useUiStore((s) => s.setFeedExpanded);
  const latest = feedEntries[0];

  return (
    <div className="shrink-0 border-t border-hairline bg-surface-0">
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex h-9 w-full items-center gap-2 px-4 text-left text-xs text-ink-dim transition-colors duration-150 hover:text-ink"
      >
        <ClockIcon size={14} className="shrink-0 text-ink-faint" />
        <span className="min-w-0 flex-1 truncate">
          <span className="text-ink-faint">{latest.time}</span>
          <span className="mx-1.5">·</span>
          {latest.text}
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
          <ul className="flex flex-col gap-1.5">
            {feedEntries.map((entry) => (
              <li key={entry.id} className="flex items-center gap-2 text-xs">
                <span className="w-16 shrink-0 text-ink-faint">{entry.time}</span>
                <span className="text-ink-dim">{entry.text}</span>
              </li>
            ))}
          </ul>
        </div>
      </motion.div>
    </div>
  );
}
