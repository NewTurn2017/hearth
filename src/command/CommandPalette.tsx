// src/command/CommandPalette.tsx
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import { Loader2 } from "lucide-react";
import { useCmdK } from "../lib/shortcuts";
import { useToast } from "../ui/Toast";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { cn } from "../lib/cn";
import { CommandInput } from "./CommandInput";
import { CommandResults, itemFromLocal, type ResultItem } from "./CommandResults";
import { CommandEmpty } from "./CommandEmpty";
import { useCommandState } from "./useCommandState";
import type { LocalCommand } from "./types";
import { useAi } from "../hooks/useAi";
import { buildSystemPrompt } from "./buildSystemPrompt";
import * as api from "../api";
import type { AgentResult, ChatMessage, ToolCall } from "../types";
import type { Project, Schedule, Memo } from "../types";

/** Carrying state for a paused agent turn — the loop is waiting on the user
 *  to approve `call`. `history` is opaque to the UI but must be round-tripped
 *  back to `ai_confirm` so the backend can resume where it left off. */
interface AiPending {
  call: ToolCall;
  label: string;
  history: ChatMessage[];
}

/** Broadcast a data-changed event matching the entity of the just-executed
 *  tool. Tool names follow `<verb>_<entity>` (`create_project`,
 *  `delete_schedule`, …), so we derive the event channel from the second
 *  token. Restricted to an allowlist so future non-entity mutation tools
 *  (e.g. a `rename_category`) don't silently dispatch garbage channels —
 *  listeners in `useProjects`, `useMemos`, `useSchedules` cover exactly
 *  these three. */
const MUTATION_ENTITIES = ["project", "memo", "schedule"] as const;
function notifyMutation(toolName: string): void {
  const entity = toolName.split("_")[1];
  if (!entity) return;
  if (!(MUTATION_ENTITIES as readonly string[]).includes(entity)) return;
  window.dispatchEvent(new CustomEvent(`${entity}s:changed`));
}

