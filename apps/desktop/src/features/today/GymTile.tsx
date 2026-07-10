import { Tile } from "../../components/ui/Tile";
import { Sparkline } from "../../components/ui/Sparkline";
import { DumbbellIcon } from "../../components/icons";
import { useGymSessions } from "../gym/useGymSessions";
import { cn } from "../../lib/cn";

export interface GymTileProps {
  className?: string;
}

const dateFormat = new Intl.DateTimeFormat("en-US", { weekday: "short" });

export function GymTile({ className }: GymTileProps) {
  const { loading, error, sessions, durationTrend } = useGymSessions(7);
  const empty = !loading && !error && sessions.length === 0;
  const last = sessions[0];

  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <DumbbellIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Gym</span>
      </div>

      {loading ? (
        <div className="h-4 w-3/4 animate-pulse rounded-pill bg-surface-2" />
      ) : empty ? (
        <p className="text-sm text-ink-faint">No sessions logged yet.</p>
      ) : (
        <div className="flex items-end justify-between gap-3">
          <p className="text-sm text-ink">
            {sessions.length} recent session{sessions.length === 1 ? "" : "s"}
            {last ? (
              <span className="text-ink-faint"> • last: {dateFormat.format(new Date(last.started_at))}</span>
            ) : null}
          </p>
          {durationTrend.length >= 2 ? <Sparkline values={durationTrend} className="shrink-0" /> : null}
        </div>
      )}
    </Tile>
  );
}
