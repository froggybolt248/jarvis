import { useEffect, useState } from "react";
import { ipc, type GymSession } from "../../lib/ipc";

export interface UseGymSessionsResult {
  loading: boolean;
  error: boolean;
  sessions: GymSession[];
  /** Session durations in minutes, oldest to newest, for sessions that have ended. */
  durationTrend: number[];
}

/** Duration in whole minutes, or `null` if the session hasn't ended yet. */
export function sessionDurationMinutes(session: GymSession): number | null {
  if (!session.ended_at) return null;
  const start = new Date(session.started_at).getTime();
  const end = new Date(session.ended_at).getTime();
  if (Number.isNaN(start) || Number.isNaN(end) || end < start) return null;
  return Math.round((end - start) / 60000);
}

/** Fetches the most recent gym sessions (newest first). */
export function useGymSessions(limit: number): UseGymSessionsResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [sessions, setSessions] = useState<GymSession[]>([]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    ipc
      .gymRecentSessions(limit)
      .then((s) => {
        if (!cancelled) setSessions(s);
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
  }, [limit]);

  const durationTrend = sessions
    .map(sessionDurationMinutes)
    .filter((v): v is number => v !== null)
    .reverse();

  return { loading, error, sessions, durationTrend };
}
