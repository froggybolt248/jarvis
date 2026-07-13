import { Tile } from "../../components/ui/Tile";
import { BookIcon } from "../../components/icons";
import { useDueCards } from "../study/useDueCards";
import { cn } from "../../lib/cn";

export interface StudyTileProps {
  className?: string;
}

export function StudyTile({ className }: StudyTileProps) {
  const { loading, error, cards } = useDueCards();
  const empty = !loading && !error && cards.length === 0;
  const next = cards[0];

  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2">
        <span className="text-dom-study">
          <BookIcon size={16} />
        </span>
        <span className="text-xs font-medium uppercase tracking-wide text-ink-dim">Study</span>
      </div>

      {loading ? (
        <div className="h-4 w-2/3 animate-pulse rounded-pill bg-surface-2" />
      ) : empty ? (
        <p className="text-sm text-ink-faint">Nothing due right now.</p>
      ) : (
        <p className="text-sm text-ink">
          {cards.length} card{cards.length === 1 ? "" : "s"} due
          {next ? <span className="text-ink-faint"> • next: {next.front}</span> : null}
        </p>
      )}
    </Tile>
  );
}
