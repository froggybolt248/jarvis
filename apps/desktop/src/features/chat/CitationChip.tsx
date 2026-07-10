import type { Citation } from "../../lib/ipc";

export interface CitationChipProps {
  citation: Citation;
}

/** A single numbered source reference beneath a streamed answer. */
export function CitationChip({ citation }: CitationChipProps) {
  return (
    <span className="inline-flex max-w-[240px] items-center gap-1.5 rounded-tile border border-hairline bg-surface-2 px-2 py-1 text-xs text-ink-dim">
      <span
        className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-surface-3 text-[10px] font-medium text-ink-faint"
        aria-hidden="true"
      >
        {citation.index}
      </span>
      <span className="truncate">
        {citation.source_path}
        {citation.heading ? <span className="text-ink-faint"> · {citation.heading}</span> : null}
      </span>
    </span>
  );
}
