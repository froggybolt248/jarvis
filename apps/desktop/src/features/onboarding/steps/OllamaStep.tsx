import { useCallback, useEffect, useRef, useState } from "react";
import { ipc, type OllamaStatus, type PullProgress, type RecommendedModels } from "../../../lib/ipc";
import { Button } from "../../../components/ui/Button";
import { Badge } from "../../../components/ui/Badge";
import { CheckIcon } from "../../../components/icons";
import { formatBytes } from "../format";
import { cn } from "../../../lib/cn";

type Phase =
  | "checking"
  | "need-install"
  | "installing"
  | "starting"
  | "pulling"
  | "ready"
  | "error";

export interface OllamaStepProps {
  onReadyChange: (ready: boolean) => void;
}

const EMBED_MODEL = "nomic-embed-text";

/** Small spinning ring, used while a phase is in flight. */
function Spinner({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-block h-3.5 w-3.5 animate-spin rounded-full border-[1.5px] border-hairline-strong border-t-accent",
        className,
      )}
      aria-hidden="true"
    />
  );
}

export function OllamaStep({ onReadyChange }: OllamaStepProps) {
  const [phase, setPhase] = useState<Phase>("checking");
  const [rec, setRec] = useState<RecommendedModels | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pullQueue, setPullQueue] = useState<string[]>([]);
  const [pullIndex, setPullIndex] = useState(0);
  const [progress, setProgress] = useState<PullProgress | null>(null);

  // Tracks the last-known-good recommendation so a Retry from the
  // "need-install" phase doesn't have to re-fetch it.
  const recRef = useRef<RecommendedModels | null>(null);

  const pullMissing = useCallback(
    async (recommended: RecommendedModels, status: OllamaStatus) => {
      const wanted = Array.from(new Set([...recommended.models, EMBED_MODEL]));
      const need = wanted.filter((m) => !status.models.includes(m));

      if (need.length === 0) {
        setPhase("ready");
        onReadyChange(true);
        return;
      }

      setPhase("pulling");
      setPullQueue(need);

      for (let i = 0; i < need.length; i++) {
        setPullIndex(i);
        setProgress(null);
        const model = need[i];
        const unlisten = await ipc.onOllamaPullProgress((p) => {
          if (p.model === model) setProgress(p);
        });
        try {
          await ipc.ollamaPull(model);
        } finally {
          unlisten();
        }
      }

      setPhase("ready");
      onReadyChange(true);
    },
    [onReadyChange],
  );

  const continueAfterInstall = useCallback(
    async (recommended: RecommendedModels) => {
      setPhase("starting");
      await ipc.ollamaEnsureRunning();
      const status = await ipc.ollamaDetect();
      await pullMissing(recommended, status);
    },
    [pullMissing],
  );

  const runSetup = useCallback(async () => {
    setError(null);
    setPhase("checking");
    onReadyChange(false);
    try {
      const recommended = await ipc.ollamaRecommendedModels();
      recRef.current = recommended;
      setRec(recommended);

      const status = await ipc.ollamaDetect();
      if (!status.installed) {
        setPhase("need-install");
        return;
      }
      await continueAfterInstall(recommended);
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  }, [continueAfterInstall, onReadyChange]);

  useEffect(() => {
    void runSetup();
    // Intentionally run once on mount; re-entering this step re-checks
    // cheaply and skips any models already pulled.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleInstall() {
    setError(null);
    setPhase("installing");
    try {
      await ipc.ollamaInstall();
      const status = await ipc.ollamaDetect();
      if (!status.installed) {
        setError("Install finished, but Ollama still isn't detected. Try again.");
        setPhase("error");
        return;
      }
      const recommended = recRef.current;
      if (!recommended) {
        setError("Lost track of the recommended models — please retry.");
        setPhase("error");
        return;
      }
      await continueAfterInstall(recommended);
    } catch (e) {
      setError(String(e));
      setPhase("error");
    }
  }

  const chatModel = rec?.models[0];
  const currentPullModel = pullQueue[pullIndex];
  const percent = progress?.percent ?? 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Setting up local AI</h1>
        <p className="text-sm leading-relaxed text-ink-dim">
          Jarvis runs entirely on your machine via Ollama — no cloud, no API keys.
        </p>
      </div>

      {rec ? (
        <div className="flex items-center gap-2 rounded-tile border border-hairline bg-surface-1 px-3.5 py-2.5">
          <Badge tone="accent">{rec.ram_gb} GB RAM</Badge>
          <span className="text-sm text-ink-dim">
            using <span className="text-ink">{chatModel}</span>
          </span>
        </div>
      ) : null}

      <div className="flex flex-col gap-3 rounded-card border border-hairline bg-surface-1 p-4">
        {phase === "checking" && (
          <div className="flex items-center gap-2.5 text-sm text-ink-dim">
            <Spinner />
            Checking your machine…
          </div>
        )}

        {phase === "need-install" && (
          <div className="flex flex-col gap-3">
            <p className="text-sm text-ink-dim">
              Ollama isn't installed yet. Jarvis can install it for you (via winget).
            </p>
            <Button variant="accent" onClick={handleInstall} className="self-start">
              Install Ollama
            </Button>
          </div>
        )}

        {phase === "installing" && (
          <div className="flex items-center gap-2.5 text-sm text-ink-dim">
            <Spinner />
            Installing Ollama — this can take a minute…
          </div>
        )}

        {phase === "starting" && (
          <div className="flex items-center gap-2.5 text-sm text-ink-dim">
            <Spinner />
            Starting the local model server…
          </div>
        )}

        {phase === "pulling" && (
          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-ink">
                Downloading <span className="font-medium">{currentPullModel}</span>
              </span>
              <span className="text-ink-faint">
                {pullIndex + 1} of {pullQueue.length}
              </span>
            </div>
            <div className="h-1.5 w-full overflow-hidden rounded-pill bg-surface-3">
              <div
                className="h-full rounded-pill bg-accent transition-[width] duration-300 ease-out"
                style={{ width: `${Math.max(2, percent)}%` }}
              />
            </div>
            <div className="flex items-center justify-between text-xs text-ink-faint">
              <span>
                {progress ? `${formatBytes(progress.completed)} / ${formatBytes(progress.total)}` : "Starting…"}
              </span>
              <span>{percent.toFixed(0)}%</span>
            </div>
            <p className="text-xs text-ink-faint">
              These are multi-gigabyte downloads — feel free to leave this running.
            </p>
          </div>
        )}

        {phase === "ready" && (
          <div className="flex items-center gap-2.5 text-sm text-positive">
            <CheckIcon size={16} />
            Local AI is ready.
          </div>
        )}

        {phase === "error" && (
          <div className="flex flex-col gap-3">
            <p className="text-sm text-caution">{error ?? "Something went wrong."}</p>
            <Button variant="ghost" onClick={() => void runSetup()} className="self-start">
              Retry
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
