import { useCallback, useRef, useState } from "react";
import type { AiResponse, AiServerState, ChatMessage } from "../types";
import * as api from "../api";

interface SystemContext {
  systemPrompt: string;
}

export function useAi() {
  const [serverState, setServerState] = useState<AiServerState>({ kind: "idle" });
  const [loading, setLoading] = useState(false);
  const historyRef = useRef<ChatMessage[]>([]);

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

  const sendQuery = useCallback(
    async (text: string, ctx: SystemContext): Promise<AiResponse> => {
      const state = await ensureRunning();
      if (state.kind !== "running") {
        throw new Error(
          state.kind === "failed" ? `AI 서버 시작 실패: ${state.error}` : "AI 서버가 실행 중이 아닙니다"
        );
      }

      const userMsg: ChatMessage = { role: "user", content: text };
      const messages: ChatMessage[] = [
        { role: "system", content: ctx.systemPrompt },
        ...historyRef.current,
        userMsg,
      ];

      setLoading(true);
      try {
        const response = await api.aiChat(messages);
        historyRef.current = [
          ...historyRef.current,
          userMsg,
          { role: "assistant", content: JSON.stringify(response) },
        ];
        return response;
      } finally {
        setLoading(false);
      }
    },
    [ensureRunning]
  );

  const resetHistory = useCallback(() => {
    historyRef.current = [];
  }, []);

  return { serverState, loading, sendQuery, resetHistory, ensureRunning };
}
