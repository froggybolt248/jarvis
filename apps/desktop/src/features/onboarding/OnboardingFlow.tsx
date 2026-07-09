import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { ipc } from "../../lib/ipc";
import { Button } from "../../components/ui/Button";
import { ChevronLeftIcon, ChevronRightIcon } from "../../components/icons";
import { StepProgress } from "./StepProgress";
import { WelcomeStep } from "./steps/WelcomeStep";
import { OllamaStep } from "./steps/OllamaStep";
import { GoogleStep } from "./steps/GoogleStep";
import { VaultPushStep } from "./steps/VaultPushStep";

const STEP_LABELS = ["Welcome", "Local AI", "Google Calendar", "Vault & Push"] as const;

export interface OnboardingFlowProps {
  onComplete: () => void;
}

/** First-run wizard: pick domains, set up local AI, optionally connect Google, place the vault. */
export function OnboardingFlow({ onComplete }: OnboardingFlowProps) {
  const [step, setStep] = useState(0);
  const [direction, setDirection] = useState(1);
  const [domains, setDomains] = useState<string[]>([]);
  const [ollamaReady, setOllamaReady] = useState(false);
  const [finishing, setFinishing] = useState(false);
  const [finishError, setFinishError] = useState<string | null>(null);

  const lastStep = STEP_LABELS.length - 1;
  const canAdvance = step === 0 ? domains.length > 0 : step === 1 ? ollamaReady : true;

  function toggleDomain(id: string) {
    setDomains((prev) => (prev.includes(id) ? prev.filter((d) => d !== id) : [...prev, id]));
  }

  function goTo(next: number) {
    setDirection(next > step ? 1 : -1);
    setStep(next);
  }

  async function handleFinish() {
    setFinishError(null);
    setFinishing(true);
    try {
      await ipc.completeOnboarding(domains);
      onComplete();
    } catch (e) {
      setFinishError(String(e));
      setFinishing(false);
    }
  }

  function handleNext() {
    if (step < lastStep) {
      goTo(step + 1);
    } else {
      void handleFinish();
    }
  }

  return (
    <div className="relative flex h-screen w-screen items-center justify-center overflow-hidden bg-canvas px-6">
      <div className="pointer-events-none absolute inset-0" aria-hidden="true">
        <div
          className="animate-mesh-drift-a absolute -left-32 -top-32 h-[480px] w-[480px] rounded-full opacity-[0.12] blur-3xl"
          style={{ background: "var(--color-mesh-amber)" }}
        />
        <div
          className="animate-mesh-drift-b absolute -bottom-32 -right-24 h-[480px] w-[480px] rounded-full opacity-[0.14] blur-3xl"
          style={{ background: "var(--color-mesh-blue)" }}
        />
      </div>

      <div className="relative flex w-full max-w-[600px] flex-col gap-6 rounded-card border border-hairline bg-surface-1/90 p-8 backdrop-blur-xl">
        <StepProgress steps={STEP_LABELS} current={step} />

        <div className="relative min-h-[360px] overflow-hidden">
          <AnimatePresence mode="wait" custom={direction} initial={false}>
            <motion.div
              key={step}
              custom={direction}
              initial={{ opacity: 0, x: direction * 24 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: direction * -24 }}
              transition={{ type: "spring", stiffness: 420, damping: 38 }}
            >
              {step === 0 && <WelcomeStep domains={domains} onToggleDomain={toggleDomain} />}
              {step === 1 && <OllamaStep onReadyChange={setOllamaReady} />}
              {step === 2 && <GoogleStep onSkip={() => goTo(3)} />}
              {step === 3 && <VaultPushStep />}
            </motion.div>
          </AnimatePresence>
        </div>

        <div className="flex items-center justify-between border-t border-hairline pt-5">
          <Button variant="ghost" onClick={() => goTo(step - 1)} disabled={step === 0 || finishing}>
            <ChevronLeftIcon size={16} />
            Back
          </Button>

          <div className="flex items-center gap-3">
            {finishError ? <span className="text-xs text-caution">{finishError}</span> : null}
            <Button variant="accent" onClick={handleNext} disabled={!canAdvance || finishing}>
              {step === lastStep ? (finishing ? "Entering Jarvis…" : "Enter Jarvis") : "Next"}
              {step < lastStep ? <ChevronRightIcon size={16} /> : null}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
