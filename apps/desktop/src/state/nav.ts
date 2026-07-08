import { create } from "zustand";

export const VIEWS = ["today", "calendar", "diet", "gym", "study", "knowledge", "settings"] as const;

export type View = (typeof VIEWS)[number];

interface NavState {
  view: View;
  setView: (view: View) => void;
}

export const useNavStore = create<NavState>((set) => ({
  view: "today",
  setView: (view) => set({ view }),
}));
