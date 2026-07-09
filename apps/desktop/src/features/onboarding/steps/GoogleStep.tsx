import { useState } from "react";
import { ipc } from "../../../lib/ipc";
import { Button } from "../../../components/ui/Button";
import { Divider } from "../../../components/ui/Divider";
import { CheckIcon } from "../../../components/icons";

type Status = "idle" | "waiting" | "listing" | "connected" | "error";

export interface GoogleStepProps {
  onSkip: () => void;
}

const STEPS = [
  <>
    Open the{" "}
    <span className="text-ink">Google Cloud Console</span> and create (or pick) a project.
  </>,
  <>
    Under <span className="text-ink">APIs &amp; Services → Library</span>, enable the{" "}
    <span className="text-ink">Google Calendar API</span>.
  </>,
  <>
    Go to <span className="text-ink">APIs &amp; Services → Credentials → Create Credentials → OAuth client ID</span>.
  </>,
  <>
    Choose application type <span className="text-ink">Desktop app</span>, then create it.
  </>,
  <>Copy the generated Client ID and Client secret into the fields below.</>,
];

export function GoogleStep({ onSkip }: GoogleStepProps) {
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const [calendarCount, setCalendarCount] = useState<number | null>(null);

  const canConnect = clientId.trim().length > 0 && clientSecret.trim().length > 0;

  async function handleConnect() {
    setError(null);
    setStatus("waiting");
    try {
      await ipc.googleConnect(clientId.trim(), clientSecret.trim());
      setStatus("listing");
      const calendars = await ipc.googleListCalendars();
      setCalendarCount(calendars.length);
      setStatus("connected");
    } catch (e) {
      setError(String(e));
      setStatus("error");
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Connect Google Calendar</h1>
        <p className="text-sm leading-relaxed text-ink-dim">
          Optional — Jarvis can read and schedule events for you. This uses your own Google
          credentials; nothing is shared with anyone but Google.
        </p>
      </div>

      <ol className="flex flex-col gap-2 rounded-card border border-hairline bg-surface-1 p-4 text-sm text-ink-dim">
        {STEPS.map((step, i) => (
          <li key={i} className="flex gap-2.5">
            <span className="text-ink-faint">{i + 1}.</span>
            <span className="leading-relaxed">{step}</span>
          </li>
        ))}
      </ol>

      <div className="flex flex-col gap-3">
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-ink-dim">Client ID</span>
          <input
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            placeholder="xxxxx.apps.googleusercontent.com"
            disabled={status === "waiting" || status === "listing"}
            className="rounded-tile border border-hairline bg-surface-0 px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint focus-visible:ring-1 focus-visible:ring-accent"
          />
        </label>
        <label className="flex flex-col gap-1.5 text-sm">
          <span className="text-ink-dim">Client secret</span>
          <input
            value={clientSecret}
            onChange={(e) => setClientSecret(e.target.value)}
            type="password"
            placeholder="GOCSPX-…"
            disabled={status === "waiting" || status === "listing"}
            className="rounded-tile border border-hairline bg-surface-0 px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint focus-visible:ring-1 focus-visible:ring-accent"
          />
        </label>
      </div>

      {status === "connected" ? (
        <div className="flex items-center gap-2.5 rounded-tile border border-hairline bg-surface-1 px-3.5 py-2.5 text-sm text-positive">
          <CheckIcon size={16} />
          Connected — {calendarCount} calendar{calendarCount === 1 ? "" : "s"} found.
        </div>
      ) : (
        <div className="flex items-center gap-3">
          <Button variant="accent" disabled={!canConnect || status === "waiting" || status === "listing"} onClick={() => void handleConnect()}>
            {status === "waiting" ? "Waiting for browser…" : status === "listing" ? "Fetching calendars…" : "Connect"}
          </Button>
          {status === "error" && error ? <span className="text-xs text-caution">{error}</span> : null}
        </div>
      )}

      <Divider />

      <div className="flex items-center justify-between">
        <p className="text-xs text-ink-faint">You can connect Google Calendar later in Settings.</p>
        <Button variant="ghost" onClick={onSkip}>
          Skip for now
        </Button>
      </div>
    </div>
  );
}
