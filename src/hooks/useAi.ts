// Thin wrapper over the AI chat flow. No more server lifecycle — the
// backend targets OpenAI directly. Errors from a missing API key bubble
// up to the palette as a toast.
import { useCallback, useState } from "react";
import type { AgentResult, ChatMessage } from "../types";
import * as api from "../api";

interface SystemContext {
  systemPrompt: string;
}

export function useAi() {
  const [loading, setLoading] = useState(false);

  const ask = useCallback(
    async (text: string, ctx: SystemContext): Promise<AgentResult> => {
      const messages: ChatMessage[] = [
        { role: "system", content: ctx.systemPrompt },
        { role: "user", content: text },
      ];

      setLoading(true);
      try {
        return await api.aiChat(messages);
      } finally {
        setLoading(false);
      }
    },
    []
  );

  const confirm = useCallback(
    async (history: ChatMessage[], call: Parameters<typeof api.aiConfirm>[1]) => {
      setLoading(true);
      try {
        return await api.aiConfirm(history, call);
      } finally {
        setLoading(false);
      }
    },
    []
  );

  return { loading, ask, confirm };
}
