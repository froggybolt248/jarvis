import { useState } from "react";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { Divider } from "../../components/ui/Divider";
import { CalendarIcon } from "../../components/icons";
import { ipc } from "../../lib/ipc";
import { useWeekEvents, groupEventsByDay } from "./useWeekEvents";

const dayHeaderFormat = new Intl.DateTimeFormat("en-US", { weekday: "long", month: "short", day: "numeric" });
const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

function formatEventTime(event: { start_at: string | null; all_day: boolean }): string {
  if (event.all_day) return "All day";
  if (!event.start_at) return "";
  const d = new Date(event.start_at);
  return Number.isNaN(d.getTime()) ? "" : timeFormat.format(d);
}

function LoadingCard() {
  return (
    <Card>
      <div className="mb-3 h-4 w-20 animate-pulse rounded-pill bg-surface-2" />
      <div className="flex flex-col gap-3">
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-2/3 animate-pulse rounded-pill bg-surface-2" />
      </div>
    </Card>
  );
}

export function CalendarScreen() {
  const { loading, error, events, refetch } = useWeekEvents();
  const [syncing, setSyncing] = useState(false);
  const groups = groupEventsByDay(events);
  const days = Array.from(groups.keys()).sort();

  const handleSync = async () => {
    setSyncing(true);
    try {
      await ipc.calendarSyncNow();
    } catch {
      // Best-effort: Google may not be connected yet. Refetch regardless
      // so the screen reflects whatever the local cache currently has.
    } finally {
      setSyncing(false);
      refetch();
    }
  };

  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2.5">
            <span className="rounded-tile bg-dom-calendar/15 p-1.5 text-dom-calendar">
              <CalendarIcon size={18} />
            </span>
            <h1 className="text-2xl font-semibold tracking-tight text-ink">Calendar</h1>
          </div>
          <Button variant="ghost" onClick={handleSync} disabled={syncing}>
            {syncing ? "Syncing…" : "Sync now"}
          </Button>
        </div>

        <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
          {loading ? (
            <LoadingCard />
          ) : (
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">This week</h2>

              {error ? (
                <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
              ) : days.length === 0 ? (
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <span className="rounded-tile bg-dom-calendar/15 p-1.5 text-dom-calendar">
                    <CalendarIcon size={20} />
                  </span>
                  <p className="text-sm text-ink-dim">Nothing on the calendar this week.</p>
                  <p className="text-xs text-ink-faint">Connect Google Calendar in Settings to see events here.</p>
                </div>
              ) : (
                <div className="flex flex-col">
                  {days.map((day, dayIndex) => (
                    <div key={day}>
                      {dayIndex > 0 ? <Divider className="my-4" /> : null}
                      <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">
                        {dayHeaderFormat.format(new Date(`${day}T00:00:00`))}
                      </span>
                      <div className="mt-2 flex flex-col">
                        {groups.get(day)!.map((event, i) => (
                          <div key={event.id}>
                            {i > 0 ? <Divider className="my-3" /> : null}
                            <div className="flex items-center justify-between gap-4">
                              <div className="flex min-w-0 flex-col gap-0.5">
                                <span className="truncate text-sm text-ink">{event.summary ?? "Untitled event"}</span>
                                {event.location ? (
                                  <span className="truncate text-xs text-ink-faint">{event.location}</span>
                                ) : null}
                              </div>
                              <span className="shrink-0 text-xs text-ink-dim">{formatEventTime(event)}</span>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          )}

          <Card className="flex flex-col gap-2">
            <h2 className="text-sm font-medium text-ink-dim">This week at a glance</h2>
            <span className="text-3xl font-semibold tabular-nums text-ink">{events.length}</span>
            <p className="text-xs text-ink-faint">event{events.length === 1 ? "" : "s"} scheduled</p>
          </Card>
        </div>
      </div>
    </div>
  );
}
