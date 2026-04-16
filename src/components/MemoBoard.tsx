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
import { PRIORITIES, CATEGORIES } from "../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";

export function MemoBoard() {
  const { memos, create, update, remove, reorder } = useMemos();
  const { projects } = useProjects(new Set(PRIORITIES), new Set(CATEGORIES));

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

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
        <Button variant="primary" size="sm" leftIcon={Plus} onClick={handleCreate}>
          메모 추가
        </Button>
      </div>
      {memos.length === 0 ? (
        <EmptyState className="flex-1" icon={StickyNote} title="메모가 없습니다" description="⌘K 또는 메모 추가 버튼으로 시작하세요" />
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <SortableContext
            items={memos.map((m) => m.id)}
            strategy={rectSortingStrategy}
          >
            <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-5">
              {memos.map((memo) => (
                <MemoCard
                  key={memo.id}
                  memo={memo}
                  projects={projects}
                  onUpdate={update}
                  onDelete={remove}
                />
              ))}
              <button
                onClick={handleCreate}
                className="rounded-xl border-2 border-dashed border-[var(--color-border)] min-h-[160px] flex items-center justify-center text-[var(--color-text-muted)] hover:border-[var(--color-brand)] hover:text-[var(--color-brand-hi)] transition-colors text-sm"
              >
                + 새 메모
              </button>
            </div>
          </SortableContext>
        </DndContext>
      )}
    </div>
  );
}
