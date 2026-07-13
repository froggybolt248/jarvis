import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ipc } from "../../lib/ipc";
import type { NtfyConfig, ProviderHealth } from "../../lib/ipc";
import { cn } from "../../lib/cn";
import { Card } from "../../components/ui/Card";
import { Divider } from "../../components/ui/Divider";
import { Button } from "../../components/ui/Button";
import { Badge } from "../../components/ui/Badge";
import { CheckIcon, ChevronDownIcon } from "../../components/icons";

const inputClass =
  "rounded-tile border border-hairline bg-surface-0 px-3 py-2 text-sm text-ink outline-none placeholder:text-ink-faint focus-visible:ring-1 focus-visible:ring-accent disabled:opacity-40";

const GOOGLE_STEPS = [
  <>
    Open the <span className="text-ink">Google Cloud Console</span> and create (or pick) a project.
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

type ConnectStatus = "idle" | "waiting" | "listing" | "error";
type SyncStatus = "idle" | "syncing" | "done" | "error";

function GoogleSection() {
  const [connected, setConnected] = useState<boolean | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [status, setStatus] = useState<ConnectStatus>("idle");
  const [error, setError] = useState<string | null>(null);
  const [showInstructions, setShowInstructions] = useState(false);
  const [disconnecting, setDisconnecting] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>("idle");
  const [syncCount, setSyncCount] = useState<number | null>(null);

  useEffect(() => {
    ipc
      .googleStatus()
      .then(setConnected)
      .catch(() => setConnected(false));
  }, []);

  const canConnect = clientId.trim().length > 0 && clientSecret.trim().length > 0;

  async function handleConnect() {
    setError(null);
    setStatus("waiting");
    try {
      await ipc.googleConnect(clientId.trim(), clientSecret.trim());
      setStatus("listing");
      await ipc.googleListCalendars();
      setStatus("idle");
      setConnected(true);
      setClientId("");
      setClientSecret("");
    } catch (e) {
      setError(String(e));
      setStatus("error");
    }
  }

  async function handleDisconnect() {
    setDisconnecting(true);
    setError(null);
    try {
      await ipc.googleDisconnect();
      setConnected(false);
      setSyncStatus("idle");
      setSyncCount(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setDisconnecting(false);
    }
  }

  async function handleSyncNow() {
    setSyncStatus("syncing");
    try {
      const n = await ipc.calendarSyncNow();
      setSyncCount(n);
      setSyncStatus("done");
    } catch (e) {
      setError(String(e));
      setSyncStatus("error");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-ink">Google Calendar</span>
        {connected ? <Badge tone="positive">Connected</Badge> : connected === false ? <Badge>Not connected</Badge> : null}
      </div>

      {connected === null ? (
        <p className="text-xs text-ink-faint">Checking status…</p>
      ) : connected ? (
        <div className="flex flex-col gap-3">
          <div className="flex items-center gap-3">
            <Button variant="ghost" onClick={() => void handleSyncNow()} disabled={syncStatus === "syncing"}>
              {syncStatus === "syncing" ? "Syncing…" : "Sync now"}
            </Button>
            <Button variant="ghost" onClick={() => void handleDisconnect()} disabled={disconnecting}>
              {disconnecting ? "Disconnecting…" : "Disconnect"}
            </Button>
          </div>
          {syncStatus === "done" ? (
            <span className="flex items-center gap-1.5 text-xs text-positive">
              <CheckIcon size={14} />
              Synced {syncCount} event{syncCount === 1 ? "" : "s"}.
            </span>
          ) : null}
          {syncStatus === "error" && error ? <span className="text-xs text-caution">{error}</span> : null}
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <button
            type="button"
            onClick={() => setShowInstructions((v) => !v)}
            className="flex items-center gap-1.5 self-start text-xs text-ink-faint transition-colors hover:text-ink-dim"
          >
            <ChevronDownIcon
              size={12}
              className={showInstructions ? "rotate-180 transition-transform" : "transition-transform"}
            />
            How to get a Client ID and secret
          </button>
          {showInstructions ? (
            <ol className="flex flex-col gap-2 rounded-tile border border-hairline bg-surface-0 p-3 text-xs text-ink-dim">
              {GOOGLE_STEPS.map((step, i) => (
                <li key={i} className="flex gap-2">
                  <span className="text-ink-faint">{i + 1}.</span>
                  <span className="leading-relaxed">{step}</span>
                </li>
              ))}
            </ol>
          ) : null}

          <div className="flex flex-col gap-2">
            <input
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              placeholder="Client ID"
              disabled={status === "waiting" || status === "listing"}
              className={inputClass}
            />
            <input
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
              type="password"
              placeholder="Client secret"
              disabled={status === "waiting" || status === "listing"}
              className={inputClass}
            />
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="accent"
              disabled={!canConnect || status === "waiting" || status === "listing"}
              onClick={() => void handleConnect()}
            >
              {status === "waiting" ? "Waiting for browser…" : status === "listing" ? "Fetching calendars…" : "Connect"}
            </Button>
            {status === "error" && error ? <span className="text-xs text-caution">{error}</span> : null}
          </div>
        </div>
      )}
    </div>
  );
}

type TestPushStatus = "idle" | "sending" | "sent" | "error";

function NtfySection() {
  const [config, setConfig] = useState<NtfyConfig | null | undefined>(undefined);
  const [settingUp, setSettingUp] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testStatus, setTestStatus] = useState<TestPushStatus>("idle");

  useEffect(() => {
    ipc
      .ntfyGetConfig()
      .then(setConfig)
      .catch((e) => {
        setConfig(null);
        setError(String(e));
      });
  }, []);

  async function handleSetup() {
    setSettingUp(true);
    setError(null);
    try {
      const cfg = await ipc.ntfySetup();
      setConfig(cfg);
    } catch (e) {
      setError(String(e));
    } finally {
      setSettingUp(false);
    }
  }

  async function handleTest() {
    setTestStatus("sending");
    try {
      await ipc.ntfySendTest();
      setTestStatus("sent");
    } catch (e) {
      setError(String(e));
      setTestStatus("error");
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-ink">Phone push (ntfy)</span>
        {config ? <Badge tone="positive">Configured</Badge> : config === null ? <Badge>Not set up</Badge> : null}
      </div>

      {config === undefined ? (
        <p className="text-xs text-ink-faint">Checking status…</p>
      ) : config ? (
        <div className="flex flex-col gap-3">
          <p className="text-xs text-ink-faint">
            Install the ntfy app and subscribe to your private topic to get pushes on your phone.
          </p>
          <div className="flex flex-col gap-1.5 text-xs">
            <span className="text-ink-faint">Topic</span>
            <code className="truncate rounded-tile bg-surface-0 px-2 py-1 text-ink-dim">{config.topic}</code>
            <span className="text-ink-faint">Server</span>
            <code className="truncate rounded-tile bg-surface-0 px-2 py-1 text-ink-dim">{config.base_url}</code>
          </div>
          <div className="flex items-center gap-3">
            <Button variant="ghost" onClick={() => void handleTest()} disabled={testStatus === "sending"}>
              {testStatus === "sending" ? "Sending…" : "Send test notification"}
            </Button>
            {testStatus === "sent" ? (
              <span className="flex items-center gap-1.5 text-xs text-positive">
                <CheckIcon size={14} />
                Sent
              </span>
            ) : null}
            {testStatus === "error" ? <span className="text-xs text-caution">Failed to send.</span> : null}
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <p className="text-xs text-ink-faint">
            Get a private topic to receive Jarvis notifications on your phone via the ntfy app.
          </p>
          <Button variant="accent" className="self-start" onClick={() => void handleSetup()} disabled={settingUp}>
            {settingUp ? "Setting up…" : "Set up push notifications"}
          </Button>
        </div>
      )}
      {error ? <p className="text-xs text-caution">{error}</p> : null}
    </div>
  );
}

const DEFAULT_QUIET_START = "22:00";
const DEFAULT_QUIET_END = "07:00";

function QuietHoursSection() {
  const [loading, setLoading] = useState(true);
  const [enabled, setEnabled] = useState(true);
  const [start, setStart] = useState(DEFAULT_QUIET_START);
  const [end, setEnd] = useState(DEFAULT_QUIET_END);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      ipc.getSetting("quiet_hours_enabled"),
      ipc.getSetting("quiet_hours_start"),
      ipc.getSetting("quiet_hours_end"),
    ])
      .then(([enabledVal, startVal, endVal]) => {
        setEnabled(enabledVal !== "false");
        setStart(startVal ?? DEFAULT_QUIET_START);
        setEnd(endVal ?? DEFAULT_QUIET_END);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  async function handleToggle() {
    const next = !enabled;
    setEnabled(next);
    try {
      await ipc.setSetting("quiet_hours_enabled", next ? "true" : "false");
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleStartChange(value: string) {
    setStart(value);
    try {
      await ipc.setSetting("quiet_hours_start", value);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleEndChange(value: string) {
    setEnd(value);
    try {
      await ipc.setSetting("quiet_hours_end", value);
    } catch (e) {
      setError(String(e));
    }
  }

  if (loading) {
    return <p className="text-xs text-ink-faint">Loading…</p>;
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-ink">Quiet hours</span>
        <button
          type="button"
          role="switch"
          aria-checked={enabled}
          onClick={() => void handleToggle()}
          className={cn(
            "relative h-5 w-9 rounded-pill transition-colors",
            enabled ? "bg-accent" : "bg-surface-2 border border-hairline",
          )}
        >
          <span
            className={cn(
              "absolute top-0.5 h-4 w-4 rounded-full bg-canvas transition-transform",
              enabled ? "translate-x-[18px]" : "translate-x-0.5",
            )}
          />
        </button>
      </div>

      <div className="flex items-center gap-3">
        <label className="flex flex-1 flex-col gap-1.5 text-xs text-ink-faint">
          Start
          <input
            type="time"
            value={start}
            onChange={(e) => void handleStartChange(e.target.value)}
            disabled={!enabled}
            className={inputClass}
          />
        </label>
        <label className="flex flex-1 flex-col gap-1.5 text-xs text-ink-faint">
          End
          <input
            type="time"
            value={end}
            onChange={(e) => void handleEndChange(e.target.value)}
            disabled={!enabled}
            className={inputClass}
          />
        </label>
      </div>

      <p className="text-xs text-ink-faint">Notifications during this window are batched until morning.</p>
      {error ? <p className="text-xs text-caution">{error}</p> : null}
    </div>
  );
}

function VaultSection() {
  const [vaultDir, setVaultDir] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .getSetting("vault_path")
      .then((path) => {
        if (path && path.trim().length > 0) return path;
        return ipc.getDefaultVaultDir();
      })
      .then(setVaultDir)
      .catch((e) => setError(String(e)));
  }, []);

  async function handleChangeLocation() {
    setError(null);
    try {
      const dir = await open({ directory: true });
      if (typeof dir !== "string") return;
      setBusy(true);
      await ipc.setVaultDir(dir);
      setVaultDir(dir);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-ink-faint">A plain folder of Markdown files you fully own — Obsidian-compatible.</p>
      <div className="flex items-center justify-between gap-3">
        <code className="min-w-0 flex-1 truncate rounded-tile bg-surface-0 px-2.5 py-1.5 text-xs text-ink-dim">
          {vaultDir ?? "…"}
        </code>
        <Button variant="ghost" onClick={() => void handleChangeLocation()} disabled={busy}>
          Change location…
        </Button>
      </div>
      {error ? <p className="text-xs text-caution">{error}</p> : null}
    </div>
  );
}

const AUTO_MODEL_VALUE = "";

function AboutSection() {
  const [health, setHealth] = useState<ProviderHealth | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [chatModel, setChatModel] = useState(AUTO_MODEL_VALUE);
  const [modelError, setModelError] = useState<string | null>(null);

  useEffect(() => {
    ipc
      .ollamaHealth()
      .then(setHealth)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    ipc
      .getSetting("chat_model")
      .then((v) => setChatModel(v ?? AUTO_MODEL_VALUE))
      .catch((e) => setModelError(String(e)));
  }, []);

  async function handleModelChange(value: string) {
    setChatModel(value);
    try {
      await ipc.setSetting("chat_model", value);
    } catch (e) {
      setModelError(String(e));
    }
  }

  return (
    <div className="flex flex-col gap-3">
      <span className="text-sm font-medium text-ink">About</span>
      {error ? (
        <p className="text-xs text-caution">Couldn't reach the local model server — {error}</p>
      ) : health ? (
        <div className="flex flex-col gap-3 text-xs text-ink-dim">
          <div className="flex items-center justify-between">
            <span className="text-ink-faint">Ollama</span>
            <span>{health.version}</span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-ink-faint">Models installed</span>
            <div className="flex flex-wrap gap-1.5">
              {health.models.map((m) => (
                <Badge key={m}>{m}</Badge>
              ))}
            </div>
          </div>
          <label className="flex flex-col gap-1.5">
            <span className="text-ink-faint">Chat model</span>
            <select
              value={chatModel}
              onChange={(e) => void handleModelChange(e.target.value)}
              className={inputClass}
            >
              <option value={AUTO_MODEL_VALUE}>Auto</option>
              {health.models.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </label>
          {modelError ? <p className="text-xs text-caution">{modelError}</p> : null}
        </div>
      ) : (
        <p className="text-xs text-ink-faint">Checking local model server…</p>
      )}
    </div>
  );
}

export function SettingsScreen() {
  return (
    <div className="h-full overflow-y-auto p-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Settings</h1>

        <div className="grid gap-4 xl:grid-cols-2">
          <Card>
            <h2 className="mb-4 text-sm font-medium text-ink-dim">Connections</h2>
            <GoogleSection />
            <Divider className="my-4" />
            <NtfySection />
          </Card>

          <div className="flex flex-col gap-4">
            <Card>
              <h2 className="mb-4 text-sm font-medium text-ink-dim">Notifications</h2>
              <QuietHoursSection />
            </Card>

            <Card>
              <h2 className="mb-4 text-sm font-medium text-ink-dim">Vault</h2>
              <VaultSection />
            </Card>

            <Card>
              <AboutSection />
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
