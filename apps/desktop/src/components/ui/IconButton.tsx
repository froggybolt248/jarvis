import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  /** Marks the icon as the active/selected state (e.g. current nav item). */
  active?: boolean;
  label: string;
}

export function IconButton({ children, className, active = false, label, ...props }: IconButtonProps) {
  return (
    <button
      aria-label={label}
      className={cn(
        "relative inline-flex h-9 w-9 items-center justify-center rounded-tile text-ink-dim transition-colors duration-150 hover:bg-surface-2 hover:text-ink focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent",
        active && "bg-surface-2 text-ink",
        className,
      )}
      {...props}
    >
      {children}
    </button>
  );
}
