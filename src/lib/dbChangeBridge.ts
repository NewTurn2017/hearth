import { useEffect } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const DB_CHANGE_EVENTS = [
  "projects:changed",
  "memos:changed",
  "schedules:changed",
  "categories:changed",
] as const;

/**
 * Rust can detect writes made by other SQLite connections (for example the
 * `hearth` CLI or Codex skill) and emits Tauri events. The React data hooks
 * intentionally subscribe to DOM CustomEvents so in-app mutations can keep
 * using a lightweight browser-local bus. This bridge connects those two
 * buses once at app boot.
 */
export function useTauriDbChangeBridge(): void {
  useEffect(() => {
    let cancelled = false;
    const unlisteners: UnlistenFn[] = [];

    for (const eventName of DB_CHANGE_EVENTS) {
      void listen(eventName, () => {
        if (cancelled) return;
        window.dispatchEvent(new CustomEvent(eventName));
      })
        .then((unlisten) => {
          if (cancelled) {
            unlisten();
          } else {
            unlisteners.push(unlisten);
          }
        })
        .catch(() => {
          // Browser-only dev/test contexts do not expose Tauri's event bridge.
          // Existing DOM CustomEvents still keep in-app mutations reactive.
        });
    }

    return () => {
      cancelled = true;
      for (const unlisten of unlisteners.splice(0)) {
        unlisten();
      }
    };
  }, []);
}
