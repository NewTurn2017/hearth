// src/components/TopBar.tsx
import { useState } from "react";
import {
  LayoutGrid,
  CalendarDays,
  StickyNote,
  Settings2,
  Sparkles,
} from "lucide-react";
import type { Tab } from "../types";
import type { PendingUpdate } from "../hooks/useAppUpdater";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
import { useToast } from "../ui/Toast";
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
  onOpenSettings,
  version,
  pendingUpdate,
}: {
  active: Tab;
  onChange: (tab: Tab) => void;
  onOpenSettings: () => void;
  version: string;
  pendingUpdate: PendingUpdate | null;
}) {
  const toast = useToast();
  const [installing, setInstalling] = useState(false);
  const handleInstall = async () => {
    if (!pendingUpdate) return;
    setInstalling(true);
    try {
      await pendingUpdate.install();
    } catch (e) {
      toast.error(`업데이트 실패: ${e}`);
      setInstalling(false);
    }
    // On success the app relaunches — no state to clean up.
  };

  return (
    <div className="flex items-center gap-1 px-4 h-12 bg-[var(--color-surface-1)] border-b border-[var(--color-border)]">
      <span className="text-heading text-[var(--color-text-hi)] mr-1 tracking-tight">
        Hearth
      </span>
      {version && (
        <Tooltip label={pendingUpdate ? `현재 v${version} · 최신 v${pendingUpdate.version}` : `버전 v${version}`}>
          <span className="text-[11px] font-mono text-[var(--color-text-dim)] mr-3">
            v{version}
          </span>
        </Tooltip>
      )}
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
      {pendingUpdate && (
        <Tooltip label={`새 버전 v${pendingUpdate.version} 설치 후 재시작`}>
          <Button
            variant="primary"
            size="sm"
            leftIcon={Sparkles}
            onClick={handleInstall}
            disabled={installing}
          >
            {installing ? "설치 중…" : `업데이트 v${pendingUpdate.version}`}
          </Button>
        </Tooltip>
      )}
      <AiStatusPill />
      <Button
        variant="ghost"
        size="sm"
        leftIcon={Settings2}
        onClick={onOpenSettings}
      >
        설정
      </Button>
    </div>
  );
}
