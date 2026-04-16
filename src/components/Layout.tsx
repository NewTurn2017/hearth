import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { TabBar } from "./TabBar";
import { Sidebar } from "./Sidebar";
import { AiPanel } from "./AiPanel";
import type { Tab, Priority, Category } from "../types";
import { PRIORITIES, CATEGORIES } from "../types";
import * as api from "../api";

export function Layout({
  children,
}: {
  children: (props: {
    activeTab: Tab;
    priorities: Set<Priority>;
    categories: Set<Category>;
  }) => React.ReactNode;
}) {
  const [activeTab, setActiveTab] = useState<Tab>("projects");
  const [priorities, setPriorities] = useState<Set<Priority>>(
    new Set(PRIORITIES)
  );
  const [categories, setCategories] = useState<Set<Category>>(
    new Set(CATEGORIES)
  );
  const [aiOpen, setAiOpen] = useState(false);

  const togglePriority = (p: Priority) => {
    setPriorities((prev) => {
      const next = new Set(prev);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });
  };

  const toggleCategory = (c: Category) => {
    setCategories((prev) => {
      const next = new Set(prev);
      if (next.has(c)) next.delete(c);
      else next.add(c);
      return next;
    });
  };

  const handleImport = async () => {
    const file = await open({
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    });
    if (!file) return;
    const clearExisting = confirm(
      "기존 데이터를 삭제하고 새로 가져오시겠습니까?"
    );
    try {
      const result = await api.importExcel(file, clearExisting);
      alert(`${result.projects_imported}개 프로젝트 가져오기 완료!`);
      window.location.reload();
    } catch (e) {
      alert(`가져오기 실패: ${e}`);
    }
  };

  const handleBackup = async () => {
    try {
      const path = await api.backupDb();
      alert(`백업 완료: ${path}`);
    } catch (e) {
      alert(`백업 실패: ${e}`);
    }
  };

  return (
    <div className="h-screen flex flex-col">
      <TabBar
        active={activeTab}
        onChange={setActiveTab}
        onImport={handleImport}
        onBackup={handleBackup}
      />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          activePriorities={priorities}
          activeCategories={categories}
          onTogglePriority={togglePriority}
          onToggleCategory={toggleCategory}
        />
        <main className="flex-1 overflow-y-auto p-4">
          {children({ activeTab, priorities, categories })}
        </main>
        {aiOpen && (
          <div className="w-80 shrink-0 bg-[var(--bg-secondary)] border-l border-[var(--border-color)] flex flex-col">
            <AiPanel onClose={() => setAiOpen(false)} />
          </div>
        )}
      </div>
      <button
        onClick={() => setAiOpen(!aiOpen)}
        className="fixed bottom-4 right-4 w-10 h-10 rounded-full bg-[var(--accent)] text-white flex items-center justify-center text-lg shadow-lg hover:opacity-90 transition-opacity z-50"
        title={aiOpen ? "AI 닫기" : "AI 열기"}
      >
        {aiOpen ? "✕" : "🤖"}
      </button>
    </div>
  );
}
