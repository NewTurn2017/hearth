import type { Tab } from "../types";

const tabs: { key: Tab; label: string; icon: string }[] = [
  { key: "projects", label: "프로젝트", icon: "📋" },
  { key: "calendar", label: "캘린더", icon: "📅" },
  { key: "memos", label: "메모보드", icon: "📌" },
];

export function TabBar({
  active,
  onChange,
  onImport,
  onBackup,
}: {
  active: Tab;
  onChange: (tab: Tab) => void;
  onImport: () => void;
  onBackup: () => void;
}) {
  return (
    <div className="flex items-center gap-1 px-4 py-2 bg-[var(--bg-secondary)] border-b border-[var(--border-color)]">
      <span className="text-lg font-bold mr-4 text-[var(--accent)]">
        Project Genie
      </span>
      {tabs.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          className={`px-3 py-1.5 rounded-md text-sm font-medium transition-colors ${
            active === tab.key
              ? "bg-[var(--accent)] text-white"
              : "text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)]"
          }`}
        >
          {tab.icon} {tab.label}
        </button>
      ))}
      <div className="flex-1" />
      <button
        onClick={onImport}
        className="px-2 py-1 text-xs rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors"
        title="Excel Import"
      >
        📥 가져오기
      </button>
      <button
        onClick={onBackup}
        className="px-2 py-1 text-xs rounded text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-tertiary)] transition-colors"
        title="백업"
      >
        💾 백업
      </button>
    </div>
  );
}
