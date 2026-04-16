import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import type { Project } from "../types";
import { CATEGORY_COLORS } from "../types";
import type { Category } from "../types";

export function ProjectCard({
  project,
  onUpdate,
  onDelete,
  onOpenGhostty,
  onOpenFinder,
}: {
  project: Project;
  onUpdate: (id: number, fields: Record<string, string>) => void;
  onDelete: (id: number) => void;
  onOpenGhostty: (path: string) => void;
  onOpenFinder: (path: string) => void;
}) {
  const [editing, setEditing] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: project.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const startEdit = (field: string, value: string) => {
    setEditing(field);
    setEditValue(value);
  };

  const commitEdit = () => {
    if (editing && editValue.trim()) {
      onUpdate(project.id, { [editing]: editValue.trim() });
    }
    setEditing(null);
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="flex items-center gap-3 px-3 py-2 bg-[var(--bg-tertiary)] rounded-lg group hover:bg-[#353740] transition-colors"
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab text-[var(--text-secondary)] hover:text-[var(--text-primary)] shrink-0"
        title="드래그하여 순서 변경"
      >
        ≡
      </button>

      <div className="flex-1 min-w-0">
        {editing === "name" ? (
          <input
            autoFocus
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => e.key === "Enter" && commitEdit()}
            className="bg-transparent border-b border-[var(--accent)] outline-none text-sm w-full text-[var(--text-primary)]"
          />
        ) : (
          <span
            onClick={() => startEdit("name", project.name)}
            className="text-sm font-medium cursor-pointer truncate block"
          >
            {project.name}
          </span>
        )}
        {editing === "evaluation" ? (
          <input
            autoFocus
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => e.key === "Enter" && commitEdit()}
            className="bg-transparent border-b border-[var(--accent)] outline-none text-xs w-full text-[var(--text-secondary)] mt-0.5"
          />
        ) : (
          <span
            onClick={() => startEdit("evaluation", project.evaluation ?? "")}
            className="text-xs text-[var(--text-secondary)] cursor-pointer truncate block mt-0.5"
          >
            {project.evaluation || "메모 없음"}
          </span>
        )}
      </div>

      {project.category && (
        <span
          className="text-xs px-2 py-0.5 rounded-full shrink-0"
          style={{
            backgroundColor:
              (CATEGORY_COLORS[project.category as Category] ?? "#6b7280") +
              "20",
            color: CATEGORY_COLORS[project.category as Category] ?? "#6b7280",
          }}
        >
          {project.category}
        </span>
      )}

      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
        {project.path && (
          <>
            <button
              onClick={() => onOpenGhostty(project.path!)}
              className="px-1.5 py-0.5 text-xs rounded bg-[var(--bg-secondary)] hover:bg-[var(--accent)] transition-colors"
              title="Ghostty에서 열기"
            >
              ▶
            </button>
            <button
              onClick={() => onOpenFinder(project.path!)}
              className="px-1.5 py-0.5 text-xs rounded bg-[var(--bg-secondary)] hover:bg-[var(--accent)] transition-colors"
              title="Finder에서 열기"
            >
              📁
            </button>
          </>
        )}
        <button
          onClick={() => onDelete(project.id)}
          className="px-1.5 py-0.5 text-xs rounded bg-[var(--bg-secondary)] hover:bg-red-600 transition-colors"
          title="삭제"
        >
          ✕
        </button>
      </div>
    </div>
  );
}
