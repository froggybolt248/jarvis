import type { IconProps } from "./types";

export function SparkleIcon({ size = 16, className }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <path d="M12 3.5c.5 3.2 1.8 4.5 5 5-3.2.5-4.5 1.8-5 5-.5-3.2-1.8-4.5-5-5 3.2-.5 4.5-1.8 5-5Z" />
      <path d="M18.5 15.5c.25 1.5.85 2.1 2.35 2.35-1.5.25-2.1.85-2.35 2.35-.25-1.5-.85-2.1-2.35-2.35 1.5-.25 2.1-.85 2.35-2.35Z" />
    </svg>
  );
}
