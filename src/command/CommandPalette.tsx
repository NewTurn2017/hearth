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
import { useCmdK } from "../lib/shortcuts";
import { useToast } from "../ui/Toast";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { cn } from "../lib/cn";
import { CommandInput } from "./CommandInput";
import { CommandResults, itemFromLocal, itemFromAi, type ResultItem } from "./CommandResults";
import { CommandEmpty } from "./CommandEmpty";
import { useCommandState } from "./useCommandState";
import type { LocalCommand } from "./types";
import { useAi } from "../hooks/useAi";
import { buildSystemPrompt } from "./buildSystemPrompt";
import { executeAiAction } from "./executeAiAction";
import type { AiAction } from "../types";
import type { Project, Schedule, Memo } from "../types";

export function CommandPalette({
  commands,
  snapshot,
}: {
  commands: LocalCommand[];
  snapshot: () => Promise<{ projects: Project[]; schedules: Schedule[]; memos: Memo[] }>;
}) {
  const state = useCommandState(commands);
  const toast = useToast();
  const inputRef = useRef<HTMLInputElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [pendingConfirm, setPendingConfirm] = useState<LocalCommand | null>(null);

  const ai = useAi();
  const [aiReply, setAiReply] = useState<string | undefined>(undefined);
  const [aiActions, setAiActions] = useState<AiAction[]>([]);
  const [pendingAiConfirm, setPendingAiConfirm] = useState<AiAction | null>(null);

  useCmdK(() => {
    state.setOpen(true);
    setTimeout(() => inputRef.current?.focus(), 0);
  });

  // Debounced AI fire when mode === 'ai'
  useEffect(() => {
    setAiReply(undefined);
    setAiActions([]);
    if (state.mode !== "ai" || state.aiQuery.length === 0) return;
    const q = state.aiQuery;
    const handle = setTimeout(async () => {
      try {
        const snap = await snapshot();
        const resp = await ai.sendQuery(q, {
          systemPrompt: buildSystemPrompt(snap),
        });
        setAiReply(resp.reply);
        setAiActions(resp.actions);
      } catch (e) {
        toast.error(`AI 오류: ${e}`);
      }
    }, 300);
    return () => clearTimeout(handle);
  }, [state.mode, state.aiQuery, ai, toast, snapshot]);

  const items: ResultItem[] = useMemo(() => {
    if (state.mode === "ai") return aiActions.map((a, i) => itemFromAi(a, i));
    return state.localMatches.map(itemFromLocal);
  }, [state.mode, state.localMatches, aiActions]);

  useEffect(() => {
    setActiveIndex(0);
  }, [state.query]);

  const close = useCallback(() => {
    state.setOpen(false);
    state.reset();
    setPendingConfirm(null);
    setPendingAiConfirm(null);
    setAiReply(undefined);
    setAiActions([]);
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

  const runAi = useCallback(
    async (action: AiAction) => {
      try {
        const undo = await executeAiAction(action);
        toast.success(`${action.label} 완료`, { undo });
        close();
      } catch (e) {
        toast.error(`${action.label} 실패: ${e}`);
      }
    },
    [toast, close]
  );

  const onSelect = useCallback(
    (i: number) => {
      if (state.mode === "ai") {
        const action = aiActions[i];
        if (!action) return;
        if (action.type === "mutation") {
          setPendingAiConfirm(action);
        } else {
          runAi(action);
        }
        return;
      }
      const cmd = state.localMatches[i];
      if (!cmd) return;
      if (cmd.mutation) {
        setPendingConfirm(cmd);
      } else {
        executeCommand(cmd);
      }
    },
    [state.mode, state.localMatches, aiActions, executeCommand, runAi]
  );

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, items.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        onSelect(activeIndex);
      } else if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    },
    [items.length, activeIndex, onSelect, close]
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
            />
            {items.length === 0 ? (
              <CommandEmpty text="매칭되는 명령이 없습니다. '?'로 AI에 물어보세요." />
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
            <p className="text-[13px] text-[var(--color-text)] mb-5">
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

      <Dialog
        open={!!pendingAiConfirm}
        onClose={() => setPendingAiConfirm(null)}
        labelledBy="ai-confirm-title"
      >
        {pendingAiConfirm && (
          <>
            <h2 id="ai-confirm-title" className="text-heading text-[var(--color-text-hi)] mb-2">
              확인
            </h2>
            <p className="text-[13px] text-[var(--color-text)] mb-5">
              {pendingAiConfirm.label}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPendingAiConfirm(null)}>
                취소
              </Button>
              <Button
                variant="primary"
                autoFocus
                onClick={() => {
                  const a = pendingAiConfirm;
                  setPendingAiConfirm(null);
                  runAi(a);
                }}
              >
                실행
              </Button>
            </div>
          </>
        )}
      </Dialog>
    </>
  );
}
