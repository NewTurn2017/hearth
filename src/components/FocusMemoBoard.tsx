import { useMemo, useRef, useState, type ReactNode } from "react";
import {
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import type { CategoryRow, Memo, MemoTag, Project } from "../types";
import type { MemoUpdateInput } from "../api";
import { cn } from "../lib/cn";
import {
  clampFocusCoordinate,
  clampFocusPositionForNote,
  defaultFocusPosition,
  filterFocusMemos,
  type FocusFilters,
  type FocusQuickFilter,
} from "../lib/focusMemoLayout";
import { useToast } from "../ui/Toast";
import { FocusMemoNote } from "./FocusMemoNote";

const QUICK_FILTERS: { value: FocusQuickFilter; label: string }[] = [
  { value: "all", label: "전체" },
  { value: "important", label: "중요" },
  { value: "unlinked", label: "미연결" },
];

const FALLBACK_BOARD_WIDTH = 1000;
const FALLBACK_BOARD_HEIGHT = 560;
const FOCUS_NOTE_WIDTH = 210;
const FOCUS_NOTE_HEIGHT = 140;

export function FocusMemoBoard({
  memos,
  projects,
  categories,
  tags,
  sequence,
  highlightedId,
  onUpdate,
  onDelete,
  onReload,
  onCreateTag,
}: {
  memos: Memo[];
  projects: Project[];
  categories: CategoryRow[];
  tags: MemoTag[];
  sequence: Map<number, number>;
  highlightedId: number | null;
  onUpdate: (id: number, fields: MemoUpdateInput) => void | Promise<unknown>;
  onDelete: (id: number) => void;
  onReload: () => void | Promise<unknown>;
  onCreateTag: (name: string) => Promise<MemoTag>;
}) {
  const [filters, setFilters] = useState<FocusFilters>({
    quick: "all",
    category: null,
    tag: null,
  });
  const boardRef = useRef<HTMLDivElement>(null);
  const toast = useToast();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } }),
  );

  const defaultIndexByMemoId = useMemo(() => {
    // Defaults are anchored to the full memo order, not the filtered order, so
    // a memo without stored focus_x/y does not jump when rail filters change.
    return new Map(memos.map((memo, index) => [memo.id, index]));
  }, [memos]);

  const visibleMemos = useMemo(
    () => filterFocusMemos(memos, projects, filters),
    [filters, memos, projects],
  );

  const measuredBoardDimensions = () => {
    const rect = boardRef.current?.getBoundingClientRect();
    if (!rect || rect.width <= 0 || rect.height <= 0) return null;
    return { width: rect.width, height: rect.height };
  };

  const boardDimensions = () =>
    measuredBoardDimensions() ?? {
      width: FALLBACK_BOARD_WIDTH,
      height: FALLBACK_BOARD_HEIGHT,
    };

  const noteDimensions = (memoId: number) => {
    const noteEl = boardRef.current?.querySelector<HTMLElement>(
      `[data-memo-id="${memoId}"]`,
    );
    const rect = noteEl?.getBoundingClientRect();
    return {
      width: rect && rect.width > 0 ? rect.width : FOCUS_NOTE_WIDTH,
      height: rect && rect.height > 0 ? rect.height : FOCUS_NOTE_HEIGHT,
    };
  };

  const positionFor = (memo: Memo) => {
    const fallback = defaultFocusPosition(
      defaultIndexByMemoId.get(memo.id) ?? 0,
    );
    const value = {
      x: memo.focus_x === null ? fallback.x : memo.focus_x,
      y: memo.focus_y === null ? fallback.y : memo.focus_y,
    };
    const board = measuredBoardDimensions();
    if (!board) {
      return {
        x: clampFocusCoordinate(value.x),
        y: clampFocusCoordinate(value.y),
      };
    }

    const note = noteDimensions(memo.id);
    return {
      x: clampFocusPositionForNote({
        value: value.x,
        boardSize: board.width,
        noteSize: note.width,
      }),
      y: clampFocusPositionForNote({
        value: value.y,
        boardSize: board.height,
        noteSize: note.height,
      }),
    };
  };

  const handleDragEnd = async (event: DragEndEvent) => {
    const id =
      typeof event.active.id === "number"
        ? event.active.id
        : Number(event.active.id);
    const memo = memos.find((item) => item.id === id);
    if (!memo) return;

    const board = boardDimensions();
    const note = noteDimensions(memo.id);
    const current = positionFor(memo);
    const focus_x = clampFocusPositionForNote({
      value: current.x + event.delta.x / board.width,
      boardSize: board.width,
      noteSize: note.width,
    });
    const focus_y = clampFocusPositionForNote({
      value: current.y + event.delta.y / board.height,
      boardSize: board.height,
      noteSize: note.height,
    });

    try {
      await onUpdate(memo.id, { focus_x, focus_y });
    } catch (error) {
      toast.error(`포커스 위치 저장 실패: ${error}`);
      await onReload();
    }
  };

  return (
    <div className="flex flex-1 min-h-0 gap-4">
      <aside className="w-[180px] shrink-0 overflow-y-auto rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-1)] p-3">
        <RailSection title="빠른 필터">
          {QUICK_FILTERS.map((quick) => (
            <RailButton
              key={quick.value}
              active={filters.quick === quick.value}
              onClick={() =>
                setFilters((prev) => ({ ...prev, quick: quick.value }))
              }
            >
              {quick.label}
            </RailButton>
          ))}
        </RailSection>

        <RailSection title="카테고리">
          <RailButton
            active={filters.category === null}
            onClick={() => setFilters((prev) => ({ ...prev, category: null }))}
          >
            전체
          </RailButton>
          {categories.map((category) => (
            <RailButton
              key={category.id}
              active={filters.category === category.name}
              onClick={() =>
                setFilters((prev) => ({ ...prev, category: category.name }))
              }
            >
              {category.name}
            </RailButton>
          ))}
        </RailSection>

        <RailSection title="메모 태그">
          <RailButton
            active={filters.tag === null}
            onClick={() => setFilters((prev) => ({ ...prev, tag: null }))}
          >
            전체
          </RailButton>
          {tags.map((tag) => (
            <RailButton
              key={tag.id}
              active={filters.tag === tag.name}
              onClick={() => setFilters((prev) => ({ ...prev, tag: tag.name }))}
            >
              #{tag.name}
            </RailButton>
          ))}
        </RailSection>
      </aside>

      <DndContext sensors={sensors} onDragEnd={handleDragEnd}>
        <div
          ref={boardRef}
          data-testid="focus-board-surface"
          className={cn(
            "relative flex-1 min-h-[560px] overflow-hidden rounded-2xl border border-[var(--color-border)]",
            "bg-[radial-gradient(circle_at_top_left,rgba(255,255,255,0.08),transparent_34%),var(--color-surface-0)]",
            "shadow-[inset_0_0_0_1px_rgba(255,255,255,0.04),0_18px_60px_rgba(0,0,0,0.22)]",
          )}
        >
          <div className="pointer-events-none absolute inset-4 rounded-xl border border-white/5" />
          {visibleMemos.map((memo) => {
            const position = positionFor(memo);
            return (
              <FocusMemoNote
                key={memo.id}
                memo={memo}
                projects={projects}
                tags={tags}
                x={position.x}
                y={position.y}
                boundWidth={FOCUS_NOTE_WIDTH}
                boundHeight={FOCUS_NOTE_HEIGHT}
                sequenceNumber={sequence.get(memo.id) ?? 0}
                highlighted={memo.id === highlightedId}
                onUpdate={onUpdate}
                onDelete={onDelete}
                onCreateTag={onCreateTag}
              />
            );
          })}
        </div>
      </DndContext>
    </div>
  );
}

function RailSection({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="mb-4 last:mb-0">
      <h3 className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-[var(--color-text-dim)]">
        {title}
      </h3>
      <div className="flex flex-col gap-1">{children}</div>
    </section>
  );
}

function RailButton({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "h-7 rounded-md px-2 text-left text-[12px] transition-colors",
        active
          ? "bg-[var(--color-surface-3)] text-[var(--color-text-hi)]"
          : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]",
      )}
    >
      {children}
    </button>
  );
}
