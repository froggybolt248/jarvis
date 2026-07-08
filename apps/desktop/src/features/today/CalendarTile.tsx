import { Tile } from "../../components/ui/Tile";
import { Badge } from "../../components/ui/Badge";
import { CalendarIcon } from "../../components/icons";
import { upcomingEvents } from "../../mock/calendar";
import { cn } from "../../lib/cn";

export interface CalendarTileProps {
  className?: string;
}

export function CalendarTile({ className }: CalendarTileProps) {
  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <CalendarIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Calendar</span>
      </div>
      <ul className="flex flex-col gap-2">
        {upcomingEvents.map((event, i) => (
          <li key={event.id} className="flex items-center justify-between gap-3 text-sm">
            <span className="flex min-w-0 items-center gap-2">
              <span className="truncate text-ink">{event.title}</span>
              {i === 0 ? <Badge tone="accent">Next</Badge> : null}
            </span>
            <span className="shrink-0 text-xs text-ink-faint">{event.time}</span>
          </li>
        ))}
      </ul>
    </Tile>
  );
}
