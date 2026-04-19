import { useCallback, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { NewProjectDialog } from "./NewProjectDialog";
import { NewMemoDialog } from "./NewMemoDialog";
import { SettingsDialog } from "./SettingsDialog";
import { CommandPalette } from "../command/CommandPalette";
import { FindPalette } from "./FindPalette";
import { buildLocalCommands } from "../command/dispatch";
import type { Tab, Priority, ToolCall } from "../types";
import { PRIORITIES } from "../types";
import { useAppUpdater } from "../hooks/useAppUpdater";
import { useDbRecoveryNotice } from "../hooks/useDbRecoveryNotice";
import { useCmdF } from "../lib/shortcuts";
import * as api from "../api";

export function Layout({
  children,
}: {
  children: (props: {
    activeTab: Tab;
    priorities: Set<Priority>;
    category: string | null;
    openNewProject: () => void;
  }) => React.ReactNode;
}) {
  const pendingUpdate = useAppUpdater();
  useDbRecoveryNotice();
  const [version, setVersion] = useState<string>("");
  useEffect(() => {
    let cancelled = false;
    void getVersion()
      .then((v) => {
        if (!cancelled) setVersion(v);
      })
      .catch(() => {
        /* non-Tauri context (tests) — leave version empty */
      });
    return () => {
      cancelled = true;
    };
  }, []);
  const [activeTab, setActiveTab] = useState<Tab>("projects");
  const [priorities, setPriorities] = useState<Set<Priority>>(new Set(PRIORITIES));
  // null = 전체 보기 (no filter, shows NULL-category rows too). A single
  // category selection deselects all others — category is exclusive, unlike
  // priority which remains multi-select.
  const [activeCategory, setActiveCategory] = useState<string | null>(null);
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsInitialTab, setSettingsInitialTab] = useState<"general" | "ai" | "backup" | "categories">("general");

  const togglePriority = (p: Priority) => {
    setPriorities((prev) => {
      const next = new Set(prev);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });
  };

  const openNewProject = useCallback(() => {
    setActiveTab("projects");
    setNewProjectOpen(true);
  }, []);

  const [newMemoOpen, setNewMemoOpen] = useState(false);
  const [newMemoProjectId, setNewMemoProjectId] = useState<number | null>(null);
  const [findOpen, setFindOpen] = useState(false);
  useCmdF(() => setFindOpen(true));

  useEffect(() => {
    const onNew = (e: Event) => {
      const detail = (e as CustomEvent<{ projectId?: number | null }>).detail;
      setNewMemoProjectId(detail?.projectId ?? null);
      setNewMemoOpen(true);
    };
    window.addEventListener("memo:new-dialog", onNew);
    return () => window.removeEventListener("memo:new-dialog", onNew);
  }, []);

  useEffect(() => {
    const onOpen = (e: Event) => {
      const detail = (e as CustomEvent<{ tab?: "general" | "ai" | "backup" | "categories" }>).detail;
      setSettingsOpen(true);
      if (detail?.tab) setSettingsInitialTab(detail.tab);
    };
    window.addEventListener("settings:open", onOpen);
    return () => window.removeEventListener("settings:open", onOpen);
  }, []);

  const openNewMemo = useCallback(() => {
    setActiveTab("memos");
    setNewMemoOpen(true);
  }, []);

  const commands = buildLocalCommands({
    openNewProject,
    openNewSchedule: () => setActiveTab("calendar"),
    openNewMemo,
  });

  // Dispatch navigation/UI tool calls returned by the agent. `switch_tab` and
  // `set_filter` map directly to our own state setters. Priorities are
  // validated against the canonical PRIORITIES list. Categories are now
  // user-editable, so `set_filter` accepts any non-empty string and trusts
  // the agent to use a live name; stale names just show an empty filter
  // until the user picks something else. Deps array is empty: every setter
  // is a React-stable reference.
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
            (c): c is string => typeof c === "string" && c.length > 0
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
        onOpenSettings={() => { setSettingsInitialTab("general"); setSettingsOpen(true); }}
        version={version}
        pendingUpdate={pendingUpdate}
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
      <FindPalette
        open={findOpen}
        onClose={() => setFindOpen(false)}
        onNavigate={setActiveTab}
      />
      <NewProjectDialog
        open={newProjectOpen}
        onClose={() => setNewProjectOpen(false)}
      />
      <NewMemoDialog
        open={newMemoOpen}
        onClose={() => setNewMemoOpen(false)}
        defaultProjectId={newMemoProjectId}
      />
      <SettingsDialog
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        initialTab={settingsInitialTab}
      />
    </div>
  );
}
