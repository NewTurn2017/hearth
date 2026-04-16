import "./App.css";
import { Layout } from "./components/Layout";
import { ProjectList } from "./components/ProjectList";
import { CalendarView } from "./components/CalendarView";
import { MemoBoard } from "./components/MemoBoard";
import { ToastProvider } from "./ui/Toast";
import { useProjects } from "./hooks/useProjects";
import type { Priority, Category } from "./types";

function ProjectsTab({
  priorities,
  category,
  onAdd,
}: {
  priorities: Set<Priority>;
  category: Category | null;
  onAdd: () => void;
}) {
  const { projects, loading, update, remove, reorder } = useProjects(
    priorities,
    category
  );

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-[var(--color-text-muted)] text-sm">
        로딩 중...
      </div>
    );
  }

  return (
    <ProjectList
      projects={projects}
      onUpdate={update}
      onDelete={remove}
      onReorder={reorder}
      onAdd={onAdd}
    />
  );
}

function App() {
  return (
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
    </ToastProvider>
  );
}

export default App;
