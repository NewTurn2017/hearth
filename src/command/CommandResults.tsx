// src/command/CommandResults.tsx
import type { LucideIcon } from "lucide-react";
import { ArrowRight, Zap } from "lucide-react";
import type { AiAction } from "../types";
import type { LocalCommand } from "./types";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";
import { cn } from "../lib/cn";

export function CommandResults({
  items,
  activeIndex,
  onHover,
  onSelect,
  aiReply,
}: {
  items: ResultItem[];
  activeIndex: number;
  onHover: (i: number) => void;
  onSelect: (i: number) => void;
  aiReply?: string;
}) {
  return (
    <div className="max-h-[360px] overflow-y-auto py-1">
      {aiReply && (
        <div className="px-4 py-3 text-[13px] text-[var(--color-text)] whitespace-pre-wrap border-b border-[var(--color-border)]">
          {aiReply}
        </div>
      )}
      {items.map((item, i) => (
        <button
          key={item.id}
          onMouseEnter={() => onHover(i)}
          onClick={() => onSelect(i)}
          className={cn(
            "w-full flex items-center gap-3 h-10 px-4 text-left text-[13px]",
            "transition-colors duration-[80ms]",
            i === activeIndex
              ? "bg-[var(--color-surface-2)] text-[var(--color-text-hi)]"
              : "text-[var(--color-text)] hover:bg-[var(--color-surface-2)]"
          )}
        >
          <Icon icon={item.icon} size={16} className="text-[var(--color-text-muted)]" />
          <span className="flex-1 truncate">{item.label}</span>
          {item.hint && (
            <span className="text-[11px] text-[var(--color-text-dim)]">{item.hint}</span>
          )}
          {i === activeIndex && <Kbd>⏎</Kbd>}
        </button>
      ))}
    </div>
  );
}

export interface ResultItem {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
}

export function itemFromLocal(c: LocalCommand): ResultItem {
  return { id: `local:${c.id}`, label: c.label, hint: c.hint, icon: c.icon };
}

export function itemFromAi(a: AiAction, idx: number): ResultItem {
  return {
    id: `ai:${idx}`,
    label: a.label,
    hint: a.type === "mutation" ? "확인 필요" : a.type === "navigation" ? "이동" : "정보",
    icon: a.type === "mutation" ? Zap : ArrowRight,
  };
}
