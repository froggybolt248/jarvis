import { CalendarIcon, DumbbellIcon, FlameIcon, BookIcon, BrainIcon } from "../../../components/icons";
import { DomainCard } from "../DomainCard";

const DOMAINS = [
  { id: "calendar", icon: CalendarIcon, title: "Calendar", description: "Schedule & reminders" },
  { id: "diet", icon: FlameIcon, title: "Diet", description: "Macros & meal logging" },
  { id: "gym", icon: DumbbellIcon, title: "Gym", description: "Workouts & progression" },
  { id: "study", icon: BookIcon, title: "Study", description: "MechE tutor & flashcards" },
  { id: "knowledge", icon: BrainIcon, title: "Knowledge", description: "Chat over your notes" },
] as const;

export interface WelcomeStepProps {
  domains: string[];
  onToggleDomain: (id: string) => void;
}

export function WelcomeStep({ domains, onToggleDomain }: WelcomeStepProps) {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold tracking-tight text-ink">Welcome to Jarvis</h1>
        <p className="text-sm leading-relaxed text-ink-dim">
          Your local-first assistant. Everything stays on this machine.
        </p>
      </div>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
        {DOMAINS.map((d) => (
          <DomainCard
            key={d.id}
            icon={d.icon}
            title={d.title}
            description={d.description}
            selected={domains.includes(d.id)}
            onToggle={() => onToggleDomain(d.id)}
          />
        ))}
      </div>

      <p className="text-xs text-ink-faint">
        {domains.length === 0
          ? "Pick at least one to continue — you can change this later in Settings."
          : `${domains.length} selected · change anytime in Settings.`}
      </p>
    </div>
  );
}
