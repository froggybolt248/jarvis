import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { weekEvents } from "../../mock/calendar";

export function CalendarScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Calendar</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">This week</h2>
        <div className="flex flex-col">
          {weekEvents.map((event, i) => (
            <div key={event.id}>
              {i > 0 ? <Divider className="my-3" /> : null}
              <div className="flex items-center justify-between gap-4">
                <div className="flex flex-col gap-0.5">
                  <span className="text-sm text-ink">{event.title}</span>
                  {event.location ? <span className="text-xs text-ink-faint">{event.location}</span> : null}
                </div>
                <span className="shrink-0 text-xs text-ink-dim">{event.time}</span>
              </div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
