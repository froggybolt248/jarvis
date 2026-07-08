import { useState } from "react";
import { Tile } from "../../components/ui/Tile";
import { Button } from "../../components/ui/Button";
import { Sheet } from "../../components/ui/Sheet";
import { Divider } from "../../components/ui/Divider";
import { BrainIcon } from "../../components/icons";
import { knowledgeStatus, recentNotes } from "../../mock/knowledge";
import { cn } from "../../lib/cn";

export interface KnowledgeTileProps {
  className?: string;
}

export function KnowledgeTile({ className }: KnowledgeTileProps) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Tile className={cn("flex flex-col gap-3", className)}>
        <div className="flex items-center gap-2 text-ink-dim">
          <BrainIcon size={16} />
          <span className="text-xs font-medium uppercase tracking-wide">Knowledge</span>
        </div>
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm text-ink">{knowledgeStatus.notesTouchedThisWeek} notes touched this week</p>
          <Button variant="ghost" onClick={() => setOpen(true)} className="shrink-0">
            View all
          </Button>
        </div>
      </Tile>

      <Sheet open={open} onClose={() => setOpen(false)} title="Recently touched notes">
        <ul className="flex flex-col">
          {recentNotes.map((note, i) => (
            <li key={note.id}>
              {i > 0 ? <Divider className="my-3" /> : null}
              <div className="flex flex-col gap-0.5">
                <span className="text-sm text-ink">{note.title}</span>
                <span className="text-xs text-ink-faint">{note.updated}</span>
              </div>
            </li>
          ))}
        </ul>
      </Sheet>
    </>
  );
}
