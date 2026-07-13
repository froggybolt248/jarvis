import { Component, type ErrorInfo, type ReactNode } from "react";
import { Button } from "../components/ui/Button";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
}

const MAX_MESSAGE_LENGTH = 300;

function truncate(message: string): string {
  if (message.length <= MAX_MESSAGE_LENGTH) return message;
  return `${message.slice(0, MAX_MESSAGE_LENGTH)}…`;
}

/**
 * Top-level crash guard: an uncaught render/effect error must never blank the
 * whole app onto the near-black canvas background with no explanation. Wraps
 * <App/> in main.tsx. Also best-effort reports the error to the file-log
 * bridge so a crash leaves a trace under the app log dir.
 */
export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Best-effort: never let logging itself throw and mask the original error.
    void import("@tauri-apps/plugin-log")
      .then(({ error: logError }) =>
        logError(`React render error: ${error.stack ?? error.message}\n${info.componentStack ?? ""}`),
      )
      .catch(() => {});
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;

    const message = truncate(error.stack ?? error.message ?? String(error));

    return (
      <div className="flex h-screen w-screen items-center justify-center bg-canvas p-6">
        <div className="flex w-full max-w-md flex-col gap-3 rounded-card border border-hairline bg-surface-1 p-7">
          <h1 className="text-base font-semibold text-ink">Jarvis hit an error</h1>
          <p className="whitespace-pre-wrap text-sm text-ink-dim">{message}</p>
          <Button
            variant="accent"
            className="mt-2 self-start"
            onClick={() => window.location.reload()}
          >
            Reload
          </Button>
        </div>
      </div>
    );
  }
}
