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
  categories,
}: {
  priorities: Set<Priority>;
  categories: Set<Category>;
}) {
  const { projects, loading, update, remove, reorder } = useProjects(
    priorities,
    categories
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
    />
  );
}

function App() {
  return (
    <ToastProvider>
      <Layout>
        {({ activeTab, priorities, categories }) => (
          <>
            {activeTab === "projects" && (
              <ProjectsTab priorities={priorities} categories={categories} />
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
