import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { Divider } from "../../components/ui/Divider";
import { BrainIcon, MicIcon, ClockIcon } from "../../components/icons";
import { useUiStore } from "../../state/ui";
import { useNotes } from "./useNotes";
import { formatRelativeTime } from "./format";

function LoadingCard() {
  return (
    <Card className="max-w-xl">
      <div className="mb-3 h-4 w-32 animate-pulse rounded-pill bg-surface-2" />
      <div className="flex flex-col gap-3">
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
      </div>
    </Card>
  );
}

/**
 * Notes are read-only here (there's no note-viewer route yet), so clicking
 * one just opens the palette — a calm way to ask Jarvis about it rather than
 * a dead click.
 */
export function KnowledgeScreen() {
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const { loading, error, notes } = useNotes();

  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Knowledge</h1>

      {loading ? (
        <LoadingCard />
      ) : (
        <Card className="max-w-xl">
          <h2 className="mb-3 text-sm font-medium text-ink-dim">Your notes</h2>

          {error ? (
            <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
          ) : notes.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-6 text-center">
              <BrainIcon size={20} className="text-ink-faint" />
              <p className="text-sm text-ink-dim">Your notes will appear here as you write them.</p>
            </div>
          ) : (
            <div className="flex flex-col">
              {notes.map((note) => (
                <div key={note.path}>
                  <Divider className="my-3" />
                  <button
                    type="button"
                    onClick={() => setPaletteOpen(true)}
                    className="flex w-full flex-col items-start gap-0.5 text-left"
                  >
                    <span className="text-sm text-ink">{note.title}</span>
                    <span className="flex items-center gap-1.5 text-xs text-ink-faint">
                      <span className="truncate">{note.path}</span>
                      <span aria-hidden="true">·</span>
                      <ClockIcon size={12} />
                      {formatRelativeTime(note.modified)}
                    </span>
                  </button>
                </div>
              ))}
            </div>
          )}
        </Card>
      )}

      <Card className="max-w-xl">
        <div className="flex flex-col items-center gap-3 py-8 text-center">
          <BrainIcon size={20} className="text-ink-faint" />
          <p className="text-sm text-ink-dim">Your vault lives in your notes, not a list here.</p>
          <p className="max-w-sm text-xs text-ink-faint">
            Ask Jarvis about anything you've written and it'll search your notes and cite what it finds.
          </p>
          <Button variant="accent" onClick={() => setPaletteOpen(true)} className="mt-1">
            <MicIcon size={16} />
            Ask Jarvis
          </Button>
        </div>
      </Card>
    </div>
  );
}
