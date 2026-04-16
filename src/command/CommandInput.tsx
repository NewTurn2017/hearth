// src/command/CommandInput.tsx
import { forwardRef, type ChangeEvent, type KeyboardEvent } from "react";
import { Search, Loader2 } from "lucide-react";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";

export const CommandInput = forwardRef<
  HTMLInputElement,
  {
    value: string;
    onChange: (v: string) => void;
    onKeyDown: (e: KeyboardEvent<HTMLInputElement>) => void;
    loading?: boolean;
    placeholder?: string;
  }
>(function CommandInput({ value, onChange, onKeyDown, loading, placeholder }, ref) {
  return (
    <div className="flex items-center gap-2 h-12 px-4 border-b border-[var(--color-border)]">
      <Icon
        icon={loading ? Loader2 : Search}
        size={18}
        className={
          loading
            ? "text-[var(--color-brand-hi)] animate-spin"
            : "text-[var(--color-text-dim)]"
        }
      />
      <input
        ref={ref}
        value={value}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={placeholder ?? "명령을 입력하거나 ?로 AI에게 물어보세요"}
        className="flex-1 bg-transparent outline-none text-[14px] text-[var(--color-text-hi)] placeholder:text-[var(--color-text-dim)]"
        autoFocus
      />
      <Kbd>ESC</Kbd>
      <Kbd>⌘K</Kbd>
    </div>
  );
});
