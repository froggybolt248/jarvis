import { useEffect, useState } from "react";
import { ipc, type DietLog, type DietTargets } from "../../lib/ipc";

/** Local (not UTC) `YYYY-MM-DD` for "today", matching how the user thinks about their day. */
function todayDateString(): string {
  const d = new Date();
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export interface DietTotals {
  calories: number;
  protein_g: number;
  carbs_g: number;
  fat_g: number;
}

/** Sums the numeric fields across today's logs, treating missing values as 0. */
export function sumDietLogs(logs: DietLog[]): DietTotals {
  return logs.reduce(
    (acc, log) => ({
      calories: acc.calories + (log.calories ?? 0),
      protein_g: acc.protein_g + (log.protein_g ?? 0),
      carbs_g: acc.carbs_g + (log.carbs_g ?? 0),
      fat_g: acc.fat_g + (log.fat_g ?? 0),
    }),
    { calories: 0, protein_g: 0, carbs_g: 0, fat_g: 0 },
  );
}

export interface UseDietTodayResult {
  loading: boolean;
  error: boolean;
  logs: DietLog[];
  targets: DietTargets | null;
  totals: DietTotals;
}

/** Fetches today's diet logs + the current targets from the backend. */
export function useDietToday(): UseDietTodayResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [logs, setLogs] = useState<DietLog[]>([]);
  const [targets, setTargets] = useState<DietTargets | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    Promise.all([ipc.dietLogsForDate(todayDateString()), ipc.dietCurrentTargets()])
      .then(([l, t]) => {
        if (cancelled) return;
        setLogs(l);
        setTargets(t);
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
  }, []);

  return { loading, error, logs, targets, totals: sumDietLogs(logs) };
}
