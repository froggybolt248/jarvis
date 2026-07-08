import { useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { cn } from "../../lib/cn";

export interface TooltipProps {
  label: string;
  children: ReactNode;
  side?: "right" | "top" | "bottom";
  className?: string;
}

/** Minimal hover tooltip. Used for the icon-only nav rail. */
export function Tooltip({ label, children, side = "right", className }: TooltipProps) {
  const [visible, setVisible] = useState(false);

  const sideClasses: Record<NonNullable<TooltipProps["side"]>, string> = {
    right: "left-full top-1/2 ml-2 -translate-y-1/2",
    top: "bottom-full left-1/2 mb-2 -translate-x-1/2",
    bottom: "top-full left-1/2 mt-2 -translate-x-1/2",
  };

  return (
    <div
      className={cn("relative inline-flex", className)}
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
      onFocus={() => setVisible(true)}
      onBlur={() => setVisible(false)}
    >
      {children}
      <AnimatePresence>
        {visible ? (
          <motion.div
            role="tooltip"
            className={cn(
              "pointer-events-none absolute z-50 whitespace-nowrap rounded-[6px] border border-hairline bg-surface-3 px-2 py-1 text-xs font-medium text-ink",
              sideClasses[side],
            )}
            initial={{ opacity: 0, scale: 0.96 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.96 }}
            transition={{ duration: 0.12 }}
          >
            {label}
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
