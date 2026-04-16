import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { MemoCard } from "./MemoCard";
import { useMemos } from "../hooks/useMemos";
import { useProjects } from "../hooks/useProjects";
import { PRIORITIES, CATEGORIES } from "../types";

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
    <div>
      <div className="flex justify-between items-center mb-4">
        <h2 className="text-lg font-semibold">메모보드</h2>
        <button
          onClick={handleCreate}
          className="px-3 py-1.5 text-sm rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
        >
          + 새 메모
        </button>
      </div>
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={memos.map((m) => m.id)}
          strategy={rectSortingStrategy}
        >
          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
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
              className="rounded-xl border-2 border-dashed border-[var(--border-color)] min-h-[140px] flex items-center justify-center text-[var(--text-secondary)] hover:border-[var(--accent)] hover:text-[var(--accent)] transition-colors"
            >
              + 새 메모
            </button>
          </div>
        </SortableContext>
      </DndContext>
    </div>
  );
}
