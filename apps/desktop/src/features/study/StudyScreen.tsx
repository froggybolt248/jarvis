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
    <div className="flex items-center gap-2">
      <input
        className={`${inputClass} flex-1`}
        placeholder="Front"
        value={front}
        onChange={(e) => setFront(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <input
        className={`${inputClass} flex-1`}
        placeholder="Back"
        value={back}
        onChange={(e) => setBack(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && submit()}
      />
      <Button variant="accent" onClick={submit} disabled={!front.trim() || !back.trim() || saving}>
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
  const { loading, error, cards, refetch } = useDueCards();

  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Study</h1>

      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">New card</h2>
        <NewCardForm onCreated={refetch} />
      </Card>

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
    </div>
  );
}
