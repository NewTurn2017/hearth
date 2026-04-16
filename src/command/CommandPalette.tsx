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
import { CommandResults, itemFromLocal, type ResultItem } from "./CommandResults";
import { CommandEmpty } from "./CommandEmpty";
import { useCommandState } from "./useCommandState";
import type { LocalCommand } from "./types";

export function CommandPalette({ commands }: { commands: LocalCommand[] }) {
  const state = useCommandState(commands);
  const toast = useToast();
  const inputRef = useRef<HTMLInputElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [pendingConfirm, setPendingConfirm] = useState<LocalCommand | null>(null);

  useCmdK(() => {
    state.setOpen(true);
    setTimeout(() => inputRef.current?.focus(), 0);
  });

  const items: ResultItem[] = useMemo(
    () => state.localMatches.map(itemFromLocal),
    [state.localMatches]
  );

  useEffect(() => {
    setActiveIndex(0);
  }, [state.query]);

  const close = useCallback(() => {
    state.setOpen(false);
    state.reset();
    setPendingConfirm(null);
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
            />
            {items.length === 0 ? (
              <CommandEmpty text="매칭되는 명령이 없습니다. '?'로 AI에 물어보세요." />
            ) : (
              <CommandResults
                items={items}
                activeIndex={activeIndex}
                onHover={setActiveIndex}
                onSelect={onSelect}
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
    </>
  );
}
