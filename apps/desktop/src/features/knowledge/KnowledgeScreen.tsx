import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { recentNotes } from "../../mock/knowledge";

export function KnowledgeScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Knowledge</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">Recently touched</h2>
        <div className="flex flex-col">
          {recentNotes.map((note, i) => (
            <div key={note.id}>
              {i > 0 ? <Divider className="my-3" /> : null}
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-ink">{note.title}</span>
                <span className="shrink-0 text-xs text-ink-faint">{note.updated}</span>
              </div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
