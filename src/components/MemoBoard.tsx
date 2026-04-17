import { useMemo } from "react";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { Plus, StickyNote } from "lucide-react";
import { MemoCard } from "./MemoCard";
import { useMemos } from "../hooks/useMemos";
import { useProjects } from "../hooks/useProjects";
import { PRIORITIES } from "../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { globalSequence, groupMemosByProject } from "../lib/memoSequence";

export function MemoBoard() {
  const { memos, create, update, remove, reorder } = useMemos();
  // MemoBoard wants every project for the grouping + picker; `null` means
  // "no category filter" (전체 보기) so NULL-category rows are also included.
  const { projects } = useProjects(new Set(PRIORITIES), null);

  const groups = useMemo(
    () => groupMemosByProject(memos, projects),
    [memos, projects]
  );
  const seq = useMemo(() => globalSequence(memos), [memos]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  // Task 13 replaces this with the full cross-group reorder. For now keep the
  // flat within-list semantics so nothing regresses visually.
  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = memos.findIndex((m) => m.id === active.id);
    const newIndex = memos.findIndex((m) => m.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const reordered = [...memos];
    const [moved] = reordered.splice(oldIndex, 1);
    reordered.splice(newIndex, 0, moved);
    reorder(reordered.map((m) => m.id));
  };

  const handleCreate = () => {
    create({ content: "" });
  };

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="flex justify-between items-center mb-5">
        <h2 className="text-heading text-[var(--color-text-hi)]">메모보드</h2>
        <Button
          variant="primary"
          size="sm"
          leftIcon={Plus}
          onClick={handleCreate}
        >
          메모 추가
        </Button>
      </div>
      {memos.length === 0 ? (
        <EmptyState
          className="flex-1"
          icon={StickyNote}
          title="메모가 없습니다"
          description="⌘K 또는 메모 추가 버튼으로 시작하세요"
        />
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <div className="flex flex-col gap-6 flex-1 min-h-0">
            {groups.map((g) => {
              const key = g.kind === "project" ? `proj-${g.project.id}` : "etc";
              const title =
                g.kind === "project"
                  ? `${g.project.name} · ${g.project.priority}`
                  : "기타 · 프로젝트 미연결";
              return (
                <section key={key}>
                  <header className="mb-3 flex items-center gap-2 text-[12px] text-[var(--color-text-muted)] border-b border-[var(--color-border)] pb-1.5">
                    <span className="font-semibold text-[var(--color-text)]">
                      {title}
                    </span>
                    <span className="text-[var(--color-text-dim)]">
                      ({g.memos.length})
                    </span>
                  </header>
                  <SortableContext
                    items={g.memos.map((m) => m.id)}
                    strategy={rectSortingStrategy}
                  >
                    <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-5">
                      {g.memos.map((m) => (
                        <MemoCard
                          key={m.id}
                          memo={m}
                          projects={projects}
                          onUpdate={update}
                          onDelete={remove}
                          sequenceNumber={seq.get(m.id) ?? 0}
                        />
                      ))}
                    </div>
                  </SortableContext>
                </section>
              );
            })}
          </div>
        </DndContext>
      )}
    </div>
  );
}
