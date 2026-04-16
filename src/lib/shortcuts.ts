// src/lib/shortcuts.ts
import { useEffect } from "react";

export function isMac() {
  return /Mac|iPod|iPhone|iPad/.test(navigator.platform);
}

/** Listen for ⌘K / Ctrl+K globally. Calls `handler` with no arg. */
export function useCmdK(handler: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = isMac() ? e.metaKey : e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        handler();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}
