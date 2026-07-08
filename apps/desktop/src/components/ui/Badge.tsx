import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  children: ReactNode;
  tone?: "neutral" | "accent" | "positive";
}

const tones: Record<NonNullable<BadgeProps["tone"]>, string> = {
  neutral: "bg-surface-2 text-ink-dim",
  accent: "bg-accent/15 text-accent",
  positive: "bg-positive/15 text-positive",
};

/** Subtle inline label. Never a raw red dot — status is conveyed by text + tone. */
export function Badge({ children, className, tone = "neutral", ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-pill px-2 py-0.5 text-xs font-medium leading-none",
        tones[tone],
        className,
      )}
      {...props}
    >
      {children}
    </span>
  );
}
