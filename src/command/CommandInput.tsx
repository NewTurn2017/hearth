// src/command/CommandInput.tsx
import { forwardRef, type ChangeEvent, type KeyboardEvent } from "react";
import { Search, Loader2, Sparkles } from "lucide-react";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";
import { cn } from "../lib/cn";

export const CommandInput = forwardRef<
  HTMLInputElement,
  {
    value: string;
    onChange: (v: string) => void;
    onKeyDown: (e: KeyboardEvent<HTMLInputElement>) => void;
    loading?: boolean;
    placeholder?: string;
    mode?: "local" | "ai";
    hasResponse?: boolean;
  }
>(function CommandInput(
  { value, onChange, onKeyDown, loading, placeholder, mode, hasResponse },
  ref
) {
  const isAi = mode === "ai";
  return (
    <div
      className={cn(
        "flex items-center gap-2 h-12 px-4 border-b",
        "transition-colors duration-[120ms]",
        isAi
          ? "bg-[var(--color-brand-soft)] border-[var(--color-brand)]"
          : "border-[var(--color-border)]"
      )}
    >
      <Icon
        icon={loading ? Loader2 : isAi ? Sparkles : Search}
        size={18}
        className={
          loading
            ? "text-[var(--color-brand-hi)] animate-spin"
            : isAi
            ? "text-[var(--color-brand-hi)]"
            : "text-[var(--color-text-dim)]"
        }
      />
      <input
        ref={ref}
        value={value}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={
          placeholder ??
          (isAi
            ? "질문을 입력하고 ⏎ — 예: '? PickAt 프로젝트 추가'"
            : "명령을 입력하거나 ?로 AI에게 물어보세요")
        }
        className="flex-1 bg-transparent outline-none text-[14px] text-[var(--color-text-hi)] placeholder:text-[var(--color-text-dim)]"
        autoFocus
      />
      {/* Progression hint — shows the user what Enter will do next. */}
      {isAi && loading && (
        <span className="text-[11px] text-[var(--color-brand-hi)]">응답 중…</span>
      )}
      {isAi && !loading && !hasResponse && value.trim().length > 1 && (
        <span className="text-[11px] text-[var(--color-text-muted)]">
          ⏎ 질문 보내기
        </span>
      )}
      <Kbd>ESC</Kbd>
      <Kbd>⌘K</Kbd>
    </div>
  );
});
