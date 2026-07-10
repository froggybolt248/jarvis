import { useEffect, useState } from "react";
import { ipc, type NoteSummary } from "../../lib/ipc";

export interface UseNotesResult {
  loading: boolean;
  error: boolean;
  notes: NoteSummary[];
}

/** Fetches vault note summaries, newest-modified first. */
export function useNotes(): UseNotesResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [notes, setNotes] = useState<NoteSummary[]>([]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    ipc
      .vaultListNotes()
      .then((n) => {
        if (!cancelled) setNotes(n);
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

  return { loading, error, notes };
}
