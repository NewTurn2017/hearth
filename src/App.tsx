import "./App.css";
import { Layout } from "./components/Layout";
import { ProjectList } from "./components/ProjectList";
import { CalendarView } from "./components/CalendarView";
import { MemoBoard } from "./components/MemoBoard";
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
    return <div className="text-[var(--text-secondary)]">로딩 중...</div>;
  }

  if (projects.length === 0) {
    return (
      <div className="text-[var(--text-secondary)] text-center mt-20">
        프로젝트가 없습니다. Excel 파일을 가져오기 해주세요.
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
  );
}

export default App;
