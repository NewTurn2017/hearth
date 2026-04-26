// Unified settings modal. Replaces the AI-only dialog.
//
// Three tabs: AI / 백업 / 카테고리. We keep every section mounted so unsaved
// input survives a tab switch; only the `active` prop flips so each section
// can refetch on activation.

import { useEffect, useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { cn } from "../lib/cn";
import { SettingsGeneralSection } from "./SettingsGeneralSection";
import { SettingsThemeSection } from "./SettingsThemeSection";
import { SettingsAiSection } from "./SettingsAiSection";
import { SettingsBackupSection } from "./SettingsBackupSection";
import { SettingsCategoriesSection } from "./SettingsCategoriesSection";
import { SettingsIntegrationsSection } from "./SettingsIntegrationsSection";
import { SettingsLicenseSection } from "./SettingsLicenseSection";
import { SettingsAboutSection } from "./SettingsAboutSection";
import type { PendingUpdate } from "../hooks/useAppUpdater";

type TabKey =
  | "general"
  | "theme"
  | "ai"
  | "backup"
  | "categories"
  | "integrations"
  | "license"
  | "about";

const TABS: { key: TabKey; label: string }[] = [
  { key: "general", label: "일반" },
  { key: "theme", label: "테마" },
  { key: "ai", label: "AI" },
  { key: "backup", label: "백업/가져오기" },
  { key: "categories", label: "카테고리" },
  { key: "integrations", label: "통합" },
  { key: "license", label: "라이선스" },
  { key: "about", label: "정보" },
];

export function SettingsDialog({
  open,
  onClose,
  initialTab = "general",
  pendingUpdate,
  updateChecking = false,
  onCheckForUpdates,
}: {
  open: boolean;
  onClose: () => void;
  initialTab?: TabKey;
  pendingUpdate?: PendingUpdate | null;
  updateChecking?: boolean;
  onCheckForUpdates?: () => Promise<void>;
}) {
  const [tab, setTab] = useState<TabKey>(initialTab);

  // Sync the active tab whenever the dialog (re-)opens via a different entry
  // point — e.g. the AiStatusPill dispatches "settings:open" with tab:"ai".
  useEffect(() => {
    if (open) setTab(initialTab);
  }, [open, initialTab]);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy="settings-title"
      className="max-w-2xl"
    >
      <h2
        id="settings-title"
        className="text-heading text-[var(--color-text-hi)] mb-4"
      >
        설정
      </h2>

      <div
        role="tablist"
        aria-label="설정 탭"
        className="flex gap-1 mb-5 border-b border-[var(--color-border)]"
      >
        {TABS.map((t) => (
          <button
            key={t.key}
            type="button"
            role="tab"
            aria-selected={tab === t.key}
            onClick={() => setTab(t.key)}
            className={cn(
              "px-3 h-9 text-[13px] -mb-px border-b-2 transition-colors",
              tab === t.key
                ? "border-[var(--color-brand-hi)] text-[var(--color-text-hi)]"
                : "border-transparent text-[var(--color-text-muted)] hover:text-[var(--color-text)]",
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Each section stays mounted; only visibility flips. `active` tells
          the section it is the one in focus so it can refetch. */}
      <div className={tab === "general" ? "" : "hidden"}>
        <SettingsGeneralSection
          active={tab === "general"}
          pendingUpdate={pendingUpdate ?? null}
          updateChecking={updateChecking}
          onCheckForUpdates={onCheckForUpdates}
        />
      </div>
      <div className={tab === "theme" ? "" : "hidden"}>
        <SettingsThemeSection />
      </div>
      <div className={tab === "ai" ? "" : "hidden"}>
        <SettingsAiSection active={tab === "ai"} />
      </div>
      <div className={tab === "backup" ? "" : "hidden"}>
        <SettingsBackupSection active={tab === "backup"} />
      </div>
      <div className={tab === "categories" ? "" : "hidden"}>
        <SettingsCategoriesSection />
      </div>
      <div className={tab === "integrations" ? "" : "hidden"}>
        <SettingsIntegrationsSection />
      </div>
      <div className={tab === "license" ? "" : "hidden"}>
        <SettingsLicenseSection />
      </div>
      <div className={tab === "about" ? "" : "hidden"}>
        <SettingsAboutSection active={tab === "about"} />
      </div>

      <div className="flex justify-end mt-6">
        <Button variant="secondary" onClick={onClose}>
          닫기
        </Button>
      </div>
    </Dialog>
  );
}
