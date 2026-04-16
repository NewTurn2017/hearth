import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { Plus, FolderOpen } from "lucide-react";
import { ProjectCard } from "./ProjectCard";
import type { Project, Priority } from "../types";
import { PRIORITY_COLORS, PRIORITY_LABELS } from "../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import * as api from "../api";

export function ProjectList({
  projects,
  onUpdate,
  onDelete,
  onReorder,
  onAdd,
}: {
  projects: Project[];
  onUpdate: (id: number, fields: Record<string, string>) => void;
  onDelete: (id: number) => void;
  onReorder: (priority: string, ids: number[]) => void;
  onAdd?: () => void;
}) {
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const groups = new Map<string, Project[]>();
  for (const p of projects) {
    const list = groups.get(p.priority) ?? [];
    list.push(p);
    groups.set(p.priority, list);
  }

  const handleDragEnd = (priority: string) => (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const group = groups.get(priority) ?? [];
    const oldIndex = group.findIndex((p) => p.id === active.id);
    const newIndex = group.findIndex((p) => p.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const reordered = [...group];
    const [moved] = reordered.splice(oldIndex, 1);
    reordered.splice(newIndex, 0, moved);
    onReorder(priority, reordered.map((p) => p.id));
  };

  if (projects.length === 0) {
    return (
      <EmptyState
        icon={FolderOpen}
        title="프로젝트가 없습니다"
        description="Excel로 가져오거나 ⌘K 로 추가하세요"
        action={onAdd && <Button variant="primary" size="sm" leftIcon={Plus} onClick={onAdd}>프로젝트 추가</Button>}
      />
    );
  }

  return (
    <div className="flex flex-col gap-6">
      {[...groups.entries()].map(([priority, items]) => (
        <div key={priority}>
          <div className="flex items-center gap-2 mb-2">
            <span
              className="w-3 h-3 rounded-full"
              style={{
                backgroundColor:
                  PRIORITY_COLORS[priority as Priority] ?? "#6b7280",
              }}
            />
            <h2 className="text-sm font-semibold text-[var(--color-text)]">
              {priority} — {PRIORITY_LABELS[priority as Priority] ?? priority}
            </h2>
            <span className="text-xs text-[var(--color-text-muted)]">
              ({items.length}개)
            </span>
          </div>
          <DndContext
            sensors={sensors}
            collisionDetection={closestCenter}
            onDragEnd={handleDragEnd(priority)}
          >
            <SortableContext
              items={items.map((p) => p.id)}
              strategy={verticalListSortingStrategy}
            >
              <div className="flex flex-col gap-1">
                {items.map((project) => (
                  <ProjectCard
                    key={project.id}
                    project={project}
                    onUpdate={onUpdate}
                    onDelete={onDelete}
                    onOpenGhostty={(path) => api.openInGhostty(path)}
                    onOpenFinder={(path) => api.openInFinder(path)}
                  />
                ))}
              </div>
            </SortableContext>
          </DndContext>
        </div>
      ))}
    </div>
  );
}
