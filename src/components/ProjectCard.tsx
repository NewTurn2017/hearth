import { useState, type MouseEvent } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Play, FolderOpen, X, Settings2, StickyNote, Trash2 } from "lucide-react";
import type { Project, Priority } from "../types";
import {
  CATEGORY_COLORS,
  PRIORITIES,
  PRIORITY_COLORS,
  PRIORITY_LABELS,
} from "../types";
import { Badge } from "../ui/Badge";
import { Icon } from "../ui/Icon";
import { Popover } from "../ui/Popover";
import { Tooltip } from "../ui/Tooltip";
import { cn } from "../lib/cn";
import { useCategories } from "../hooks/useCategories";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu, type ContextMenuItem } from "../ui/ContextMenu";
import * as api from "../api";

export function ProjectCard({
  project,
  onUpdate,
  onDelete,
  onOpenTerminal,
  onOpenFinder,
  onOpenDetail,
  highlighted,
}: {
  project: Project;
  onUpdate: (id: number, fields: Record<string, string>) => void;
  onDelete: (id: number) => void;
  onOpenTerminal: (path: string) => void;
  onOpenFinder: (path: string) => void;
  onOpenDetail: (project: Project) => void;
  highlighted?: boolean;
}) {
  const [editing, setEditing] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: project.id });
  const { categories } = useCategories();
  const { menu, open: openMenu, close: closeMenu } = useContextMenu();

  const menuItems: ContextMenuItem[] = [
    {
      id: "add-memo",
      label: "프로젝트 메모 추가",
      icon: StickyNote,
      onSelect: () =>
        window.dispatchEvent(
          new CustomEvent("memo:new-dialog", { detail: { projectId: project.id } })
        ),
    },
    {
      id: "settings",
      label: "프로젝트 설정",
      icon: Settings2,
      onSelect: () => onOpenDetail(project),
    },
    ...(project.path
      ? ([
          {
            id: "terminal",
            label: "터미널에서 열기",
            icon: Play,
            onSelect: () => api.openInTerminal(project.path!),
          },
          {
            id: "finder",
            label: "Finder에서 열기",
            icon: FolderOpen,
            onSelect: () => api.openInFinder(project.path!),
          },
        ] as ContextMenuItem[])
      : []),
    { id: "sep", label: "", separator: true, onSelect: () => {} },
    {
      id: "delete",
      label: "삭제",
      icon: Trash2,
      danger: true,
      onSelect: () => onDelete(project.id),
    },
  ];

  const catRow = categories.find((c) => c.name === project.category);
  const catColor =
    catRow?.color ??
    (project.category
      ? (CATEGORY_COLORS as Record<string, string | undefined>)[project.category] ??
        "#6b7280"
      : "#6b7280");

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

  // The whole card listens for a double click to open the detail dialog;
  // every interactive child swallows click / double-click so those actions
  // don't accidentally trigger navigation.
  const stop = (e: MouseEvent) => e.stopPropagation();

  return (
    <div
      ref={setNodeRef}
      style={style}
      data-project-id={project.id}
      onDoubleClick={() => onOpenDetail(project)}
      onContextMenu={openMenu}
      className={cn(
        "group relative flex flex-col gap-2 p-3 rounded-[var(--radius-md)]",
        "bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)]",
        "border border-transparent hover:border-[var(--color-border)]",
        "transition-colors duration-[120ms] cursor-default select-none",
        highlighted && "find-highlight"
      )}
    >
      <div
        className="absolute top-2 right-2 flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity"
        onClick={stop}
        onDoubleClick={stop}
      >
        {project.path && (
          <>
            <Tooltip label="터미널에서 열기">
              <button
                onClick={() => onOpenTerminal(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-1)]"
                aria-label="터미널에서 열기"
              >
                <Icon icon={Play} size={14} />
              </button>
            </Tooltip>
            <Tooltip label="Finder에서 열기">
              <button
                onClick={() => onOpenFinder(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-1)]"
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

      <div className="flex items-start gap-2 pr-20">
        <button
          {...attributes}
          {...listeners}
          onClick={stop}
          onDoubleClick={stop}
          className="mt-0.5 cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)] shrink-0"
          aria-label="드래그하여 순서 변경"
        >
          <Icon icon={GripVertical} size={16} />
        </button>
        <div className="flex-1 min-w-0" onDoubleClick={(e) => {
          // Name area should still let users double-click elsewhere on the
          // card to open details, but single clicks here go to inline edit.
          // The root's onDoubleClick will handle it — this guard only stops
          // the event when the user is mid-edit.
          if (editing) e.stopPropagation();
        }}>
          {editing === "name" ? (
            <input
              autoFocus
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={commitEdit}
              onKeyDown={(e) => e.key === "Enter" && commitEdit()}
              onClick={stop}
              onDoubleClick={stop}
              className="bg-transparent border-b border-[var(--color-brand-hi)] outline-none text-[14px] font-medium w-full text-[var(--color-text)]"
            />
          ) : (
            <button
              type="button"
              onClick={(e) => {
                stop(e);
                startEdit("name", project.name);
              }}
              onDoubleClick={stop}
              className="text-[14px] font-semibold text-[var(--color-text-hi)] cursor-text truncate block w-full text-left"
            >
              {project.name}
            </button>
          )}
        </div>
      </div>

      <div
        className="flex items-center gap-1.5 flex-wrap"
        onClick={stop}
        onDoubleClick={stop}
      >
        <Popover
          trigger={({ onClick, "aria-expanded": ae }) => (
            <button
              type="button"
              onClick={onClick}
              aria-expanded={ae}
              aria-label={`우선순위 변경 — 현재 ${project.priority}`}
              className="shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-brand)] rounded-full"
            >
              <Badge tone={PRIORITY_COLORS[project.priority as Priority] ?? "#6b7280"}>
                {project.priority}
              </Badge>
            </button>
          )}
        >
          {({ close }) => (
            <div className="flex flex-col">
              {PRIORITIES.map((p) => (
                <button
                  key={p}
                  type="button"
                  onClick={() => {
                    if (p !== project.priority) onUpdate(project.id, { priority: p });
                    close();
                  }}
                  className={cn(
                    "flex items-center gap-2 px-2 h-7 text-[12px] text-left rounded",
                    "hover:bg-[var(--color-surface-3)]",
                    p === project.priority && "bg-[var(--color-surface-3)]"
                  )}
                >
                  <span
                    className="w-2 h-2 rounded-full"
                    style={{ backgroundColor: PRIORITY_COLORS[p] }}
                  />
                  <span className="font-medium text-[var(--color-text)]">{p}</span>
                  <span className="text-[var(--color-text-dim)]">
                    {PRIORITY_LABELS[p]}
                  </span>
                </button>
              ))}
            </div>
          )}
        </Popover>

        <Popover
          trigger={({ onClick, "aria-expanded": ae }) => (
            <button
              type="button"
              onClick={onClick}
              aria-expanded={ae}
              aria-label={`카테고리 변경 — 현재 ${project.category ?? "없음"}`}
              className="shrink-0 focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-brand)] rounded-full"
            >
              {project.category ? (
                <Badge tone={catColor}>{project.category}</Badge>
              ) : (
                <Badge>카테고리</Badge>
              )}
            </button>
          )}
        >
          {({ close }) => (
            <div className="flex flex-col">
              {categories.map((c) => (
                <button
                  key={c.id}
                  type="button"
                  onClick={() => {
                    if (c.name !== project.category)
                      onUpdate(project.id, { category: c.name });
                    close();
                  }}
                  className={cn(
                    "flex items-center gap-2 px-2 h-7 text-[12px] text-left rounded",
                    "hover:bg-[var(--color-surface-3)]",
                    c.name === project.category && "bg-[var(--color-surface-3)]"
                  )}
                >
                  <span
                    className="w-2 h-2 rounded-full"
                    style={{ backgroundColor: c.color }}
                  />
                  <span className="text-[var(--color-text)]">{c.name}</span>
                </button>
              ))}
            </div>
          )}
        </Popover>
      </div>

      <div onClick={stop} onDoubleClick={stop}>
        {editing === "evaluation" ? (
          <textarea
            autoFocus
            rows={3}
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => {
              if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) commitEdit();
            }}
            className="bg-transparent border border-[var(--color-border)] outline-none text-[12px] w-full text-[var(--color-text-muted)] rounded-[var(--radius-sm)] px-2 py-1 focus:border-[var(--color-brand-hi)]"
          />
        ) : (
          <button
            type="button"
            onClick={() => startEdit("evaluation", project.evaluation ?? "")}
            className="block w-full text-left text-[12px] text-[var(--color-text-muted)] cursor-text line-clamp-3 leading-snug"
          >
            {project.evaluation || (
              <span className="text-[var(--color-text-dim)]">메모 없음</span>
            )}
          </button>
        )}
      </div>

      {project.path && (
        <div className="text-[11px] font-mono text-[var(--color-text-dim)] truncate">
          {project.path}
        </div>
      )}

      <ContextMenu
        open={menu.open}
        x={menu.x}
        y={menu.y}
        items={menuItems}
        onClose={closeMenu}
      />
    </div>
  );
}
