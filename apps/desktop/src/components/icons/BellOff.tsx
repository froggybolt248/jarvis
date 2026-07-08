import type { IconProps } from "./types";

export function BellOffIcon({ size = 16, className }: IconProps) {
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
      <path d="M18 15.5V11a6 6 0 0 0-4.7-5.86" />
      <path d="M8.3 6.1A6 6 0 0 0 6 11v4.5c0 .9-.4 1.8-1 2.5h11" />
      <path d="M10 19a2 2 0 0 0 4 0" />
      <path d="M4 4l16 16" />
    </svg>
  );
}
