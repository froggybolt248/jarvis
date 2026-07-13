import { HeroCard } from "./HeroCard";
import { CalendarTile } from "./CalendarTile";
import { DietTile } from "./DietTile";
import { GymTile } from "./GymTile";
import { StudyTile } from "./StudyTile";
import { KnowledgeTile } from "./KnowledgeTile";

const gridTemplate = {
  gridTemplateAreas: '"hero hero cal" "hero hero diet" "gym study knowledge"',
  gridTemplateRows: "auto auto auto",
  gridTemplateColumns: "1fr 1fr 1fr",
};

export function TodayScreen() {
  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto grid w-full max-w-5xl grid-cols-3 gap-4" style={gridTemplate}>
        <HeroCard className="[grid-area:hero]" />
        <CalendarTile className="[grid-area:cal]" />
        <DietTile className="[grid-area:diet]" />
        <GymTile className="[grid-area:gym]" />
        <StudyTile className="[grid-area:study]" />
        <KnowledgeTile className="[grid-area:knowledge]" />
      </div>
    </div>
  );
}
