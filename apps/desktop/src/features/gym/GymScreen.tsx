import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { Sparkline } from "../../components/ui/Sparkline";
import { DumbbellIcon } from "../../components/icons";
import { useGymSessions, sessionDurationMinutes } from "./useGymSessions";

const dateFormat = new Intl.DateTimeFormat("en-US", { weekday: "short", month: "short", day: "numeric" });
const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatSessionDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : `${dateFormat.format(d)} · ${timeFormat.format(d)}`;
}

function LoadingCard() {
  return (
    <Card className="max-w-xl">
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
  const { loading, error, sessions, durationTrend } = useGymSessions(10);

  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Gym</h1>

      {loading ? (
        <LoadingCard />
      ) : (
        <Card className="max-w-xl">
          <h2 className="mb-3 text-sm font-medium text-ink-dim">Training</h2>

          {error ? (
            <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
          ) : sessions.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-6 text-center">
              <DumbbellIcon size={20} className="text-ink-faint" />
              <p className="text-sm text-ink-dim">No sessions logged yet.</p>
              <p className="text-xs text-ink-faint">Tell Jarvis about a workout and it'll show up here.</p>
            </div>
          ) : (
            <div className="flex flex-col">
              {durationTrend.length >= 2 ? (
                <>
                  <div className="flex items-center justify-between gap-4">
                    <span className="text-sm text-ink">Last {durationTrend.length} sessions</span>
                    <Sparkline values={durationTrend} />
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
    </div>
  );
}