export function CommandPalette({
  commands,
  snapshot,
  onClientIntent,
}: {
  commands: LocalCommand[];
  snapshot: () => Promise<{ projects: Project[]; schedules: Schedule[]; memos: Memo[] }>;
  /** Dispatches a navigation/UI-state tool call the agent returned. The
   *  palette collects these during the loop and hands each one off after the
   *  final reply so the user sees the answer and the UI moves in tandem.
   *  Wiring lives in `Layout` where the real state setters are. */
  onClientIntent?: (call: ToolCall) => void;
}) {
  const state = useCommandState(commands);
  const toast = useToast();
  const inputRef = useRef<HTMLInputElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [pendingConfirm, setPendingConfirm] = useState<LocalCommand | null>(null);

  const ai = useAi();
  const [aiReply, setAiReply] = useState<string | undefined>(undefined);
  const [aiPending, setAiPending] = useState<AiPending | null>(null);

  useCmdK(() => {
    state.setOpen(true);
    setTimeout(() => inputRef.current?.focus(), 0);
  });

  // Eagerly start the MLX server as soon as the user enters AI mode
  // (so the loading dialog appears right after typing `?`, before they've
  // even finished the question).
  useEffect(() => {
    if (state.mode !== "ai") return;
    ai.ensureRunning().catch(() => {
      /* surfaced via serverState === 'failed' dialog */
    });
  }, [state.mode, ai.ensureRunning]);

  // Editing the question invalidates any prior answer or pending mutation —
  // otherwise approving an old "delete #3" modal after retyping a new query
  // would execute a mutation the user no longer sees on screen.
  useEffect(() => {
    setAiReply(undefined);
    setAiPending(null);
  }, [state.mode, state.aiQuery]);

  /** Merge one agent-loop result into component state. */
  const applyResult = useCallback(
    (r: AgentResult) => {
      if (r.kind === "final") {
        setAiReply(r.reply);
        setAiPending(null);
        // Hand off any collected navigation/UI tool calls (set_filter,
        // switch_tab, focus_*) — Layout's handler applies them to real state.
        // Fall back to a toast if the host forgot to wire one (dev-only
        // defensive path; not expected in production).
        for (const ci of r.client_intents) {
          if (onClientIntent) {
            onClientIntent(ci);
          } else {
            toast.success(`${ci.name.replace(/_/g, " ")} 요청됨`);
          }
        }
      } else {
        // pending: stash it so the confirm dialog opens and we can resume.
        setAiReply(undefined);
        setAiPending({ call: r.call, label: r.label, history: r.history });
      }
    },
    [onClientIntent, toast]
  );

  const fireAi = useCallback(async () => {
    if (state.mode !== "ai" || state.aiQuery.length === 0 || ai.loading) return;
    try {
      const snap = await snapshot();
      const r = await ai.ask(state.aiQuery, {
        systemPrompt: buildSystemPrompt(snap),
      });
      applyResult(r);
    } catch (e) {
      toast.error(`AI 오류: ${e}`);
    }
  }, [state.mode, state.aiQuery, ai, snapshot, toast, applyResult]);

  // AI mode no longer has selectable action rows — mutations flow through the
  // confirm dialog automatically, so the palette list only renders local
  // commands.
  const items: ResultItem[] = useMemo(() => {
    if (state.mode === "ai") return [];
    return state.localMatches.map(itemFromLocal);
  }, [state.mode, state.localMatches]);

  useEffect(() => {
    setActiveIndex(0);
  }, [state.query]);

  const close = useCallback(() => {
    state.setOpen(false);
    state.reset();
    setPendingConfirm(null);
    setAiPending(null);
    setAiReply(undefined);
  }, [state]);

  const executeCommand = useCallback(
    async (cmd: LocalCommand) => {
      try {
        const undo = await cmd.run();
        toast.success(`${cmd.label} 완료`, {
          undo: typeof undo === "function" ? undo : undefined,
        });
        close();
      } catch (e) {
        toast.error(`${cmd.label} 실패: ${e}`);
      }
    },
    [toast, close]
  );

  const onSelect = useCallback(
    (i: number) => {
      // In AI mode the list is empty (items comes back as []), so this path
      // only fires for local commands.
      const cmd = state.localMatches[i];
      if (!cmd) return;
      if (cmd.mutation) {
        setPendingConfirm(cmd);
      } else {
        executeCommand(cmd);
      }
    },
    [state.localMatches, executeCommand]
  );

  const approveAi = useCallback(async () => {
    if (!aiPending) return;
    const pending = aiPending;
    setAiPending(null); // close the modal eagerly — the loop may open a new one
    try {
      const r = await ai.confirm(pending.history, pending.call);
      // Notify data hooks (useProjects etc.) to refetch. The backend writes
      // the row synchronously before `confirm` returns, so by the time we
      // dispatch here the DB is already authoritative. Without this dispatch
      // the list stays visually stale even though the row exists.
      notifyMutation(pending.call.name);
      applyResult(r);
      if (r.kind === "final") {
        toast.success(`${pending.label} 완료`);
      }
    } catch (e) {
      toast.error(`${pending.label} 실패: ${e}`);
    }
  }, [aiPending, ai, applyResult, toast]);

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      // Ignore Enter fired while the IME is composing Hangul — otherwise the
      // user loses the final character of a composition the moment they hit
      // Enter. `nativeEvent.isComposing` is the correct signal on React 19.
      if (e.nativeEvent.isComposing) return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, items.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const hasResponded = aiReply !== undefined || aiPending !== null;
        if (
          state.mode === "ai" &&
          !hasResponded &&
          !ai.loading &&
          state.aiQuery.length > 0
        ) {
          fireAi();
        } else if (items.length > 0) {
          onSelect(activeIndex);
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    },
    [
      items.length,
      activeIndex,
      onSelect,
      close,
      state.mode,
      state.aiQuery,
      aiReply,
      aiPending,
      ai.loading,
      fireAi,
    ]
  );

  if (!state.open) return null;

  return (
    <>
      {createPortal(
        <div
          className="fixed inset-0 z-[90] flex items-start justify-center pt-[15vh] bg-black/40"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) close();
          }}
        >
          <div
            className={cn(
              "w-full max-w-[620px] rounded-[var(--radius-xl)]",
              "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
              "shadow-[var(--shadow-e3)] overflow-hidden"
            )}
          >
            <CommandInput
              ref={inputRef}
              value={state.query}
              onChange={state.setQuery}
              onKeyDown={onKeyDown}
              loading={ai.loading}
              mode={state.mode}
              hasResponse={aiReply !== undefined || aiPending !== null}
            />
            {items.length === 0 && !(state.mode === "ai" && aiReply) ? (
              <CommandEmpty
                text={
                  state.mode === "ai"
                    ? state.aiQuery.length === 0
                      ? "AI 모드 — 질문을 입력하세요. 예: '? PickAt 프로젝트 추가'"
                      : ai.loading
                      ? "AI가 응답을 작성 중입니다…"
                      : aiPending
                      ? "확인 대기 중…"
                      : "응답이 없습니다. 질문을 조금 바꿔 보세요."
                    : "매칭되는 명령이 없습니다. '?'로 AI에 물어보세요."
                }
              />
            ) : (
              <CommandResults
                items={items}
                activeIndex={activeIndex}
                onHover={setActiveIndex}
                onSelect={onSelect}
                aiReply={state.mode === "ai" ? aiReply : undefined}
              />
            )}
          </div>
        </div>,
        document.body
      )}

      <Dialog
        open={!!pendingConfirm}
        onClose={() => setPendingConfirm(null)}
        labelledBy="confirm-title"
      >
        {pendingConfirm && (
          <>
            <h2 id="confirm-title" className="text-heading text-[var(--color-text-hi)] mb-2">
              확인
            </h2>
            <p className="text-[13px] text-[var(--color-text)] mb-5 whitespace-pre-line">
              {pendingConfirm.confirmMessage ?? `${pendingConfirm.label}을(를) 실행합니다.`}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPendingConfirm(null)}>
                취소
              </Button>
              <Button
                variant="primary"
                autoFocus
                onClick={() => {
                  const cmd = pendingConfirm;
                  setPendingConfirm(null);
                  executeCommand(cmd);
                }}
              >
                실행
              </Button>
            </div>
          </>
        )}
      </Dialog>

      {/* AI mutation confirm — auto-opens whenever the agent loop returns a
          Pending, so the user never has to hunt for a button to proceed. */}
      <Dialog
        open={!!aiPending}
        onClose={() => setAiPending(null)}
        labelledBy="ai-confirm-title"
      >
        {aiPending && (
          <>
            <h2 id="ai-confirm-title" className="text-heading text-[var(--color-text-hi)] mb-2">
              AI 실행 확인
            </h2>
            <p className="text-[13px] text-[var(--color-text)] mb-5 whitespace-pre-line">
              {aiPending.label}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setAiPending(null)}>
                취소
              </Button>
              <Button variant="primary" autoFocus onClick={approveAi}>
                실행
              </Button>
            </div>
          </>
        )}
      </Dialog>

      {/* MLX 서버 부팅 대기 다이얼로그 — 최초 모델 로드가 수십 초 걸리므로
          사용자에게 진행 상황을 분명히 보여준다. */}
      <Dialog
        open={ai.serverState.kind === "starting"}
        onClose={() => {
          /* 시작 중에는 임의 종료 방지 — 명시적 취소 버튼으로만 */
        }}
        labelledBy="ai-loading-title"
      >
        <h2
          id="ai-loading-title"
          className="text-heading text-[var(--color-text-hi)] mb-2"
        >
          AI 서버 시작 중
        </h2>
        <p className="text-[13px] text-[var(--color-text)] mb-4">
          MLX 모델을 로드하고 있습니다. 최초 실행 시 수십 초 ~ 최대 2분 걸릴 수 있어요.
        </p>
        <div className="flex items-center gap-2 text-[12px] text-[var(--color-text-muted)] mb-5">
          <Loader2 size={14} className="animate-spin" aria-hidden />
          <span>127.0.0.1:18080 연결 대기 중…</span>
        </div>
        <div className="flex justify-end">
          <Button
            variant="secondary"
            onClick={async () => {
              try {
                await api.stopAiServer();
              } finally {
                ai.resetServerState();
              }
            }}
          >
            취소
          </Button>
        </div>
      </Dialog>

      <Dialog
        open={ai.serverState.kind === "failed"}
        onClose={ai.resetServerState}
        labelledBy="ai-error-title"
      >
        <h2
          id="ai-error-title"
          className="text-heading text-[var(--color-text-hi)] mb-2"
        >
          AI 서버 시작 실패
        </h2>
        <p className="text-[13px] text-[var(--color-danger)] mb-5 whitespace-pre-line break-words">
          {ai.serverState.kind === "failed" ? ai.serverState.error : ""}
        </p>
        <div className="flex justify-end">
          <Button variant="primary" autoFocus onClick={ai.resetServerState}>
            확인
          </Button>
        </div>
      </Dialog>
    </>
  );
}
