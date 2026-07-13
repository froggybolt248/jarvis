import { useCallback, useEffect, useState } from "react";
import { ipc, type CalendarEvent, type DietTargets } from "../../lib/ipc";
import { sumDietLogs, type DietTotals } from "../diet/useDietToday";

function todayDateString(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export interface BriefingData {
  todayEvents: CalendarEvent[];
  dueCardCount: number;
  dietTotals: DietTotals;
  dietTargets: DietTargets | null;
}

export interface UseBriefingResult {
  loading: boolean;
  data: BriefingData | null;
  refetch: () => void;
}

/**
 * The real data behind the Today hero: today's events, due study cards, and
 * diet progress, fetched together. Each source degrades independently — a
 * failed fetch contributes an empty slice rather than failing the briefing.
 */
export function useBriefing(): UseBriefingResult {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<BriefingData | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  useEffect(() => {
    let cancelled = false;
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    const end = new Date(start);
    end.setDate(end.getDate() + 1);

    Promise.all([
      ipc.calendarEventsBetween(start.toISOString(), end.toISOString()).catch(() => []),
      ipc.studyDueCards(new Date().toISOString()).catch(() => []),
      ipc.dietLogsForDate(todayDateString()).catch(() => []),
      ipc.dietCurrentTargets().catch(() => null),
    ]).then(([events, cards, logs, targets]) => {
      if (cancelled) return;
      setData({
        todayEvents: events,
        dueCardCount: cards.length,
        dietTotals: sumDietLogs(logs),
        dietTargets: targets,
      });
      setLoading(false);
    });

    return () => {
      cancelled = true;
    };
  }, [refreshToken]);

  const refetch = useCallback(() => setRefreshToken((t) => t + 1), []);

  return { loading, data, refetch };
}

export function greetingForHour(hour: number): string {
  if (hour < 5) return "Up late?";
  if (hour < 12) return "Good morning.";
  if (hour < 18) return "Good afternoon.";
  return "Good evening.";
}

const timeFormat = new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "2-digit" });

/** The next event starting after `now`, if any. */
export function nextUpcomingEvent(events: CalendarEvent[], now: Date): CalendarEvent | null {
  for (const event of events) {
    if (event.all_day || !event.start_at) continue;
    const start = new Date(event.start_at);
    if (!Number.isNaN(start.getTime()) && start > now) return event;
  }
  return null;
}

/** Short "what needs you" items, most urgent first. Empty = all clear. */
export function attentionItems(data: BriefingData, now: Date): string[] {
  const items: string[] = [];

  const next = nextUpcomingEvent(data.todayEvents, now);
  if (next?.start_at) {
    const start = new Date(next.start_at);
    const minutes = Math.round((start.getTime() - now.getTime()) / 60_000);
    const when = minutes <= 90 ? `in ${minutes} min` : `at ${timeFormat.format(start)}`;
    items.push(`${next.summary ?? "An event"} ${when}`);
  }

  if (data.dueCardCount > 0) {
    items.push(`${data.dueCardCount} study card${data.dueCardCount === 1 ? "" : "s"} due for review`);
  }

  const target = data.dietTargets?.calories;
  if (target) {
    const remaining = Math.round(target - data.dietTotals.calories);
    if (remaining > 0 && now.getHours() >= 17) {
      items.push(`${remaining} kcal left of today's ${target} target`);
    }
  }

  return items;
}
