// Lightweight poller for the MLX server state so UI chrome (e.g. TopBar pill)
// can show liveness without coupling to the chat flow in `useAi`. Polls every
// 5s — cheap because the backend cached `probe_alive` result amounts to one
// HTTP GET against 127.0.0.1.
import { useEffect, useState } from "react";
import type { AiServerState } from "../types";
import * as api from "../api";

const POLL_INTERVAL_MS = 5000;

export function useAiStatus(): AiServerState {
  const [state, setState] = useState<AiServerState>({ kind: "idle" });

  useEffect(() => {
    let cancelled = false;

    const tick = async () => {
      try {
        const next = await api.aiServerStatus();
        if (!cancelled) setState(next);
      } catch {
        // Backend unreachable — keep last known state. Tauri reconnect will heal.
      }
    };

    tick();
    const id = setInterval(tick, POLL_INTERVAL_MS);
    // Trigger an immediate re-probe when either the settings change or a
    // start/stop action completes, so the pill does not lag behind by up to
    // POLL_INTERVAL_MS waiting for the next tick.
    const onChange = () => tick();
    window.addEventListener("ai-settings:changed", onChange);
    window.addEventListener("ai-server:changed", onChange);
    return () => {
      cancelled = true;
      clearInterval(id);
      window.removeEventListener("ai-settings:changed", onChange);
      window.removeEventListener("ai-server:changed", onChange);
    };
  }, []);

  return state;
}
