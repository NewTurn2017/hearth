import { useMemo, useState, type ReactNode } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { Plus, FolderOpen } from "lucide-react";
import { ProjectCard } from "./ProjectCard";
import { EmptyDropZone } from "./EmptyDropZone";
import { deriveTarget } from "../lib/dragTargets";
import type { Project, Priority } from "../types";
import { PRIORITIES, PRIORITY_COLORS, PRIORITY_LABELS } from "../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import * as api from "../api";

/**
 * Wraps a priority group's card grid so dropping on gutters / whitespace
 * still counts as a drop on that group (append at end). Without this, only
 * direct card hovers registered, which made cross-group moves feel like a
 * tiny target. `closestCorners` will still prefer a specific card when the
 * pointer is over one — the wrapper wins only on empty space.
 */
function GroupDropZone({
  priority,
  active,
  children,
}: {
  priority: Priority;
  active: boolean;
  children: ReactNode;
}) {
  const { setNodeRef, isOver } = useDroppable({ id: `priority-${priority}-group` });
  return (
    <div
      ref={setNodeRef}
      className={cn(
        "w-full rounded-[var(--radius-md)] p-1 transition-colors",
        active && "border-2 border-dashed border-[var(--color-border)]",
        active && isOver && "border-[var(--color-brand)] bg-[var(--color-surface-2)]"
      )}
    >
      {children}
    </div>
  );
}

export function ProjectList({
  projects,
  onUpdate,
  onDelete,
  onReorder,
  onAdd,
  onOpenDetail,
}: {
  projects: Project[];
  onUpdate: (
    id: number,
    fields: Record<string, string>
  ) => Promise<unknown> | void;
  onDelete: (id: number) => void;
  onReorder: (priority: string, ids: number[]) => Promise<unknown> | void;
  onAdd?: () => void;
  onOpenDetail: (project: Project) => void;
}) {
  const toast = useToast();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );
  const [activeId, setActiveId] = useState<number | null>(null);

  const groups = useMemo(() => {
    const map = new Map<Priority, Project[]>();
    for (const p of PRIORITIES) map.set(p, []);
    for (const p of projects) {
      if ((PRIORITIES as readonly string[]).includes(p.priority)) {
        map.get(p.priority as Priority)!.push(p);
      }
    }
    return map;
  }, [projects]);

  const idsOf = (priority: Priority) =>
    (groups.get(priority) ?? []).map((p) => p.id);

  const activeProject =
    activeId === null ? null : projects.find((p) => p.id === activeId) ?? null;

  const handleDragStart = (e: DragStartEvent) => {
    if (typeof e.active.id === "number") setActiveId(e.active.id);
  };

  const handleDragEnd = async (e: DragEndEvent) => {
    setActiveId(null);
    const { active, over } = e;
    if (!over) return;
    const activeNumId =
      typeof active.id === "number" ? active.id : Number(active.id);
    const target = deriveTarget(
      over.id as string | number,
      projects
    );
    if (!target) return;
    const source = projects.find((p) => p.id === activeNumId);
    if (!source) return;

    // Same-group reorder: splice the id into its new slot and persist.
    if (source.priority === target.priority) {
      if (activeNumId === over.id) return;
      const groupIds = idsOf(target.priority);
      const from = groupIds.indexOf(activeNumId);
      const to =
        target.overId === null
          ? groupIds.length - 1
          : groupIds.indexOf(target.overId);
      if (from < 0 || to < 0) return;
      const next = [...groupIds];
      const [moved] = next.splice(from, 1);
      next.splice(to, 0, moved);
      try {
        await onReorder(target.priority, next);
      } catch (err) {
        toast.error(`순서 저장 실패: ${err}`);
      }
      return;
    }

    // Cross-group move: change the priority first (so the backend sees the
    // card in the right bucket), then reorder the target group with the id
    // inserted at the drop position, then reorder the source group without
    // it. Serialized so the next refetch sees canonical state.
    const sourcePriority = source.priority as Priority;
    const targetGroupIds = idsOf(target.priority);
    const sourceGroupIds = idsOf(sourcePriority);
    const insertAt =
      target.overId === null
        ? targetGroupIds.length
        : targetGroupIds.indexOf(target.overId);
    const nextTargetIds = [...targetGroupIds];
    nextTargetIds.splice(Math.max(0, insertAt), 0, activeNumId);
    const nextSourceIds = sourceGroupIds.filter((id) => id !== activeNumId);

    try {
      await onUpdate(activeNumId, { priority: target.priority });
      await onReorder(target.priority, nextTargetIds);
      await onReorder(sourcePriority, nextSourceIds);
    } catch (err) {
      toast.error(`우선순위 변경 실패: ${err}`);
    }
  };

  if (projects.length === 0) {
    return (
      <EmptyState
        className="flex-1"
        icon={FolderOpen}
        title="프로젝트가 없습니다"
        description="Excel로 가져오거나 ⌘K 로 추가하세요"
        action={
          onAdd && (
            <Button variant="primary" size="sm" leftIcon={Plus} onClick={onAdd}>
              프로젝트 추가
            </Button>
          )
        }
      />
    );
  }

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="flex justify-between items-center mb-5">
        <h2 className="text-heading text-[var(--color-text-hi)] flex items-center gap-2">
          <FolderOpen size={18} />
          프로젝트
        </h2>
        {onAdd && (
          <Button variant="primary" size="sm" leftIcon={Plus} onClick={onAdd}>
            프로젝트 추가
          </Button>
        )}
      </div>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="flex flex-col gap-7 flex-1 min-h-0">
          {PRIORITIES.map((priority) => {
            const items = groups.get(priority) ?? [];
            return (
              <div key={priority}>
                <div className="flex items-center gap-2 mb-2">
                  <span
                    className="w-3 h-3 rounded-full"
                    style={{
                      backgroundColor: PRIORITY_COLORS[priority] ?? "#6b7280",
                    }}
                  />
                  <h2 className="text-sm font-semibold text-[var(--color-text)]">
                    {priority} — {PRIORITY_LABELS[priority] ?? priority}
                  </h2>
                  <span className="text-xs text-[var(--color-text-muted)]">
                    ({items.length}개)
                  </span>
                </div>
                <SortableContext
                  items={items.map((p) => p.id)}
                  strategy={rectSortingStrategy}
                >
                  {items.length === 0 ? (
                    <EmptyDropZone
                      id={`priority-${priority}-empty`}
                      label={`${priority} 비어 있음 · 드래그해서 추가`}
                    />
                  ) : (
                    <GroupDropZone
                      priority={priority}
                      active={activeId !== null}
                    >
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                        {items.map((project) => (
                          <ProjectCard
                            key={project.id}
                            project={project}
                            onUpdate={onUpdate}
                            onDelete={onDelete}
                            onOpenGhostty={(path) => api.openInGhostty(path)}
                            onOpenFinder={(path) => api.openInFinder(path)}
                            onOpenDetail={onOpenDetail}
                          />
                        ))}
                      </div>
                    </GroupDropZone>
                  )}
                </SortableContext>
              </div>
            );
          })}
        </div>

        <DragOverlay>
          {activeProject ? (
            <div className="rounded-[var(--radius-md)] border border-[var(--color-brand)] bg-[var(--color-surface-1)] px-3 py-2.5 shadow-lg text-[14px] font-semibold text-[var(--color-text-hi)]">
              {activeProject.name}
            </div>
          ) : null}
        </DragOverlay>
      </DndContext>
    </div>
  );
}
