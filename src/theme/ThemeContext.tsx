// Theme state lifted into React context. Call `setTheme` to flip the
// appearance — it (1) mutates the DOM immediately, (2) updates localStorage
// for FOUC-free reloads, (3) persists to the Rust settings table, and
// (4) emits a Tauri event so the Quick Capture window reacts too.

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";
import { emit } from "@tauri-apps/api/event";
import type { ThemeSetting } from "./types";
import { DEFAULT_THEME } from "./types";
import { applyTheme } from "./applyTheme";
import * as api from "../api";

export const THEME_LS_KEY = "hearth.theme";
export const THEME_EVENT = "theme-changed";

type ThemeContextValue = {
  theme: ThemeSetting;
  setTheme: (next: ThemeSetting) => Promise<void>;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);

function readCache(): ThemeSetting | null {
  try {
    const raw = localStorage.getItem(THEME_LS_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as ThemeSetting;
  } catch {
    return null;
  }
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<ThemeSetting>(
    () => readCache() ?? DEFAULT_THEME,
  );

  // Reconcile with Rust-side settings on mount. If the backend disagrees with
  // the cache (e.g. user changed theme in another instance), re-apply and
  // update the cache.
  useEffect(() => {
    let alive = true;
    api
      .getTheme()
      .then((fresh) => {
        if (!alive || !fresh) return;
        if (JSON.stringify(fresh) !== JSON.stringify(theme)) {
          applyTheme(fresh);
          localStorage.setItem(THEME_LS_KEY, JSON.stringify(fresh));
          setThemeState(fresh);
        }
      })
      .catch(() => {
        /* backend unavailable; keep cached theme */
      });
    return () => {
      alive = false;
    };
    // run once on mount only — re-checking on every theme change would bounce
    // back to cache values.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setTheme = useCallback(async (next: ThemeSetting) => {
    applyTheme(next);
    localStorage.setItem(THEME_LS_KEY, JSON.stringify(next));
    setThemeState(next);
    try {
      await api.setTheme(next);
      await emit(THEME_EVENT, next);
    } catch (e) {
      // Persistence failed — leave the UI flipped, surface to console. The
      // next boot will fall back to the localStorage cache anyway.
      console.error("setTheme persist failed:", e);
    }
  }, []);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme(): ThemeContextValue {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error("useTheme used outside ThemeProvider");
  return ctx;
}
