/** Static mock data. Swap for a real diet-tracking source once the backend lands. */
export interface Macro {
  label: string;
  grams: number;
  target: number;
  colorVar: string;
}

export const calories = { consumed: 1420, target: 2200 };

export const macros: Macro[] = [
  { label: "Protein", grams: 96, target: 150, colorVar: "var(--color-accent)" },
  { label: "Carbs", grams: 140, target: 220, colorVar: "var(--color-ink-dim)" },
  { label: "Fat", grams: 42, target: 70, colorVar: "var(--color-ink-faint)" },
];
