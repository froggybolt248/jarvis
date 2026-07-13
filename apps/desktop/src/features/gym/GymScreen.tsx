import { useState } from "react";
import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { Sparkline } from "../../components/ui/Sparkline";
import { Button } from "../../components/ui/Button";
import { DumbbellIcon } from "../../components/icons";
import { ipc } from "../../lib/ipc";
import { useGymSessions, sessionDurationMinutes } from "./useGymSessions";

const inputClass =
  "rounded-tile border border-hairline bg-surface-2 px-2 py-1 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus:ring-1 focus:ring-accent";

function LogSetForm({ onLogged }: { onLogged: () => void }) {
  const [exercise, setExercise] = useState("");
  const [weight, setWeight] = useState("");
  const [reps, setReps] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    const trimmed = exercise.trim();
    if (!trimmed || saving) return;
    setSaving(true);
    try {
      await ipc.gymLogWorkout({
        sets: [
          {
            exercise: trimmed,
            weight: weight.trim() ? Number(weight) : undefined,
            reps: reps.trim() ? Number(reps) : undefined,
          },
        ],
      });
      setExercise("");
      setWeight("");
      setReps("");
      onLogged();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <input
        className={inputClass}
        placeholder="Exercise"
        value={exercise}
        onChange={(e) => setExercise(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <div className="flex items-center gap-2">
        <input
          className={`${inputClass} flex-1`}
          placeholder="weight"
          inputMode="decimal"
          value={weight}
          onChange={(e) => setWeight(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
        <input
          className={`${inputClass} w-16`}
          placeholder="reps"
          inputMode="numeric"
          value={reps}
          onChange={(e) => setReps(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && submit()}
        />
        <Button variant="accent" onClick={submit} disabled={!exercise.trim() || saving}>
          Log
        </Button>
      </div>
    </div>
  );
}

const dateFormat = new Intl.DateTimeFormat("en-US", { weekday: "short", month: "short", day: "numeric" });
const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatSessionDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : `${dateFormat.format(d)} · ${timeFormat.format(d)}`;
}

function LoadingCard() {
  return (
    <Card>
      <div className="mb-3 h-4 w-24 animate-pulse rounded-pill bg-surface-2" />
      <div className="flex flex-col gap-3">
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-2/3 animate-pulse rounded-pill bg-surface-2" />
      </div>
    </Card>
  );
}

export function GymScreen() {
  const { loading, error, sessions, durationTrend, refetch } = useGymSessions(10);

  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <div className="flex items-center gap-2.5">
          <span className="rounded-tile bg-dom-gym/15 p-1.5 text-dom-gym">
            <DumbbellIcon size={18} />
          </span>
          <h1 className="text-2xl font-semibold tracking-tight text-ink">Gym</h1>
        </div>

        <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
          {loading ? (
            <LoadingCard />
          ) : (
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">Training</h2>

              {error ? (
                <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
              ) : sessions.length === 0 ? (
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <span className="rounded-tile bg-dom-gym/15 p-1.5 text-dom-gym">
                    <DumbbellIcon size={20} />
                  </span>
                  <p className="text-sm text-ink-dim">No sessions logged yet.</p>
                  <p className="text-xs text-ink-faint">Tell Jarvis about a workout and it'll show up here.</p>
                </div>
              ) : (
                <div className="flex flex-col">
                  {durationTrend.length >= 2 ? (
                    <>
                      <div className="flex items-center justify-between gap-4">
                        <span className="text-sm text-ink">Last {durationTrend.length} sessions</span>
                        <Sparkline values={durationTrend} color="var(--color-dom-gym)" />
                      </div>
                      <Divider className="my-3" />
                    </>
                  ) : null}
                  {sessions.map((session, i) => {
                    const minutes = sessionDurationMinutes(session);
                    return (
                      <div key={session.id}>
                        {i > 0 ? <Divider className="my-3" /> : null}
                        <div className="flex items-center justify-between gap-4">
                          <div className="flex flex-col gap-0.5">
                            <span className="text-sm text-ink">{formatSessionDate(session.started_at)}</span>
                            {session.notes ? <span className="text-xs text-ink-faint">{session.notes}</span> : null}
                          </div>
                          <span className="shrink-0 text-xs text-ink-dim">
                            {minutes !== null ? `${minutes} min` : "In progress"}
                          </span>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </Card>
          )}

          <div className="flex flex-col gap-4">
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">Log set</h2>
              <LogSetForm onLogged={refetch} />
            </Card>
            <Card className="flex flex-col gap-2">
              <h2 className="text-sm font-medium text-ink-dim">Recent sessions</h2>
              <span className="text-3xl font-semibold tabular-nums text-ink">{sessions.length}</span>
              <p className="text-xs text-ink-faint">in the last 10 logged</p>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
