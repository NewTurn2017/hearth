import { useState, useRef, useEffect } from "react";
import { ChatMessage } from "./ChatMessage";
import { useAi } from "../hooks/useAi";
import type { Project } from "../types";
import * as api from "../api";

const SYSTEM_PROMPT = `당신은 Project Genie AI입니다. 프로젝트, 일정, 메모를 관리하는 유능한 한국어 어시스턴트입니다.

다음 JSON 형식으로 액션을 수행할 수 있습니다:
{"action": "create_project", "name": "...", "priority": "P2", "category": "Side"}
{"action": "delete_project", "id": 5}
{"action": "update_project", "id": 3, "fields": {"priority": "P0"}}
{"action": "search_projects", "query": "..."}
{"action": "create_schedule", "date": "2026-01-01", "time": "10:00", "description": "..."}
{"action": "create_memo", "content": "...", "color": "pink"}

사용 가능한 액션: create_project, update_project, delete_project, search_projects, create_schedule, update_schedule, delete_schedule, create_memo, update_memo, delete_memo

항상 한국어로 응답하세요. 간결하고 친절하게 답변하세요.

현재 프로젝트 요약:
`;

export function AiPanel({ onClose }: { onClose: () => void }) {
  const {
    messages,
    serverStatus,
    loading,
    starting,
    startServer,
    sendMessage,
    checkStatus,
  } = useAi();
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    checkStatus().then((status) => {
      if (!status.running) {
        startServer();
      }
    });
    api.getProjects().then(setProjects).catch(console.error);

    return () => {
      api.stopAiServer().catch(console.error);
    };
  }, []);

  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [messages]);

  const projectsSummary = projects
    .map((p) => `[${p.priority}] ${p.name} (${p.category ?? "미분류"})`)
    .join("\n");

  const handleSend = async () => {
    if (!input.trim() || loading) return;
    const msg = input.trim();
    setInput("");

    const response = await sendMessage(msg, SYSTEM_PROMPT + projectsSummary);

    if (response?.tool_calls) {
      for (const call of response.tool_calls) {
        try {
          const args = call.arguments;
          switch (call.name) {
            case "create_project":
              await api.createProject(
                args.name as string,
                args.priority as string,
                args.category as string | undefined,
                args.path as string | undefined
              );
              break;
            case "update_project":
              await api.updateProject(
                args.id as number,
                args.fields as Record<string, string>
              );
              break;
            case "delete_project":
              await api.deleteProject(args.id as number);
              break;
            case "create_schedule":
              await api.createSchedule(
                args as Parameters<typeof api.createSchedule>[0]
              );
              break;
            case "create_memo":
              await api.createMemo(
                args as Parameters<typeof api.createMemo>[0]
              );
              break;
          }
          api.getProjects().then(setProjects).catch(console.error);
        } catch (e) {
          console.error("Tool call failed:", e);
        }
      }
    }
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--border-color)]">
        <span className="text-sm font-semibold">🤖 AI 어시스턴트</span>
        <button
          onClick={onClose}
          className="text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
        >
          ✕
        </button>
      </div>

      {!serverStatus.running ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            {starting ? (
              <>
                <div className="animate-spin text-2xl mb-2">⚙️</div>
                <p className="text-sm text-[var(--text-secondary)]">
                  AI 모델 로딩 중...
                </p>
              </>
            ) : (
              <>
                <p className="text-sm text-[var(--text-secondary)] mb-2">
                  AI 서버 연결 실패
                </p>
                <button
                  onClick={startServer}
                  className="px-3 py-1.5 text-sm rounded bg-[var(--accent)] text-white"
                >
                  재시도
                </button>
              </>
            )}
          </div>
        </div>
      ) : (
        <>
          <div ref={scrollRef} className="flex-1 overflow-y-auto p-3">
            {messages.length === 0 && (
              <p className="text-sm text-[var(--text-secondary)] text-center mt-8">
                프로젝트 관리에 대해 물어보세요.
                <br />
                예: "P0 프로젝트 뭐 있어?", "새 프로젝트 추가해줘"
              </p>
            )}
            {messages.map((msg, i) => (
              <ChatMessage key={i} message={msg} />
            ))}
            {loading && (
              <div className="text-sm text-[var(--text-secondary)] animate-pulse">
                생각 중...
              </div>
            )}
          </div>

          <div className="p-2 border-t border-[var(--border-color)]">
            <div className="flex gap-2">
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) =>
                  e.key === "Enter" && !e.shiftKey && handleSend()
                }
                placeholder="메시지 입력..."
                className="flex-1 px-3 py-2 rounded-lg bg-[var(--bg-primary)] border border-[var(--border-color)] text-sm outline-none focus:border-[var(--accent)] text-[var(--text-primary)]"
                disabled={loading}
              />
              <button
                onClick={handleSend}
                disabled={loading || !input.trim()}
                className="px-3 py-2 rounded-lg bg-[var(--accent)] text-white text-sm disabled:opacity-50"
              >
                ↑
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
