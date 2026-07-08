import type { IconProps } from "./types";

export function FlameIcon({ size = 16, className }: IconProps) {
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
      <path d="M12 3c1.2 2.4-1.8 3.6-1.8 6.2a1.8 1.8 0 0 0 3.6 0c0-.6-.2-1-.5-1.4" />
      <path d="M12 21a6.5 6.5 0 0 0 6.5-6.5c0-3.2-2-5-3.4-6.6.4 2.4-1 3.6-1 5a2.1 2.1 0 0 1-4.2 0c0-.6.1-1 .3-1.5C8.4 12.7 5.5 14.3 5.5 17A6.5 6.5 0 0 0 12 21Z" />
    </svg>
  );
}
