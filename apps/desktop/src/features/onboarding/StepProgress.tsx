import { cn } from "../../lib/cn";

export interface StepProgressProps {
  steps: readonly string[];
  current: number;
  className?: string;
}

/** Persistent 4-segment progress bar + "Step N of M · Label" caption. */
export function StepProgress({ steps, current, className }: StepProgressProps) {
  return (
    <div className={cn("flex flex-col gap-2.5", className)}>
      <div className="flex items-center gap-1.5">
        {steps.map((label, i) => (
          <div
            key={label}
            className={cn(
              "h-1 flex-1 rounded-pill transition-colors duration-300",
              i <= current ? "bg-accent" : "bg-surface-3",
            )}
          />
        ))}
      </div>
      <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">
        Step {current + 1} of {steps.length} · {steps[current]}
      </span>
    </div>
  );
}
