import { create } from "zustand";
export type Theme = "dark" | "light";
function initialTheme(): Theme {
  try {
    return localStorage.getItem("meetagain-theme") === "dark"
      ? "dark"
      : "light";
  } catch {
    return "light";
  }
}
interface ViewState {
  theme: Theme;
  tick: number;
  playing: boolean;
  selectedAgent: number | null;
  focusPoint: [number, number] | null;
  toggleTheme: () => void;
  setTick: (tick: number) => void;
  setPlaying: (playing: boolean) => void;
  selectAgent: (id: number | null) => void;
  focusMap: (coordinates: [number, number]) => void;
  resetView: () => void;
}
export const useView = create<ViewState>((set) => ({
  theme: initialTheme(),
  tick: 0,
  playing: false,
  selectedAgent: null,
  focusPoint: null,
  toggleTheme: () =>
    set((state) => {
      const theme = state.theme === "dark" ? "light" : "dark";
      try {
        localStorage.setItem("meetagain-theme", theme);
      } catch {
        /* Storage is optional. */
      }
      return { theme };
    }),
  setTick: (tick) => set({ tick }),
  setPlaying: (playing) => set({ playing }),
  selectAgent: (selectedAgent) => set({ selectedAgent }),
  focusMap: (coordinates) => set({ focusPoint: [...coordinates] }),
  resetView: () =>
    set({ tick: 0, playing: false, selectedAgent: null, focusPoint: null }),
}));
