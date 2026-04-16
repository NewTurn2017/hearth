import { useState, useCallback } from "react";
import type { ChatMessage, AiServerStatus } from "../types";
import * as api from "../api";

export function useAi() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [serverStatus, setServerStatus] = useState<AiServerStatus>({
    running: false,
    port: 8080,
  });
  const [loading, setLoading] = useState(false);
  const [starting, setStarting] = useState(false);

  const startServer = useCallback(async () => {
    setStarting(true);
    try {
      const status = await api.startAiServer();
      setServerStatus(status);
    } catch (e) {
      console.error("Failed to start AI server:", e);
    } finally {
      setStarting(false);
    }
  }, []);

  const stopServer = useCallback(async () => {
    await api.stopAiServer();
    setServerStatus({ running: false, port: 8080 });
    setMessages([]);
  }, []);

  const checkStatus = useCallback(async () => {
    try {
      const status = await api.aiServerStatus();
      setServerStatus(status);
      return status;
    } catch {
      return { running: false, port: 8080 };
    }
  }, []);

  const sendMessage = useCallback(
    async (content: string, systemPrompt: string) => {
      const userMsg: ChatMessage = { role: "user", content };
      const allMessages: ChatMessage[] = [
        { role: "system", content: systemPrompt },
        ...messages,
        userMsg,
      ];

      setMessages((prev) => [...prev, userMsg]);
      setLoading(true);

      try {
        const response = await api.aiChat(allMessages);
        const assistantMsg: ChatMessage = {
          role: "assistant",
          content: response.content,
        };
        setMessages((prev) => [...prev, assistantMsg]);
        return response;
      } catch (e) {
        const errorMsg: ChatMessage = {
          role: "assistant",
          content: `오류: ${e}`,
        };
        setMessages((prev) => [...prev, errorMsg]);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [messages]
  );

  return {
    messages,
    serverStatus,
    loading,
    starting,
    startServer,
    stopServer,
    checkStatus,
    sendMessage,
  };
}
