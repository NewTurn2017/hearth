import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Play, FolderOpen, X } from "lucide-react";
import type { Project, Category } from "../types";
import { CATEGORY_COLORS } from "../types";
import { Badge } from "../ui/Badge";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
import { cn } from "../lib/cn";

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
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: project.id });

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
      className={cn(
        "flex items-center gap-3 px-3 h-11 rounded-[var(--radius-md)] group",
        "bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)]",
        "border border-transparent hover:border-[var(--color-border)]",
        "transition-colors duration-[120ms]"
      )}
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)] shrink-0"
        aria-label="드래그하여 순서 변경"
      >
        <Icon icon={GripVertical} size={16} />
      </button>

      <div className="flex-1 min-w-0">
        {editing === "name" ? (
          <input
            autoFocus
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => e.key === "Enter" && commitEdit()}
            className="bg-transparent border-b border-[var(--color-brand-hi)] outline-none text-[13px] w-full text-[var(--color-text)]"
          />
        ) : (
          <span
            onClick={() => startEdit("name", project.name)}
            className="text-[13px] font-medium text-[var(--color-text)] cursor-text truncate block"
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
            className="bg-transparent border-b border-[var(--color-brand-hi)] outline-none text-[11px] w-full text-[var(--color-text-muted)] mt-0.5"
          />
        ) : (
          <span
            onClick={() => startEdit("evaluation", project.evaluation ?? "")}
            className="text-[11px] text-[var(--color-text-dim)] cursor-text truncate block mt-0.5"
          >
            {project.evaluation || "메모 없음"}
          </span>
        )}
      </div>

      {project.category && (
        <Badge tone={CATEGORY_COLORS[project.category as Category] ?? "#6b7280"}>
          {project.category}
        </Badge>
      )}

      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
        {project.path && (
          <>
            <Tooltip label="Ghostty에서 열기">
              <button
                onClick={() => onOpenGhostty(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-2)]"
                aria-label="Ghostty에서 열기"
              >
                <Icon icon={Play} size={14} />
              </button>
            </Tooltip>
            <Tooltip label="Finder에서 열기">
              <button
                onClick={() => onOpenFinder(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-2)]"
                aria-label="Finder에서 열기"
              >
                <Icon icon={FolderOpen} size={14} />
              </button>
            </Tooltip>
          </>
        )}
        <Tooltip label="삭제">
          <button
            onClick={() => onDelete(project.id)}
            className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]"
            aria-label="삭제"
          >
            <Icon icon={X} size={14} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
