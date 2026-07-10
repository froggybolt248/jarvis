import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { BookIcon } from "../../components/icons";
import { useDueCards } from "./useDueCards";

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

export function StudyScreen() {
  const { loading, error, cards } = useDueCards();

  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Study</h1>

      {loading ? (
        <LoadingCard />
      ) : (
        <Card className="max-w-xl">
          <h2 className="mb-3 text-sm font-medium text-ink-dim">Spaced repetition</h2>

          {error ? (
            <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
          ) : cards.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-6 text-center">
              <BookIcon size={20} className="text-ink-faint" />
              <p className="text-sm text-ink-dim">Nothing due right now.</p>
              <p className="text-xs text-ink-faint">Add cards through Jarvis and they'll surface here as they're due.</p>
            </div>
          ) : (
            <div className="flex flex-col">
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-ink">Cards due</span>
                <span className="text-xs text-ink-dim">{cards.length}</span>
              </div>
              {cards.map((card) => (
                <div key={card.id}>
                  <Divider className="my-3" />
                  <div className="flex flex-col gap-0.5">
                    <span className="text-sm text-ink">{card.front}</span>
                    <span className="text-xs text-ink-faint">
                      {card.repetitions > 0 ? `Reviewed ${card.repetitions}× · ` : ""}
                      due {new Date(card.due_at).toLocaleString()}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </Card>
      )}
    </div>
  );
}
