import { Tile } from "../../components/ui/Tile";
import { Button } from "../../components/ui/Button";
import { BrainIcon } from "../../components/icons";
import { useUiStore } from "../../state/ui";
import { cn } from "../../lib/cn";
import { useNotes } from "../knowledge/useNotes";

export interface KnowledgeTileProps {
  className?: string;
}

/**
 * Shows the vault's note count + a couple of recent titles when available,
 * falling back to the quiet "ask Jarvis" invitation while loading, on
 * error, or when the vault is empty.
 */
export function KnowledgeTile({ className }: KnowledgeTileProps) {
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const { loading, error, notes } = useNotes();

  const showList = !loading && !error && notes.length > 0;

  return (
    <Tile className={cn("flex flex-col gap-3", className)}>
      <div className="flex items-center gap-2 text-ink-dim">
        <BrainIcon size={16} />
        <span className="text-xs font-medium uppercase tracking-wide">Knowledge</span>
        {showList && <span className="text-xs text-ink-faint">{notes.length}</span>}
      </div>

      {showList ? (
        <div className="flex flex-col gap-1">
          {notes.slice(0, 3).map((note) => (
            <span key={note.path} className="truncate text-sm text-ink-dim">
              {note.title}
            </span>
          ))}
        </div>
      ) : (
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm text-ink-faint">Ask Jarvis about your notes</p>
          <Button variant="ghost" onClick={() => setPaletteOpen(true)} className="shrink-0">
            Ask
          </Button>
        </div>
      )}
    </Tile>
  );
}
