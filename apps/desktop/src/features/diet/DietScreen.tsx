import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { calories, macros } from "../../mock/diet";

export function DietScreen() {
  return (
    <div className="flex flex-col gap-6 p-8">
      <h1 className="text-lg font-semibold tracking-tight text-ink">Diet</h1>
      <Card className="max-w-xl">
        <h2 className="mb-3 text-sm font-medium text-ink-dim">Today</h2>
        <div className="flex flex-col">
          <div className="flex items-center justify-between gap-4">
            <span className="text-sm text-ink">Calories</span>
            <span className="text-xs text-ink-dim">
              {calories.consumed} <span className="text-ink-faint">/ {calories.target} kcal</span>
            </span>
          </div>
          {macros.map((macro) => (
            <div key={macro.label}>
              <Divider className="my-3" />
              <div className="flex items-center justify-between gap-4">
                <span className="text-sm text-ink">{macro.label}</span>
                <span className="text-xs text-ink-dim">
                  {macro.grams}g <span className="text-ink-faint">/ {macro.target}g</span>
                </span>
              </div>
            </div>
          ))}
        </div>
      </Card>
    </div>
  );
}
