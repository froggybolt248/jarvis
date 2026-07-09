import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import QRCode from "qrcode";
import { ipc } from "../../../lib/ipc";
import { Button } from "../../../components/ui/Button";
import { Divider } from "../../../components/ui/Divider";
import { CheckIcon } from "../../../components/icons";

type TestPushStatus = "idle" | "sending" | "sent" | "error";

export function VaultPushStep() {
  const [vaultDir, setVaultDir] = useState<string | null>(null);
  const [vaultBusy, setVaultBusy] = useState(false);
  const [vaultError, setVaultError] = useState<string | null>(null);

  const [topic, setTopic] = useState<string | null>(null);
  const [subscribeUrl, setSubscribeUrl] = useState<string | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [ntfyError, setNtfyError] = useState<string | null>(null);
  const [testStatus, setTestStatus] = useState<TestPushStatus>("idle");

  useEffect(() => {
    ipc
      .getDefaultVaultDir()
      .then((def) => setVaultDir(def))
      .catch((e) => setVaultError(String(e)));

    ipc
      .ntfySetup()
      .then(async (cfg) => {
        const url = `${cfg.base_url}/${cfg.topic}`;
        setTopic(cfg.topic);
        setSubscribeUrl(url);
        const dataUrl = await QRCode.toDataURL(url, { margin: 1, width: 200 });
        setQrDataUrl(dataUrl);
      })
      .catch((e) => setNtfyError(String(e)));
  }, []);

  async function handleChangeLocation() {
    setVaultError(null);
    try {
      const dir = await open({ directory: true });
      if (typeof dir !== "string") return;
      setVaultBusy(true);
      await ipc.setVaultDir(dir);
      setVaultDir(dir);
    } catch (e) {
      setVaultError(String(e));
    } finally {
      setVaultBusy(false);
    }
  }

  async function handleTestPush() {
    setTestStatus("sending");
    try {
      await ipc.ntfySendTest();
      setTestStatus("sent");
    } catch (e) {
      setNtfyError(String(e));
      setTestStatus("error");
    }
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Vault &amp; phone push</h1>
        <p className="text-sm leading-relaxed text-ink-dim">Both optional — sensible defaults are already in place.</p>
      </div>

      <div className="flex flex-col gap-3 rounded-card border border-hairline bg-surface-1 p-4">
        <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">Vault location</span>
        <p className="text-xs text-ink-faint">
          A plain folder of Markdown files you fully own — Obsidian-compatible.
        </p>
        <div className="flex items-center justify-between gap-3">
          <code className="truncate rounded-tile bg-surface-0 px-2.5 py-1.5 text-xs text-ink-dim">
            {vaultDir ?? "…"}
          </code>
          <Button variant="ghost" onClick={() => void handleChangeLocation()} disabled={vaultBusy}>
            Change location…
          </Button>
        </div>
        {vaultError ? <p className="text-xs text-caution">{vaultError}</p> : null}
      </div>

      <Divider />

      <div className="flex flex-col gap-3 rounded-card border border-hairline bg-surface-1 p-4">
        <span className="text-xs font-medium uppercase tracking-wide text-ink-faint">Phone push (ntfy)</span>
        <p className="text-xs text-ink-faint">
          Install the ntfy app, then scan this to subscribe to your private topic.
        </p>
        <div className="flex items-center gap-4">
          <div className="flex h-[104px] w-[104px] shrink-0 items-center justify-center overflow-hidden rounded-tile border border-hairline bg-surface-0">
            {qrDataUrl ? (
              <img src={qrDataUrl} alt="ntfy subscribe QR code" className="h-full w-full" />
            ) : (
              <span className="text-xs text-ink-faint">…</span>
            )}
          </div>
          <div className="flex min-w-0 flex-col gap-1.5 text-xs">
            <span className="text-ink-faint">Topic</span>
            <code className="truncate rounded-tile bg-surface-0 px-2 py-1 text-ink-dim">{topic ?? "…"}</code>
            <span className="truncate text-ink-faint">{subscribeUrl}</span>
          </div>
        </div>
        <div className="flex items-center gap-3">
          <Button variant="ghost" onClick={() => void handleTestPush()} disabled={!topic || testStatus === "sending"}>
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
        {ntfyError ? <p className="text-xs text-caution">{ntfyError}</p> : null}
      </div>
    </div>
  );
}
