import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  getQuickCaptureShortcut,
  getQuickCaptureShortcutError,
  rebindQuickCaptureShortcut,
} from "../api";

function pretty(combo: string): string {
  const isMac = navigator.userAgent.includes("Mac");
  return combo
    .replace("CommandOrControl", isMac ? "⌘" : "Ctrl")
    .replace("Shift", "⇧")
    .replace("Alt", "⌥")
    .replace(/\+/g, "")
    .toUpperCase();
}

export function useQuickCaptureShortcut() {
  const [combo, setCombo] = useState<string>("");
  const [error, setError] = useState<string>("");

  const reload = useCallback(async () => {
    const [c, e] = await Promise.all([
      getQuickCaptureShortcut(),
      getQuickCaptureShortcutError().catch(() => ""),
    ]);
    setCombo(c);
    setError(e);
  }, []);

  useEffect(() => {
    void reload();
    let off: UnlistenFn | null = null;
    listen<string>("quick-capture-shortcut:changed", (e) => {
      setCombo(e.payload);
      setError("");
    }).then((f) => (off = f));
    return () => {
      if (off) off();
    };
  }, [reload]);

  const rebind = useCallback(async (next: string) => {
    const saved = await rebindQuickCaptureShortcut(next);
    setCombo(saved);
    setError("");
    return saved;
  }, []);

  return { combo, display: combo ? pretty(combo) : "", error, rebind, reload };
}
