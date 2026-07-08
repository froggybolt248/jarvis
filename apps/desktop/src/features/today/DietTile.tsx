import { Tile } from "../../components/ui/Tile";
import { ProgressRing } from "../../components/ui/ProgressRing";
import { FlameIcon } from "../../components/icons";
import { calories, macros } from "../../mock/diet";
import { cn } from "../../lib/cn";

export interface DietTileProps {
  className?: string;
}

export function DietTile({ className }: DietTileProps) {
  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <FlameIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Diet</span>
      </div>
      <div className="flex items-center gap-4">
        <ProgressRing value={calories.consumed} max={calories.target} size={72} strokeWidth={6}>
          <div className="flex flex-col items-center">
            <span className="text-sm font-semibold text-ink">{calories.consumed}</span>
            <span className="text-[10px] text-ink-faint">/ {calories.target}</span>
          </div>
        </ProgressRing>
        <div className="flex flex-1 flex-col gap-2">
          {macros.map((macro) => {
            const pct = Math.min(100, Math.round((macro.grams / macro.target) * 100));
            return (
              <div key={macro.label} className="flex flex-col gap-1">
                <div className="flex items-center justify-between text-[11px] text-ink-faint">
                  <span>{macro.label}</span>
                  <span>{macro.grams}g</span>
                </div>
                <div className="h-1 w-full overflow-hidden rounded-pill bg-surface-3">
                  <div
                    className="h-full rounded-pill"
                    style={{ width: `${pct}%`, background: macro.colorVar }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </Tile>
  );
}
