import { Tile } from "../../components/ui/Tile";
import { Badge } from "../../components/ui/Badge";
import { CalendarIcon } from "../../components/icons";
import { useWeekEvents } from "../calendar/useWeekEvents";
import { cn } from "../../lib/cn";

export interface CalendarTileProps {
  className?: string;
}

const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatEventTime(event: { start_at: string | null; all_day: boolean }): string {
  if (event.all_day) return "All day";
  if (!event.start_at) return "";
  const d = new Date(event.start_at);
  return Number.isNaN(d.getTime()) ? "" : timeFormat.format(d);
}

export function CalendarTile({ className }: CalendarTileProps) {
  const { loading, error, events } = useWeekEvents();
  const upcoming = events.filter((e) => !e.start_at || new Date(e.start_at).getTime() >= Date.now()).slice(0, 3);
  const empty = !loading && !error && upcoming.length === 0;

  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <CalendarIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Calendar</span>
      </div>

      {loading ? (
        <div className="flex flex-col gap-2">
          <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
          <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        </div>
      ) : empty ? (
        <p className="text-sm text-ink-faint">Nothing coming up this week.</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {upcoming.map((event, i) => (
            <li key={event.id} className="flex items-center justify-between gap-3 text-sm">
              <span className="flex min-w-0 items-center gap-2">
                <span className="truncate text-ink">{event.summary ?? "Untitled event"}</span>
                {i === 0 ? <Badge tone="accent">Next</Badge> : null}
              </span>
              <span className="shrink-0 text-xs text-ink-faint">{formatEventTime(event)}</span>
            </li>
          ))}
        </ul>
      )}
    </Tile>
  );
}
