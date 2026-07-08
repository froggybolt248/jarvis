import type { ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { XIcon } from "../icons";
import { IconButton } from "./IconButton";
import { cn } from "../../lib/cn";

export interface SheetProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  side?: "left" | "right";
  className?: string;
}

/** Side panel that slides in from an edge over a scrim. */
export function Sheet({ open, onClose, title, children, side = "right", className }: SheetProps) {
  return (
    <AnimatePresence>
      {open ? (
        <div className="fixed inset-0 z-40 flex">
          <motion.div
            className="absolute inset-0 bg-canvas/60"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            onClick={onClose}
          />
          <motion.div
            className={cn(
              "relative ml-auto flex h-full w-[360px] flex-col border-l border-hairline bg-surface-1 p-5",
              side === "left" && "ml-0 mr-auto border-l-0 border-r",
              className,
            )}
            initial={{ x: side === "left" ? -24 : 24, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: side === "left" ? -24 : 24, opacity: 0 }}
            transition={{ type: "spring", stiffness: 420, damping: 38, duration: 0.2 }}
          >
            <div className="mb-4 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-ink">{title}</h2>
              <IconButton label="Close" onClick={onClose}>
                <XIcon size={16} />
              </IconButton>
            </div>
            <div className="flex-1 overflow-y-auto">{children}</div>
          </motion.div>
        </div>
      ) : null}
    </AnimatePresence>
  );
}
