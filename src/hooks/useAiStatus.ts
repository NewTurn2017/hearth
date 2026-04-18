// Simple AI status — reflects whether an OpenAI key is stored. No polling,
// no server lifecycle. Updates via the "ai-settings:changed" event.
import { useEffect, useState } from "react";
import * as api from "../api";

export type AiStatus = "configured" | "missing" | "unknown";

export function useAiStatus(): AiStatus {
  const [status, setStatus] = useState<AiStatus>("unknown");

  useEffect(() => {
    let cancelled = false;

    const refresh = async () => {
      try {
        const s = await api.getAiSettings();
        if (!cancelled) setStatus(s.has_openai_key ? "configured" : "missing");
      } catch {
        // Leave whatever was last seen.
      }
    };

    refresh();
    window.addEventListener("ai-settings:changed", refresh);
    return () => {
      cancelled = true;
      window.removeEventListener("ai-settings:changed", refresh);
    };
  }, []);

  return status;
}
