import React from "react";
import ReactDOM from "react-dom/client";
import { attachConsole, error as logError } from "@tauri-apps/plugin-log";
import App from "./App";
import { ErrorBoundary } from "./app/ErrorBoundary";
import "@fontsource-variable/inter";
import "./styles/tailwind.css";

// Mirror the webview console into the Rust-side log file in dev, so a crash
// during development shows up in both places.
if (import.meta.env.DEV) {
  void attachConsole();
}

// Any error that escapes React (outside the render tree the ErrorBoundary
// covers) or an unhandled promise rejection must still leave a trace in the
// log file instead of vanishing silently.
window.onerror = (message, source, lineno, colno, err) => {
  void logError(`window.onerror: ${err?.stack ?? String(message)} (${source}:${lineno}:${colno})`).catch(() => {});
};

window.onunhandledrejection = (event) => {
  const reason = event.reason;
  const detail = reason instanceof Error ? (reason.stack ?? reason.message) : String(reason);
  void logError(`unhandledrejection: ${detail}`).catch(() => {});
};

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
