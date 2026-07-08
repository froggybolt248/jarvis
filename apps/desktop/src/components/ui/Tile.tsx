import type { HTMLAttributes, ReactNode } from "react";
import { motion } from "motion/react";
import { cn } from "../../lib/cn";

type NonConflictingDivProps = Omit<
  HTMLAttributes<HTMLDivElement>,
  "onDrag" | "onDragStart" | "onDragEnd" | "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration"
>;

export interface TileProps extends NonConflictingDivProps {
  children: ReactNode;
}

/**
 * Bento tile. Flat surface-1 with a hairline border; on hover it lifts one
 * surface step (surface-2) and rises 2px via a short spring. No shadows.
 */
export function Tile({ children, className, ...props }: TileProps) {
  return (
    <motion.div
      className={cn(
        "rounded-tile border border-hairline bg-surface-1 p-4 transition-colors duration-150 hover:bg-surface-2",
        className,
      )}
      whileHover={{ y: -2 }}
      transition={{ type: "spring", stiffness: 420, damping: 32 }}
      {...props}
    >
      {children}
    </motion.div>
  );
}
