import { cn } from "../../lib/cn";
import { Button } from "../../components/ui/Button";
import { MicIcon } from "../../components/icons";
import { useUiStore } from "../../state/ui";
import { attentionItems, greetingForHour, useBriefing } from "./useBriefing";

const dateLabel = new Intl.DateTimeFormat("en-US", {
  weekday: "long",
  month: "long",
  day: "numeric",
}).format(new Date());

export interface HeroCardProps {
  className?: string;
}

/**
 * The one glassmorphic surface in the app: a translucent card over a slow
 * ambient mesh-gradient, clipped to the card's own bounds. Everything else
 * in the shell stays flat. Content is the real briefing — today's events,
 * due cards, diet progress — composed client-side from the same reads the
 * tiles use.
 */
export function HeroCard({ className }: HeroCardProps) {
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const { loading, data } = useBriefing();

  const now = new Date();
  const eventCount = data?.todayEvents.length ?? 0;
  const attention = data ? attentionItems(data, now) : [];

  const summaryLine = data
    ? [
        eventCount === 0 ? "A clear calendar today" : `${eventCount} event${eventCount === 1 ? "" : "s"} today`,
        data.dueCardCount > 0 ? `${data.dueCardCount} card${data.dueCardCount === 1 ? "" : "s"} to review` : null,
        data.dietTotals.calories > 0 ? `${Math.round(data.dietTotals.calories)} kcal logged` : null,
      ]
        .filter(Boolean)
        .join(" · ")
    : "";

  return (
    <div className={cn("relative overflow-hidden rounded-card", className)}>
      <div className="absolute inset-0 bg-surface-1" aria-hidden="true">
        <div
          className="animate-mesh-drift-a absolute -left-16 -top-24 h-[420px] w-[420px] rounded-full opacity-30 blur-3xl"
          style={{ background: "var(--color-mesh-amber)" }}
        />
        <div
          className="animate-mesh-drift-b absolute -bottom-28 -right-10 h-[420px] w-[420px] rounded-full opacity-40 blur-3xl"
          style={{ background: "var(--color-mesh-blue)" }}
        />
      </div>

      <div className="relative flex h-full flex-col justify-between gap-6 border border-hairline bg-surface-1/40 p-7 backdrop-blur-xl">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">{dateLabel}</span>
            <h1 className="text-3xl font-semibold tracking-tight text-ink text-balance">
              {greetingForHour(now.getHours())}
            </h1>
          </div>
          <Button variant="accent" onClick={() => setPaletteOpen(true)}>
            <MicIcon size={16} />
            Ask Jarvis
          </Button>
        </div>

        {loading ? (
          <div className="flex flex-col gap-2" aria-hidden="true">
            <div className="h-3 w-64 animate-pulse rounded-pill bg-surface-2/70" />
            <div className="h-3 w-40 animate-pulse rounded-pill bg-surface-2/70" />
          </div>
        ) : (
          <p className="text-sm leading-relaxed text-ink-dim">{summaryLine}</p>
        )}

        <div className="flex flex-col gap-2">
          <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">Needs attention</span>
          {!loading && attention.length === 0 ? (
            <p className="text-sm text-ink-dim">All clear — nothing needs you right now.</p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {attention.map((item) => (
                <li key={item} className="flex items-start gap-2 text-sm text-ink">
                  <span className="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-accent" aria-hidden="true" />
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  );
}
