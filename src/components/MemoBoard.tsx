import { useEffect, useMemo, useState } from "react";
import {
  DndContext,
  DragOverlay,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent, DragStartEvent } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { Plus, StickyNote } from "lucide-react";
import { MemoCard } from "./MemoCard";
import { useMemos } from "../hooks/useMemos";
import { useProjects } from "../hooks/useProjects";
import { PRIORITIES } from "../types";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { useToast } from "../ui/Toast";
import { globalSequence, groupMemosByProject } from "../lib/memoSequence";
import * as api from "../api";

// Stable Set reference so `useProjects`'s effect deps don't churn every
// render (useProjects useCallback-s `load` on [priorities, category], and
// a fresh `new Set(...)` on every render re-creates `load` → refetches in
// a tight loop).
const ALL_PRIORITIES = new Set(PRIORITIES);

export function MemoBoard() {
  const { memos, update, remove, reload } = useMemos();
  // MemoBoard wants every project for the grouping + picker; `null` means
  // "no category filter" (전체 보기) so NULL-category rows are also included.
  const { projects } = useProjects(ALL_PRIORITIES, null);
  const toast = useToast();

  const groups = useMemo(
    () => groupMemosByProject(memos, projects),
    [memos, projects]
  );
  const seq = useMemo(() => globalSequence(memos), [memos]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const [activeId, setActiveId] = useState<number | null>(null);
  const [highlightedId, setHighlightedId] = useState<number | null>(null);

  // Listen for search-palette focus requests. We scroll the card into view
  // and trigger a one-shot glow via `find-highlight`. Re-keying on every
  // event (setHighlightedId(null) → id) so repeated clicks on the same memo
  // restart the animation instead of sitting on a static ring.
  useEffect(() => {
    const onFocus = (e: Event) => {
      const detail = (e as CustomEvent<{ memoId?: number }>).detail;
      const id = detail?.memoId;
      if (typeof id !== "number") return;
      setHighlightedId(null);
      requestAnimationFrame(() => {
        setHighlightedId(id);
        const el = document.querySelector<HTMLElement>(
          `[data-memo-id="${id}"]`
        );
        el?.scrollIntoView({ behavior: "smooth", block: "center" });
      });
      const t = window.setTimeout(() => setHighlightedId(null), 2000);
      return () => window.clearTimeout(t);
    };
    window.addEventListener("memo:focus", onFocus);
    return () => window.removeEventListener("memo:focus", onFocus);
  }, []);

  const handleDragStart = (e: DragStartEvent) => {
    if (typeof e.active.id === "number") setActiveId(e.active.id);
  };

  // Cross-group drag: re-parent the memo to the target group's project (or
  // null for 기타), then rebuild a flat id list in current display order so
  // the global #N badges stay stable for non-moved memos.
  const handleDragEnd = async (e: DragEndEvent) => {
    setActiveId(null);
    const { active, over } = e;
    if (!over || active.id === over.id) return;
    const activeIdNum =
      typeof active.id === "number" ? active.id : Number(active.id);
    const overIdNum = typeof over.id === "number" ? over.id : NaN;
    const sourceMemo = memos.find((m) => m.id === activeIdNum);
    const targetMemo = memos.find((m) => m.id === overIdNum);
    if (!sourceMemo || !targetMemo) return;

    const sameGroup = sourceMemo.project_id === targetMemo.project_id;
    const targetProjectId = targetMemo.project_id ?? null;

    // Speculative copy of the memo list with the source rebound to the
    // target group, so group rebuild below puts it in the right bucket.
    const nextMemos = memos.map((m) =>
      m.id === sourceMemo.id ? { ...m, project_id: targetProjectId } : m
    );

    // Reorder within the target group: take the rebound group, move source
    // to the target's index. This captures the drop position precisely.
    const targetGroupIds = nextMemos
      .filter((m) => (m.project_id ?? null) === targetProjectId)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((m) => m.id);
    const fromIdx = targetGroupIds.indexOf(sourceMemo.id);
    const toIdx = targetGroupIds.indexOf(targetMemo.id);
    if (fromIdx >= 0 && toIdx >= 0 && fromIdx !== toIdx) {
      const [moved] = targetGroupIds.splice(fromIdx, 1);
      targetGroupIds.splice(toIdx, 0, moved);
    }

    // Rebuild the flat order: each group keeps its current relative order,
    // except the target group uses the spliced list computed above.
    const groupsAfter = groupMemosByProject(nextMemos, projects);
    const fullIds: number[] = [];
    for (const g of groupsAfter) {
      const isTargetGroup =
        (g.kind === "project" &&
          targetProjectId !== null &&
          g.project.id === targetProjectId) ||
        (g.kind === "etc" && targetProjectId === null);
      if (isTargetGroup) {
        fullIds.push(...targetGroupIds);
      } else {
        fullIds.push(...g.memos.map((m) => m.id));
      }
    }

    try {
      if (!sameGroup) {
        // `undefined` would be omitted by the Rust serde layer; send an
        // explicit null so the column is set to SQL NULL for 기타 drops.
        await api.updateMemo(sourceMemo.id, { project_id: targetProjectId });
      }
      await api.reorderMemos(fullIds);
      await reload();
    } catch (err) {
      toast.error(`메모 이동 실패: ${err}`);
    }
  };

  const handleCreate = () => {
    window.dispatchEvent(new CustomEvent("memo:new-dialog"));
  };

  const activeMemo =
    activeId !== null ? memos.find((m) => m.id === activeId) ?? null : null;

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
          onDragStart={handleDragStart}
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
                          highlighted={m.id === highlightedId}
                        />
                      ))}
                    </div>
                  </SortableContext>
                </section>
              );
            })}
          </div>

          <DragOverlay>
            {activeMemo ? (
              <div className="rounded-xl bg-[var(--color-surface-1)] border border-[var(--color-brand)] px-3 py-2 text-[12px] text-[var(--color-text-hi)] shadow-lg max-w-[260px]">
                {activeMemo.content.slice(0, 80) || "(비어 있음)"}
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>
      )}
    </div>
  );
}
