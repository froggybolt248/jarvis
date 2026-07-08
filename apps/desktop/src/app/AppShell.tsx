import type { ComponentType } from "react";
import { useNavStore, type View } from "../state/nav";
import { NavRail } from "./NavRail";
import { QuietFeed } from "./QuietFeed";
import { CommandPalette } from "../features/command-palette/CommandPalette";
import { TodayScreen } from "../features/today/TodayScreen";
import { CalendarScreen } from "../features/calendar/CalendarScreen";
import { DietScreen } from "../features/diet/DietScreen";
import { GymScreen } from "../features/gym/GymScreen";
import { StudyScreen } from "../features/study/StudyScreen";
import { KnowledgeScreen } from "../features/knowledge/KnowledgeScreen";
import { SettingsScreen } from "../features/settings/SettingsScreen";

const screens: Record<View, ComponentType> = {
  today: TodayScreen,
  calendar: CalendarScreen,
  diet: DietScreen,
  gym: GymScreen,
  study: StudyScreen,
  knowledge: KnowledgeScreen,
  settings: SettingsScreen,
};

export function AppShell() {
  const view = useNavStore((s) => s.view);
  const Screen = screens[view];

  return (
    <div className="flex h-screen w-screen bg-canvas text-ink">
      <NavRail />
      <div className="flex min-w-0 flex-1 flex-col">
        <main className="min-h-0 flex-1 overflow-hidden">
          <Screen />
        </main>
        <QuietFeed />
      </div>
      <CommandPalette />
    </div>
  );
}
