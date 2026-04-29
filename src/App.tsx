import { useEffect, useMemo, useState } from "react";
import "./App.css";
import { Layout } from "./components/Layout";
import { ProjectList } from "./components/ProjectList";
import { ProjectDetailDialog } from "./components/ProjectDetailDialog";
import { CalendarView } from "./components/CalendarView";
import { MemoBoard } from "./components/MemoBoard";
import { MigrationWizard } from "./components/MigrationWizard";
import { ToastProvider } from "./ui/Toast";
import { ThemeProvider } from "./theme/ThemeContext";
import { useProjects } from "./hooks/useProjects";
import { useMemos } from "./hooks/useMemos";
import { useUiScale } from "./hooks/useUiScale";
import { useTauriDbChangeBridge } from "./lib/dbChangeBridge";
import type { Priority } from "./types";

function ProjectsTab({
  priorities,
  category,
  onAdd,
}: {
  priorities: Set<Priority>;
  category: string | null;
  onAdd: () => void;
}) {
  const { projects, loading, update, remove, reorder } = useProjects(
    priorities,
    category
  );
  const { memos, reload: reloadMemos } = useMemos();
  const [detailProjectId, setDetailProjectId] = useState<number | null>(null);

  const detailProject = useMemo(
    () => projects.find((p) => p.id === detailProjectId) ?? null,
    [projects, detailProjectId]
  );

  // Close the dialog if the underlying project was deleted or filtered out
  // from the current view — otherwise the dialog would keep rendering a
  // stale snapshot and saves would 404 in Rust.
  useEffect(() => {
    if (detailProjectId !== null && !detailProject) {
      setDetailProjectId(null);
    }
  }, [detailProject, detailProjectId]);


  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-[var(--color-text-muted)] text-sm">
        로딩 중...
      </div>
    );
  }

  return (
    <>
      <ProjectList
        projects={projects}
        onUpdate={update}
        onDelete={remove}
        onReorder={reorder}
        onAdd={onAdd}
        onOpenDetail={(p) => setDetailProjectId(p.id)}
      />
      <ProjectDetailDialog
        open={detailProjectId !== null}
        project={detailProject}
        memos={memos}
        onClose={() => setDetailProjectId(null)}
        onProjectUpdated={() => {
          /* useProjects subscribes to projects:changed already */
        }}
        onMemosChanged={reloadMemos}
      />
    </>
  );
}

function App() {
  useUiScale();
  useTauriDbChangeBridge();
  return (
    <ThemeProvider>
      <ToastProvider>
        <Layout>
          {({ activeTab, priorities, category, openNewProject }) => (
            <>
              {activeTab === "projects" && (
                <ProjectsTab
                  priorities={priorities}
                  category={category}
                  onAdd={openNewProject}
                />
              )}
              {activeTab === "calendar" && <CalendarView />}
              {activeTab === "memos" && <MemoBoard />}
            </>
          )}
        </Layout>
        <MigrationWizard />
      </ToastProvider>
    </ThemeProvider>
  );
}

export default App;
