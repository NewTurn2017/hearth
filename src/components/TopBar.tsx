// src/components/TopBar.tsx
import {
  LayoutGrid,
  CalendarDays,
  StickyNote,
  Download,
  Save,
  Settings2,
} from "lucide-react";
import type { Tab } from "../types";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { cn } from "../lib/cn";
import { AiStatusPill } from "./AiStatusPill";

const tabs: { key: Tab; label: string; icon: typeof LayoutGrid }[] = [
  { key: "projects", label: "프로젝트", icon: LayoutGrid },
  { key: "calendar", label: "캘린더", icon: CalendarDays },
  { key: "memos", label: "메모보드", icon: StickyNote },
];

export function TopBar({
  active,
  onChange,
  onImport,
  onBackup,
  onOpenAiSettings,
}: {
  active: Tab;
  onChange: (tab: Tab) => void;
  onImport: () => void;
  onBackup: () => void;
  onOpenAiSettings: () => void;
}) {
  return (
    <div className="flex items-center gap-1 px-4 h-12 bg-[var(--color-surface-1)] border-b border-[var(--color-border)]">
      <span className="text-heading text-[var(--color-text-hi)] mr-3 tracking-tight">
        Hearth
      </span>
      {tabs.map((t) => (
        <button
          key={t.key}
          onClick={() => onChange(t.key)}
          className={cn(
            "inline-flex items-center gap-1.5 h-8 px-3 rounded-[var(--radius-md)] text-[13px]",
            "transition-colors duration-[120ms]",
            active === t.key
              ? "bg-[var(--color-brand-soft)] text-[var(--color-brand-hi)]"
              : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]"
          )}
        >
          <Icon icon={t.icon} size={16} />
          {t.label}
        </button>
      ))}
      <div className="flex-1" />
      <AiStatusPill />
      <Button
        variant="ghost"
        size="sm"
        leftIcon={Settings2}
        onClick={onOpenAiSettings}
      >
        AI 설정
      </Button>
      <Button variant="ghost" size="sm" leftIcon={Download} onClick={onImport}>
        가져오기
      </Button>
      <Button variant="ghost" size="sm" leftIcon={Save} onClick={onBackup}>
        백업
      </Button>
    </div>
  );
}
