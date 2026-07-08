import { briefing } from "../../mock/briefing";
import { cn } from "../../lib/cn";
import { Button } from "../../components/ui/Button";
import { MicIcon } from "../../components/icons";
import { useUiStore } from "../../state/ui";

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
 * in the shell stays flat.
 */
export function HeroCard({ className }: HeroCardProps) {
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);

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

      <div className="relative flex h-full flex-col justify-between gap-6 border border-hairline bg-surface-1/40 p-6 backdrop-blur-xl">
        <div className="flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">{dateLabel}</span>
            <h1 className="text-xl font-semibold tracking-tight text-ink">{briefing.greeting}</h1>
          </div>
          <Button variant="accent" onClick={() => setPaletteOpen(true)}>
            <MicIcon size={16} />
            Ask Jarvis
          </Button>
        </div>

        <div className="flex flex-col gap-1.5">
          {briefing.lines.map((line) => (
            <p key={line} className="text-sm leading-relaxed text-ink-dim">
              {line}
            </p>
          ))}
        </div>

        <div className="flex flex-col gap-2">
          <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">Needs attention</span>
          <ul className="flex flex-col gap-1.5">
            {briefing.attention.map((item) => (
              <li key={item} className="flex items-start gap-2 text-sm text-ink">
                <span className="mt-1.5 h-1 w-1 shrink-0 rounded-full bg-accent" aria-hidden="true" />
                {item}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}
