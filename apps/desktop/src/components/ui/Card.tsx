import type { HTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Flat, static surface container. Depth comes from the hairline + surface step, never a shadow. */
export function Card({ children, className, ...props }: CardProps) {
  return (
    <div
      className={cn(
        "rounded-card border border-hairline bg-surface-1 p-5",
        className,
      )}
      {...props}
    >
      {children}
    </div>
  );
}
