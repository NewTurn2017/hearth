import { useCallback, useEffect, useState } from "react";
import { ask, open } from "@tauri-apps/plugin-dialog";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { NewProjectDialog } from "./NewProjectDialog";
import { AiSettingsDialog } from "./AiSettingsDialog";
import { CommandPalette } from "../command/CommandPalette";
import { buildLocalCommands } from "../command/dispatch";
import type { Tab, Priority, Category, ToolCall } from "../types";
import { PRIORITIES, CATEGORIES } from "../types";
import { useToast } from "../ui/Toast";
import * as api from "../api";

export function Layout({
  children,
}: {
  children: (props: {
    activeTab: Tab;
    priorities: Set<Priority>;
    category: Category | null;
    openNewProject: () => void;
  }) => React.ReactNode;
}) {
  const [activeTab, setActiveTab] = useState<Tab>("projects");
  const [priorities, setPriorities] = useState<Set<Priority>>(new Set(PRIORITIES));
  // null = 전체 보기 (no filter, shows NULL-category rows too). A single
  // category selection deselects all others — category is exclusive, unlike
  // priority which remains multi-select.
  const [activeCategory, setActiveCategory] = useState<Category | null>(null);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [aiSettingsOpen, setAiSettingsOpen] = useState(false);
  const toast = useToast();

  const togglePriority = (p: Priority) => {
    setPriorities((prev) => {
      const next = new Set(prev);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });
  };

  const handleImport = async () => {
    const file = await open({
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    });
    if (!file) return;
    // Tauri 2's `window.confirm` returns a Promise, not a boolean — an
    // un-awaited Promise would be serialized to Rust as an object and the
    // `clear_existing: bool` arg would reject with "invalid type: map".
    // Use the dialog plugin's async `ask` for a proper Yes/No prompt.
    const clearExisting = await ask(
      "기존 데이터를 삭제하고 새로 가져오시겠습니까?",
      { title: "Excel 가져오기", kind: "warning" }
    );
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

  const openNewProject = useCallback(() => {
    setActiveTab("projects");
    setNewProjectOpen(true);
  }, []);

  const commands = buildLocalCommands({
    openNewProject,
    openNewSchedule: () => setActiveTab("calendar"),
    openNewMemo: () => setActiveTab("memos"),
  });

  // Dispatch navigation/UI tool calls returned by the agent. `switch_tab` and
  // `set_filter` map directly to our own state setters. `set_filter` is
  // defensively filtered against the canonical PRIORITIES/CATEGORIES lists —
  // the agent is untrusted and a hallucinated category would silently empty
  // the list without the guard. Deps array is empty: every setter is a
  // React-stable reference.
  const handleClientIntent = useCallback((call: ToolCall) => {
    const args = call.arguments ?? {};
    switch (call.name) {
      case "switch_tab": {
        const tab = args.tab;
        if (tab === "projects" || tab === "calendar" || tab === "memos") {
          setActiveTab(tab);
        }
        break;
      }
      case "set_filter": {
        const pris = args.priorities;
        const cats = args.categories;
        if (Array.isArray(pris)) {
          const valid = pris.filter((p): p is Priority =>
            (PRIORITIES as readonly string[]).includes(p as string)
          );
          if (valid.length > 0) setPriorities(new Set(valid));
        }
        if (Array.isArray(cats)) {
          const firstValid = (cats as unknown[]).find(
            (c): c is Category =>
              typeof c === "string" &&
              (CATEGORIES as readonly string[]).includes(c)
          );
          setActiveCategory(firstValid ?? null);
        }
        break;
      }
    }
  }, []);

  // Global right-click blocker: suppress the native WebKit menu (which
  // includes "Inspect Element" in dev). Cards that want their own menu
  // open it via `useContextMenu` and call `e.stopPropagation()` inside
  // their handler so this listener never sees the bubble. Devtools stays
  // reachable via the standard keyboard shortcut.
  useEffect(() => {
    const block = (e: MouseEvent) => e.preventDefault();
    document.addEventListener("contextmenu", block);
    return () => document.removeEventListener("contextmenu", block);
  }, []);

  return (
    <div className="h-screen flex flex-col bg-[var(--color-surface-0)]">
      <TopBar
        active={activeTab}
        onChange={setActiveTab}
        onImport={handleImport}
        onBackup={handleBackup}
        onOpenAiSettings={() => setAiSettingsOpen(true)}
      />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          activePriorities={priorities}
          activeCategory={activeCategory}
          onTogglePriority={togglePriority}
          onSelectCategory={setActiveCategory}
        />
        <main className="flex-1 overflow-y-auto px-10 py-8 flex flex-col">
          {children({
            activeTab,
            priorities,
            category: activeCategory,
            openNewProject,
          })}
        </main>
      </div>
      <CommandPalette
        commands={commands}
        snapshot={async () => ({
          projects: await api.getProjects(),
          schedules: await api.getSchedules(),
          memos: await api.getMemos(),
        })}
        onClientIntent={handleClientIntent}
      />
      <NewProjectDialog
        open={newProjectOpen}
        onClose={() => setNewProjectOpen(false)}
      />
      <AiSettingsDialog
        open={aiSettingsOpen}
        onClose={() => setAiSettingsOpen(false)}
      />
    </div>
  );
}
