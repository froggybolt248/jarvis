import { Tile } from "../../components/ui/Tile";
import { BookIcon } from "../../components/icons";
import { studyStatus } from "../../mock/study";
import { cn } from "../../lib/cn";

export interface StudyTileProps {
  className?: string;
}

export function StudyTile({ className }: StudyTileProps) {
  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <BookIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Study</span>
      </div>
      <p className="text-sm text-ink">
        {studyStatus.cardsDue} cards due
        <span className="text-ink-faint"> • {studyStatus.nextDeadline}</span>
      </p>
    </Tile>
  );
}
