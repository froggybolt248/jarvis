import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { Sparkline } from "../../components/ui/Sparkline";
import { gymStatus, sessionTrend } from "../../mock/gym";

export function GymScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Gym</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">Training</h2>
        <div className="flex flex-col">
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">{gymStatus.scheduled}</span>
          </div>
          <Divider className="my-3" />
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">Last session</span>
            <span className="text-xs text-ink-dim">{gymStatus.lastSession}</span>
          </div>
          <Divider className="my-3" />
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">Last 7 sessions</span>
            <Sparkline values={sessionTrend} />
          </div>
        </div>
      </Card>
    </div>
  );
}
