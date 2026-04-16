import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { CommandPalette } from "../command/CommandPalette";
import { buildLocalCommands } from "../command/dispatch";
import type { Tab, Priority, Category } from "../types";
import { PRIORITIES, CATEGORIES } from "../types";
import { useToast } from "../ui/Toast";
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
  const [priorities, setPriorities] = useState<Set<Priority>>(new Set(PRIORITIES));
  const [categories, setCategories] = useState<Set<Category>>(new Set(CATEGORIES));
  const toast = useToast();

  const togglePriority = (p: Priority) => {
    setPriorities((prev) => {
      const next = new Set(prev);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });
  };
  const toggleCategory = (c: Category) => {
    setCategories((prev) => {
      const next = new Set(prev);
      next.has(c) ? next.delete(c) : next.add(c);
      return next;
    });
  };

  const handleImport = async () => {
    const file = await open({
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    });
    if (!file) return;
    const clearExisting = confirm("기존 데이터를 삭제하고 새로 가져오시겠습니까?");
    try {
      const filePath = Array.isArray(file) ? file[0] : file;
      const result = await api.importExcel(filePath, clearExisting);
      toast.success(`${result.projects_imported}개 프로젝트 가져왔습니다`);
      setTimeout(() => window.location.reload(), 800);
    } catch (e) {
      toast.error(`가져오기 실패: ${e}`);
    }
  };

  const handleBackup = async () => {
    try {
      const path = await api.backupDb();
      toast.success(`백업 완료: ${path}`);
    } catch (e) {
      toast.error(`백업 실패: ${e}`);
    }
  };

  const commands = buildLocalCommands({
    openNewProject: () => setActiveTab("projects"),
    openNewSchedule: () => setActiveTab("calendar"),
    openNewMemo: () => setActiveTab("memos"),
  });

  return (
    <div className="h-screen flex flex-col bg-[var(--color-surface-0)]">
      <TopBar
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
      </div>
      <CommandPalette commands={commands} />
    </div>
  );
}
