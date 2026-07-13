import { cn } from "../../lib/cn";

export interface SparklineProps {
  values: number[];
  width?: number;
  height?: number;
  className?: string;
  /** Color for the trailing point marker. Defaults to the accent token. */
  color?: string;
}

/** Tiny inline trend line. Purely decorative context, not a chart to read precisely. */
export function Sparkline({ values, width = 64, height = 24, className, color = "var(--color-accent)" }: SparklineProps) {
  if (values.length < 2) return null;

  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = max - min || 1;
  const step = width / (values.length - 1);

  const points = values
    .map((v, i) => {
      const x = i * step;
      const y = height - ((v - min) / range) * height;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={cn("overflow-visible", className)}
      aria-hidden="true"
    >
      <polyline points={points} fill="none" stroke="var(--color-ink-faint)" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
      <circle
        cx={(values.length - 1) * step}
        cy={height - ((values[values.length - 1] - min) / range) * height}
        r={2}
        fill={color}
      />
    </svg>
  );
}
