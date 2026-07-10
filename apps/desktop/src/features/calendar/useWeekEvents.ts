import { useCallback, useEffect, useState } from "react";
import { ipc, type CalendarEvent } from "../../lib/ipc";

export interface UseWeekEventsResult {
  loading: boolean;
  error: boolean;
  events: CalendarEvent[];
  /** Re-fetches the current week's events (e.g. after a manual sync). */
  refetch: () => void;
}

/** Fetches events from the start of today through 7 days out, ascending. */
export function useWeekEvents(): UseWeekEventsResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [refreshToken, setRefreshToken] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(end.getDate() + 7);

    ipc
      .calendarEventsBetween(start.toISOString(), end.toISOString())
      .then((e) => {
        if (!cancelled) setEvents(e);
      })
      .catch(() => {
        if (!cancelled) setError(true);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [refreshToken]);

  const refetch = useCallback(() => setRefreshToken((t) => t + 1), []);

  return { loading, error, events, refetch };
}

/** Groups events by local calendar day, keyed by `YYYY-MM-DD`, preserving ascending order. */
export function groupEventsByDay(events: CalendarEvent[]): Map<string, CalendarEvent[]> {
  const groups = new Map<string, CalendarEvent[]>();
  for (const event of events) {
    if (!event.start_at) continue;
    const d = new Date(event.start_at);
    if (Number.isNaN(d.getTime())) continue;
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const list = groups.get(key);
    if (list) list.push(event);
    else groups.set(key, [event]);
  }
  return groups;
}
