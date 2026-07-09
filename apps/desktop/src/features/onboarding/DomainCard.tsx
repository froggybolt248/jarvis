import type { ComponentType, KeyboardEvent } from "react";
import { Tile } from "../../components/ui/Tile";
import { CheckIcon, type IconProps } from "../../components/icons";
import { cn } from "../../lib/cn";

export interface DomainCardProps {
  icon: ComponentType<IconProps>;
  title: string;
  description: string;
  selected: boolean;
  onToggle: () => void;
}

/** Multi-select bento tile for a domain, used on the welcome step. */
export function DomainCard({ icon: Icon, title, description, selected, onToggle }: DomainCardProps) {
  function onKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onToggle();
    }
  }

  return (
    <Tile
      role="button"
      tabIndex={0}
      aria-pressed={selected}
      onClick={onToggle}
      onKeyDown={onKeyDown}
      className={cn(
        "flex cursor-pointer flex-col gap-3",
        selected && "border-accent/50 bg-surface-2",
      )}
    >
      <div className="flex items-center justify-between">
        <Icon size={20} className={selected ? "text-accent" : "text-ink-dim"} />
        {selected ? <CheckIcon size={14} className="text-accent" /> : null}
      </div>
      <div className="flex flex-col gap-0.5">
        <p className="text-sm font-medium text-ink">{title}</p>
        <p className="text-xs text-ink-faint">{description}</p>
      </div>
    </Tile>
  );
}
