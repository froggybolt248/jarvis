import type { IconProps } from "./types";

export function BrainIcon({ size = 16, className }: IconProps) {
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
      <path d="M9.5 3.5a2.8 2.8 0 0 0-2.8 2.8v.2a2.6 2.6 0 0 0-1.7 4.4 2.7 2.7 0 0 0 .5 4.6 2.7 2.7 0 0 0 2.6 3.5 2.5 2.5 0 0 0 1.4-.4" />
      <path d="M9.5 3.5c1.1 0 2 .9 2 2v11.6a2.4 2.4 0 0 1-2 2.4" />
      <path d="M14.5 3.5a2.8 2.8 0 0 1 2.8 2.8v.2a2.6 2.6 0 0 1 1.7 4.4 2.7 2.7 0 0 1-.5 4.6 2.7 2.7 0 0 1-2.6 3.5 2.5 2.5 0 0 1-1.4-.4" />
      <path d="M14.5 3.5c-1.1 0-2 .9-2 2v11.6a2.4 2.4 0 0 0 2 2.4" />
    </svg>
  );
}
