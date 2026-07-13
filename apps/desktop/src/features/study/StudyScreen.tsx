import { useState } from "react";
import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { Button } from "../../components/ui/Button";
import { BookIcon } from "../../components/icons";
import { ipc, type SrsCard } from "../../lib/ipc";
import { useDueCards } from "./useDueCards";

const inputClass =
  "rounded-tile border border-hairline bg-surface-2 px-2 py-1 text-sm text-ink placeholder:text-ink-faint focus:outline-none focus:ring-1 focus:ring-accent";

function NewCardForm({ onCreated }: { onCreated: () => void }) {
  const [front, setFront] = useState("");
  const [back, setBack] = useState("");
  const [saving, setSaving] = useState(false);

  const submit = async () => {
    const f = front.trim();
    const b = back.trim();
    if (!f || !b || saving) return;
    setSaving(true);
    try {
      await ipc.studyCreateCard({ front: f, back: b });
      setFront("");
      setBack("");
      onCreated();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-col gap-2">
      <input
        className={inputClass}
        placeholder="Front"
        value={front}
        onChange={(e) => setFront(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <input
        className={inputClass}
        placeholder="Back"
        value={back}
        onChange={(e) => setBack(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <Button
        variant="accent"
        className="self-start"
        onClick={submit}
        disabled={!front.trim() || !back.trim() || saving}
      >
        Add
      </Button>
    </div>
  );
}

const reviewOptions: { label: string; quality: number }[] = [
  { label: "Again", quality: 1 },
  { label: "Hard", quality: 3 },
  { label: "Good", quality: 4 },
  { label: "Easy", quality: 5 },
];

function ReviewRow({ card, onReviewed }: { card: SrsCard; onReviewed: () => void }) {
  const [saving, setSaving] = useState(false);

  const review = async (quality: number) => {
    if (saving) return;
    setSaving(true);
    try {
      await ipc.studyReviewCard(card.id, quality);
      onReviewed();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex items-center gap-1.5">
      {reviewOptions.map((opt) => (
        <Button key={opt.label} onClick={() => review(opt.quality)} disabled={saving} className="px-2 py-1 text-xs">
          {opt.label}
        </Button>
      ))}
    </div>
  );
}

function LoadingCard() {
  return (
    <Card>
      <div className="mb-3 h-4 w-32 animate-pulse rounded-pill bg-surface-2" />
      <div className="flex flex-col gap-3">
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
        <div className="h-4 w-full animate-pulse rounded-pill bg-surface-2" />
      </div>
    </Card>
  );
}

export function StudyScreen() {
  const { loading, error, cards, refetch } = useDueCards();

  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <div className="flex items-center gap-2.5">
          <span className="rounded-tile bg-dom-study/15 p-1.5 text-dom-study">
            <BookIcon size={18} />
          </span>
          <h1 className="text-2xl font-semibold tracking-tight text-ink">Study</h1>
        </div>

        <div className="grid gap-4 lg:grid-cols-[2fr_1fr]">
          {loading ? (
            <LoadingCard />
          ) : (
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">Spaced repetition</h2>

              {error ? (
                <p className="text-sm text-ink-faint">Couldn't reach the backend — try again shortly.</p>
              ) : cards.length === 0 ? (
                <div className="flex flex-col items-center gap-2 py-6 text-center">
                  <span className="rounded-tile bg-dom-study/15 p-1.5 text-dom-study">
                    <BookIcon size={20} />
                  </span>
                  <p className="text-sm text-ink-dim">Nothing due right now.</p>
                  <p className="text-xs text-ink-faint">
                    Add cards through Jarvis and they'll surface here as they're due.
                  </p>
                </div>
              ) : (
                <div className="flex flex-col">
                  {cards.map((card, i) => (
                    <div key={card.id}>
                      {i > 0 ? <Divider className="my-3" /> : null}
                      <div className="flex items-center justify-between gap-4">
                        <div className="flex flex-col gap-0.5">
                          <span className="text-sm text-ink">{card.front}</span>
                          <span className="text-xs text-ink-faint">
                            {card.repetitions > 0 ? `Reviewed ${card.repetitions}× · ` : ""}
                            due {new Date(card.due_at).toLocaleString()}
                          </span>
                        </div>
                        <ReviewRow card={card} onReviewed={refetch} />
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </Card>
          )}

          <div className="flex flex-col gap-4">
            <Card>
              <h2 className="mb-3 text-sm font-medium text-ink-dim">New card</h2>
              <NewCardForm onCreated={refetch} />
            </Card>
            <Card className="flex flex-col gap-2">
              <h2 className="text-sm font-medium text-ink-dim">Due now</h2>
              <span className="text-3xl font-semibold tabular-nums text-ink">{cards.length}</span>
              <p className="text-xs text-ink-faint">card{cards.length === 1 ? "" : "s"} waiting for review</p>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
