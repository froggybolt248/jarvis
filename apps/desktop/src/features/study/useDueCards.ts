import { useCallback, useEffect, useState } from "react";
import { ipc, type SrsCard } from "../../lib/ipc";

export interface UseDueCardsResult {
  loading: boolean;
  error: boolean;
  cards: SrsCard[];
  /** Re-fetches due cards (e.g. after adding or reviewing a card). */
  refetch: () => void;
}

/** Fetches SRS cards due right now, soonest first. */
export function useDueCards(): UseDueCardsResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [cards, setCards] = useState<SrsCard[]>([]);
  const [refreshToken, setRefreshToken] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(false);
    ipc
      .studyDueCards(new Date().toISOString())
      .then((c) => {
        if (!cancelled) setCards(c);
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

  return { loading, error, cards, refetch };
}
