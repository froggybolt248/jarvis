import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { settingsRows } from "../../mock/settings";

export function SettingsScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Settings</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">General</h2>
        <div className="flex flex-col">
          {settingsRows.map((row, i) => (
            <div key={row.id}>
              {i > 0 ? <Divider className="my-3" /> : null}
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-ink">{row.label}</span>
                <span className="text-xs text-ink-dim">{row.value}</span>
              </div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
