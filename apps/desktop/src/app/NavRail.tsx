import type { ComponentType } from "react";
import { useNavStore, type View } from "../state/nav";
import { Tooltip } from "../components/ui/Tooltip";
import {
  CalendarIcon,
  DumbbellIcon,
  FlameIcon,
  BookIcon,
  BrainIcon,
  SettingsIcon,
  SparkleIcon,
  type IconProps,
} from "../components/icons";

interface NavItem {
  view: View;
  label: string;
  icon: ComponentType<IconProps>;
}

const topItems: NavItem[] = [
  { view: "today", label: "Today", icon: SparkleIcon },
  { view: "calendar", label: "Calendar", icon: CalendarIcon },
  { view: "diet", label: "Diet", icon: FlameIcon },
  { view: "gym", label: "Gym", icon: DumbbellIcon },
  { view: "study", label: "Study", icon: BookIcon },
  { view: "knowledge", label: "Knowledge", icon: BrainIcon },
];

const bottomItems: NavItem[] = [{ view: "settings", label: "Settings", icon: SettingsIcon }];

function NavButton({ item }: { item: NavItem }) {
  const view = useNavStore((s) => s.view);
  const setView = useNavStore((s) => s.setView);
  const active = view === item.view;
  const Icon = item.icon;

  return (
    <Tooltip label={item.label} side="right">
      <button
        aria-label={item.label}
        aria-current={active ? "page" : undefined}
        onClick={() => setView(item.view)}
        className="relative flex h-9 w-9 items-center justify-center rounded-tile text-ink-dim transition-colors duration-150 hover:bg-surface-2 hover:text-ink data-[active=true]:text-ink"
        data-active={active}
      >
        {active ? (
          <span className="absolute -left-2 h-4 w-[2px] rounded-full bg-accent" aria-hidden="true" />
        ) : null}
        <Icon size={18} />
      </button>
    </Tooltip>
  );
}

export function NavRail() {
  return (
    <nav className="flex h-full w-14 shrink-0 flex-col items-center justify-between border-r border-hairline bg-surface-0 py-4">
      <div className="flex flex-col items-center gap-1.5">
        {topItems.map((item) => (
          <NavButton key={item.view} item={item} />
        ))}
      </div>
      <div className="flex flex-col items-center gap-1.5">
        {bottomItems.map((item) => (
          <NavButton key={item.view} item={item} />
        ))}
      </div>
    </nav>
  );
}
