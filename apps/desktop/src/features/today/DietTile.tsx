import { Tile } from "../../components/ui/Tile";
import { ProgressRing } from "../../components/ui/ProgressRing";
import { FlameIcon } from "../../components/icons";
import { useDietToday } from "../diet/useDietToday";
import { cn } from "../../lib/cn";

export interface DietTileProps {
  className?: string;
}

export function DietTile({ className }: DietTileProps) {
  const { loading, error, logs, targets, totals } = useDietToday();
  const empty = !loading && !error && logs.length === 0 && !targets;

  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2">
        <span className="text-dom-diet">
          <FlameIcon size={16} />
        </span>
        <span className="text-xs font-medium uppercase tracking-wide text-ink-dim">Diet</span>
      </div>

      {loading ? (
        <div className="flex items-center gap-4">
          <div className="h-[72px] w-[72px] shrink-0 animate-pulse rounded-full bg-surface-2" />
          <div className="flex flex-1 flex-col gap-2">
            <div className="h-2 w-full animate-pulse rounded-pill bg-surface-2" />
            <div className="h-2 w-full animate-pulse rounded-pill bg-surface-2" />
          </div>
        </div>
      ) : empty ? (
        <p className="text-sm text-ink-faint">Nothing logged yet today.</p>
      ) : (
        <div className="flex items-center gap-4">
          {targets?.calories ? (
            <ProgressRing
              value={totals.calories}
              max={targets.calories}
              size={72}
              strokeWidth={6}
              color="var(--color-dom-diet)"
            >
              <div className="flex flex-col items-center">
                <span className="text-sm font-semibold text-ink">{Math.round(totals.calories)}</span>
                <span className="text-[10px] text-ink-faint">/ {targets.calories}</span>
              </div>
            </ProgressRing>
          ) : (
            <div className="flex flex-col">
              <span className="text-sm font-semibold text-ink">{Math.round(totals.calories)}</span>
              <span className="text-[10px] text-ink-faint">kcal today</span>
            </div>
          )}
          <div className="flex flex-1 flex-col gap-2">
            {[
              { label: "Protein", grams: totals.protein_g, target: targets?.protein_g ?? null },
              { label: "Carbs", grams: totals.carbs_g, target: targets?.carbs_g ?? null },
              { label: "Fat", grams: totals.fat_g, target: targets?.fat_g ?? null },
            ].map((macro) => {
              const pct = macro.target ? Math.min(100, Math.round((macro.grams / macro.target) * 100)) : 0;
              return (
                <div key={macro.label} className="flex flex-col gap-1">
                  <div className="flex items-center justify-between text-[11px] text-ink-faint">
                    <span>{macro.label}</span>
                    <span>{Math.round(macro.grams)}g</span>
                  </div>
                  <div className="h-1 w-full overflow-hidden rounded-pill bg-surface-3">
                    <div className="h-full rounded-pill bg-dom-diet" style={{ width: `${pct}%` }} />
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </Tile>
  );
}
