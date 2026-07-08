import { create } from "zustand";

interface UiState {
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
  feedExpanded: boolean;
  setFeedExpanded: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  paletteOpen: false,
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  feedExpanded: false,
  setFeedExpanded: (feedExpanded) => set({ feedExpanded }),
}));
