import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface KbdProps extends HTMLAttributes<HTMLElement> {
  children: ReactNode;
}

export function Kbd({ children, className, ...props }: KbdProps) {
  return (
    <kbd
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-[4px] border border-hairline-strong bg-surface-2 px-1.5 font-sans text-[11px] font-medium text-ink-dim",
        className,
      )}
      {...props}
    >
      {children}
    </kbd>
  );
}
