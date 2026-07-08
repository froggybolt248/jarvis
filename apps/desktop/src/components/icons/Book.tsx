import type { IconProps } from "./types";

export function BookIcon({ size = 16, className }: IconProps) {
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
      <path d="M4 4.5h6a2.5 2.5 0 0 1 2.5 2.5v12.5A2 2 0 0 0 10.5 17H4Z" />
      <path d="M20 4.5h-6a2.5 2.5 0 0 0-2.5 2.5v12.5a2 2 0 0 1 2-2H20Z" />
    </svg>
  );
}
