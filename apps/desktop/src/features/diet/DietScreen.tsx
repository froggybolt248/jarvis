import { useState } from "react";
import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { ProgressRing } from "../../components/ui/ProgressRing";
import { Button } from "../../components/ui/Button";
import { FlameIcon } from "../../components/icons";
import { ipc } from "../../lib/ipc";
import { useDietToday } from "./useDietToday";

const inputClass =
  "rounded-tile border border-hairline bg-surface-2 px-2 py-1 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus:ring-1 focus:ring-accent";

function LogMealForm({ onLogged }: { onLogged: () => void }) {
  const [description, setDescription] = useState("");
  const [calories, setCalories] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    const trimmed = description.trim();
    if (!trimmed || saving) return;
    setSaving(true);
    try {
      await ipc.dietLogMeal({
        description: trimmed,
        calories: calories.trim() ? Number(calories) : undefined,
      });
      setDescription("");
      setCalories("");
      onLogged();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <input
        className={inputClass}
        placeholder="What did you eat?"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <div className="flex items-center gap-2">
        <input
          className={`${inputClass} flex-1`}
          placeholder="kcal"
          inputMode="numeric"
          value={calories}
          onChange={(e) => setCalories(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
        <Button variant="accent" onClick={submit} disabled={!description.trim() || saving}>
          Log
        </Button>
      </div>
    </div>
  );
}

const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatTime(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : timeFormat.format(d);
}

const macroRows = (
  totals: { protein_g: number; carbs_g: number; fat_g: number },
  targets: { protein_g: number | null; carbs_g: number | null; fat_g: number | null } | null,
) => [
  { label: "Protein", grams: totals.protein_g, target: targets?.protein_g ?? null },
  { label: "Carbs", grams: totals.carbs_g, target: targets?.carbs_g ?? null },
  { label: "Fat", grams: totals.fat_g, target: targets?.fat_g ?? null },
];

function LoadingCard() {
  return (
    <Card>
      <div className="mb-3 h-4 w-24 animate-pulse rounded-pill bg-surface-2" />
      <div className="flex items-center gap-4">
        <div className="h-[72px] w-[72px] shrink-0 animate-pulse rounded-full bg-surface-2" />
        <div className="flex flex-1 flex-col gap-2">
          <div className="h-2 w-full animate-pulse rounded-pill bg-surface-2" />
          <div className="h-2 w-full animate-pulse rounded-pill bg-surface-2" />
          <div className="h-2 w-full animate-pulse rounded-pill bg-surface-2" />
        </div>
      </div>
    </Card>
  );
}

export function DietScreen() {
  const { loading, error, logs, targets, totals, refetch } = useDietToday();

  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <div className="flex items-center gap-2.5">
          <span className="rounded-tile bg-dom-diet/15 p-1.5 text-dom-diet">
            <FlameIcon size={18} />
          </span>
          <h1 className="text-2xl font-semibold tracking-tight text-ink">Diet</h1>
        </div>

        <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
          {loading ? (
            <LoadingCard />
          ) : (
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">Today</h2>

              {error ? (
                <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
              ) : logs.length === 0 && !targets ? (
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <span className="rounded-tile bg-dom-diet/15 p-1.5 text-dom-diet">
                    <FlameIcon size={20} />
                  </span>
                  <p className="text-sm text-ink-dim">Nothing logged yet today.</p>
                  <p className="text-xs text-ink-faint">Tell Jarvis what you ate and it'll show up here.</p>
                </div>
              ) : (
                <div className="flex flex-col gap-5">
                  <div className="flex items-center gap-4">
                    {targets?.calories ? (
                      <ProgressRing
                        value={totals.calories}
                        max={targets.calories}
                        size={80}
                        strokeWidth={7}
                        color="var(--color-dom-diet)"
                      >
                        <div className="flex flex-col items-center">
                          <span className="text-lg font-semibold tabular-nums text-ink">{Math.round(totals.calories)}</span>
                          <span className="text-[10px] text-ink-faint">/ {targets.calories}</span>
                        </div>
                      </ProgressRing>
                    ) : (
                      <div className="flex flex-col">
                        <span className="text-3xl font-semibold tabular-nums text-ink">{Math.round(totals.calories)}</span>
                        <span className="text-xs text-ink-dim">kcal logged</span>
                      </div>
                    )}
                    <div className="flex flex-1 flex-col gap-2">
                      {macroRows(totals, targets).map((macro) => {
                        const pct = macro.target ? Math.min(100, Math.round((macro.grams / macro.target) * 100)) : null;
                        return (
                          <div key={macro.label} className="flex flex-col gap-1">
                            <div className="flex items-center justify-between text-[11px] text-ink-faint">
                              <span>{macro.label}</span>
                              <span>
                                {Math.round(macro.grams)}g{macro.target ? ` / ${macro.target}g` : ""}
                              </span>
                            </div>
                            {pct !== null ? (
                              <div className="h-1 w-full overflow-hidden rounded-pill bg-surface-3">
                                <div className="h-full rounded-pill bg-dom-diet" style={{ width: `${pct}%` }} />
                              </div>
                            ) : null}
                          </div>
                        );
                      })}
                    </div>
                  </div>

                  {logs.length > 0 ? (
                    <div className="flex flex-col">
                      {logs.map((log, i) => (
                        <div key={log.id}>
                          {i > 0 ? <Divider className="my-3" /> : null}
                          <div className="flex items-center justify-between gap-4">
                            <div className="flex flex-col gap-0.5">
                              <span className="text-sm text-ink">{log.description}</span>
                              <span className="text-xs text-ink-faint">{formatTime(log.logged_at)}</span>
                            </div>
                            {log.calories !== null ? (
                              <span className="shrink-0 text-xs text-ink-dim">{log.calories} kcal</span>
                            ) : null}
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-xs text-ink-faint">No meals logged yet today.</p>
                  )}
                </div>
              )}
            </Card>
          )}

          <Card>
            <h2 className="mb-3 text-sm font-medium text-ink-dim">Log meal</h2>
            <LogMealForm onLogged={refetch} />
          </Card>
        </div>
      </div>
    </div>
  );
}
