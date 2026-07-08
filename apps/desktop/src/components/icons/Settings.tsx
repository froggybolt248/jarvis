import type { IconProps } from "./types";

export function SettingsIcon({ size = 16, className }: IconProps) {
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
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3.5v2.2" />
      <path d="M12 18.3v2.2" />
      <path d="M20.5 12h-2.2" />
      <path d="M5.7 12H3.5" />
      <path d="M17.7 6.3l-1.55 1.55" />
      <path d="M7.85 16.15L6.3 17.7" />
      <path d="M17.7 17.7l-1.55-1.55" />
      <path d="M7.85 7.85L6.3 6.3" />
    </svg>
  );
}
