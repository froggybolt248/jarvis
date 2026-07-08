import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  /**
   * `ghost` — quiet, text-only, hairline on hover.
   * `accent` — primary CTA pill, the amber accent's one interactive use.
   */
  variant?: "ghost" | "accent";
}

const base =
  "inline-flex items-center justify-center gap-1.5 whitespace-nowrap text-sm font-medium transition-colors duration-150 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent disabled:opacity-40 disabled:pointer-events-none";

const variants: Record<NonNullable<ButtonProps["variant"]>, string> = {
  ghost:
    "rounded-tile px-3 py-1.5 text-ink-dim border border-transparent hover:border-hairline hover:bg-surface-2 hover:text-ink",
  accent:
    "rounded-pill px-4 py-1.5 bg-accent text-canvas hover:bg-accent/90",
};

export function Button({ children, className, variant = "ghost", ...props }: ButtonProps) {
  return (
    <button className={cn(base, variants[variant], className)} {...props}>
      {children}
    </button>
  );
}
