import { useEffect, useState } from "react";
import { ipc, type SrsCard } from "../../lib/ipc";

export interface UseDueCardsResult {
  loading: boolean;
  error: boolean;
  cards: SrsCard[];
}

/** Fetches SRS cards due right now, soonest first. */
export function useDueCards(): UseDueCardsResult {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [cards, setCards] = useState<SrsCard[]>([]);

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
  }, []);

  return { loading, error, cards };
}
