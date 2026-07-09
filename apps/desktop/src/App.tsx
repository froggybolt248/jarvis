import { useEffect, useState } from "react";
import { AppShell } from "./app/AppShell";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { ipc } from "./lib/ipc";

type Boot =
  | { status: "loading" }
  | { status: "onboarding" }
  | { status: "ready" };

/**
 * Root boot gate. On launch we ask the Rust core whether onboarding has ever
 * completed; first run shows the wizard, every subsequent run goes straight to
 * the dashboard. If the query fails we fail *open* to onboarding (safer than
 * dropping a not-yet-set-up user into an empty dashboard).
 */
function App() {
  const [boot, setBoot] = useState<Boot>({ status: "loading" });

  useEffect(() => {
    ipc
      .getOnboardingState()
      .then((s) => setBoot({ status: s.complete ? "ready" : "onboarding" }))
      .catch(() => setBoot({ status: "onboarding" }));
  }, []);

  if (boot.status === "loading") return <BootSplash />;
  if (boot.status === "onboarding")
    return <OnboardingFlow onComplete={() => setBoot({ status: "ready" })} />;
  return <AppShell />;
}

function BootSplash() {
  return (
    <div className="flex h-screen w-screen items-center justify-center bg-canvas">
      <div className="h-1.5 w-1.5 animate-ping rounded-pill bg-accent" />
    </div>
  );
}

export default App;
