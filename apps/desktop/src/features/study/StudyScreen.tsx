import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { studyStatus } from "../../mock/study";

export function StudyScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Study</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">Spaced repetition</h2>
        <div className="flex flex-col">
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">Cards due</span>
            <span className="text-xs text-ink-dim">{studyStatus.cardsDue}</span>
          </div>
          <Divider className="my-3" />
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">Upcoming deadline</span>
            <span className="text-xs text-ink-dim">{studyStatus.nextDeadline}</span>
          </div>
        </div>
      </Card>
    </div>
  );
}
