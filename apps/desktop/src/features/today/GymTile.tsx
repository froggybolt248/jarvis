import { Tile } from "../../components/ui/Tile";
import { Sparkline } from "../../components/ui/Sparkline";
import { DumbbellIcon } from "../../components/icons";
import { gymStatus, sessionTrend } from "../../mock/gym";
import { cn } from "../../lib/cn";

export interface GymTileProps {
  className?: string;
}

export function GymTile({ className }: GymTileProps) {
  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <DumbbellIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Gym</span>
      </div>
      <div className="flex items-end justify-between gap-3">
        <p className="text-sm text-ink">
          {gymStatus.scheduled}
          <span className="text-ink-faint"> • last: {gymStatus.lastSession}</span>
        </p>
        <Sparkline values={sessionTrend} className="shrink-0" />
      </div>
    </Tile>
  );
}
