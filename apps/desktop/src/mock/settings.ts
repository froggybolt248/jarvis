/** Static mock data. Swap for real preference storage once the backend lands. */
export interface SettingRow {
  id: string;
  label: string;
  value: string;
}

export const settingsRows: SettingRow[] = [
  { id: "set-1", label: "Local model", value: "Llama 3.1 8B — on-device" },
  { id: "set-2", label: "Vault location", value: "~/Jarvis/vault" },
  { id: "set-3", label: "Quiet hours", value: "22:00 – 07:00" },
];
