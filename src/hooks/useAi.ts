// Thin wrapper over the AI server lifecycle + a single-shot chat call.
//
// The old useAi hook accumulated a multi-turn `historyRef`, but the agent loop
// now encapsulates a whole turn (reads, mutations, final reply) inside a
// single `ai_chat` invocation, and the command palette treats each user
// question as an independent interaction. Holding half-finished history here
// caused stale state to leak into the next turn, so we drop it — the palette
// owns any pending-mutation history externally.
import { useCallback, useState } from "react";
import type { AgentResult, AiServerState, ChatMessage } from "../types";
import * as api from "../api";

interface SystemContext {
  systemPrompt: string;
}

export function useAi() {
  const [serverState, setServerState] = useState<AiServerState>({ kind: "idle" });
  const [loading, setLoading] = useState(false);

  const ensureRunning = useCallback(async (): Promise<AiServerState> => {
    const current = await api.aiServerStatus();
    setServerState(current);
    if (current.kind === "running") return current;

    setServerState({ kind: "starting" });
    try {
      const next = await api.startAiServer();
      setServerState(next);
      return next;
    } catch (e) {
      const failed: AiServerState = { kind: "failed", error: String(e) };
      setServerState(failed);
      return failed;
    }
  }, []);

  const ask = useCallback(
    async (text: string, ctx: SystemContext): Promise<AgentResult> => {
      const state = await ensureRunning();
      if (state.kind !== "running") {
        throw new Error(
          state.kind === "failed"
            ? `AI 서버 시작 실패: ${state.error}`
            : "AI 서버가 실행 중이 아닙니다"
        );
      }

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
    [ensureRunning]
  );

  /** Resume the loop after the user approves a pending mutation. */
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

  const resetServerState = useCallback(() => {
    setServerState({ kind: "idle" });
  }, []);

  return {
    serverState,
    loading,
    ask,
    confirm,
    ensureRunning,
    resetServerState,
  };
}
