import { useEffect, type ComponentType } from "react";
import { Command } from "cmdk";
import { AnimatePresence, motion } from "motion/react";
import { useUiStore } from "../../state/ui";
import { useNavStore, VIEWS, type View } from "../../state/nav";
import { Kbd } from "../../components/ui/Kbd";
import {
  SearchIcon,
  CalendarIcon,
  DumbbellIcon,
  FlameIcon,
  BookIcon,
  BrainIcon,
  SettingsIcon,
  SparkleIcon,
  PlusIcon,
  CheckIcon,
  type IconProps,
} from "../../components/icons";

const viewMeta: Record<View, { label: string; icon: ComponentType<IconProps> }> = {
  today: { label: "Today", icon: SparkleIcon },
  calendar: { label: "Calendar", icon: CalendarIcon },
  diet: { label: "Diet", icon: FlameIcon },
  gym: { label: "Gym", icon: DumbbellIcon },
  study: { label: "Study", icon: BookIcon },
  knowledge: { label: "Knowledge", icon: BrainIcon },
  settings: { label: "Settings", icon: SettingsIcon },
};

const actions = [
  { id: "log-meal", label: "Log a meal", icon: PlusIcon },
  { id: "start-workout", label: "Start workout", icon: PlusIcon },
  { id: "review-cards", label: "Review cards", icon: CheckIcon },
];

const itemClass =
  "flex cursor-default items-center gap-2.5 rounded-tile px-3 py-2 text-sm text-ink-dim outline-none data-[selected=true]:bg-surface-2 data-[selected=true]:text-ink";

export function CommandPalette() {
  const open = useUiStore((s) => s.paletteOpen);
  const setOpen = useUiStore((s) => s.setPaletteOpen);
  const setView = useNavStore((s) => s.setView);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        useUiStore.setState((s) => ({ paletteOpen: !s.paletteOpen }));
      }
      if (event.key === "Escape") {
        useUiStore.setState({ paletteOpen: false });
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <AnimatePresence>
      {open ? (
        <div className="fixed inset-0 z-50 flex justify-center px-4 pt-[16vh]">
          <motion.div
            className="absolute inset-0 bg-canvas/70 backdrop-blur-sm"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.15 }}
            onClick={() => setOpen(false)}
          />
          <motion.div
            className="relative h-fit w-full max-w-lg overflow-hidden rounded-card border border-hairline bg-surface-1/85 backdrop-blur-2xl shadow-[0_32px_96px_-32px_rgba(0,0,0,0.7)]"
            initial={{ opacity: 0, y: -8, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: -8, scale: 0.98 }}
            transition={{ type: "spring", stiffness: 480, damping: 34, duration: 0.2 }}
          >
            <Command shouldFilter className="flex flex-col">
              <div className="flex items-center gap-2.5 border-b border-hairline px-4 py-3.5">
                <SearchIcon size={16} className="shrink-0 text-ink-faint" />
                <Command.Input
                  autoFocus
                  placeholder="Ask Jarvis or jump to a view…"
                  className="flex-1 bg-transparent text-sm text-ink outline-none placeholder:text-ink-faint"
                />
                <Kbd>esc</Kbd>
              </div>

              <Command.List className="max-h-80 overflow-y-auto p-2">
                <Command.Empty className="px-3 py-6 text-center text-sm text-ink-faint">
                  No results.
                </Command.Empty>

                <Command.Group
                  heading="Navigate"
                  className="px-1 pb-1 pt-2 text-[11px] font-medium uppercase tracking-wide text-ink-faint [&_[cmdk-group-items]]:mt-1.5 [&_[cmdk-group-items]]:flex [&_[cmdk-group-items]]:flex-col [&_[cmdk-group-items]]:gap-0.5"
                >
                  {VIEWS.map((view) => {
                    const meta = viewMeta[view];
                    const Icon = meta.icon;
                    return (
                      <Command.Item
                        key={view}
                        value={`navigate ${meta.label}`}
                        onSelect={() => {
                          setView(view);
                          setOpen(false);
                        }}
                        className={itemClass}
                      >
                        <Icon size={16} />
                        {meta.label}
                      </Command.Item>
                    );
                  })}
                </Command.Group>

                <Command.Separator className="my-1.5 h-px bg-hairline" />

                <Command.Group
                  heading="Actions"
                  className="px-1 pb-1 pt-2 text-[11px] font-medium uppercase tracking-wide text-ink-faint [&_[cmdk-group-items]]:mt-1.5 [&_[cmdk-group-items]]:flex [&_[cmdk-group-items]]:flex-col [&_[cmdk-group-items]]:gap-0.5"
                >
                  {actions.map((action) => {
                    const Icon = action.icon;
                    return (
                      <Command.Item
                        key={action.id}
                        value={action.label}
                        onSelect={() => setOpen(false)}
                        className={itemClass}
                      >
                        <Icon size={16} />
                        {action.label}
                      </Command.Item>
                    );
                  })}
                </Command.Group>
              </Command.List>

              <div className="flex items-center justify-end gap-1.5 border-t border-hairline px-4 py-2.5 text-xs text-ink-faint">
                <Kbd>↵</Kbd>
                to ask Jarvis
              </div>
            </Command>
          </motion.div>
        </div>
      ) : null}
    </AnimatePresence>
  );
}
