import type { Memo, Project } from "../types";

export type FocusQuickFilter = "all" | "important" | "unlinked";

export type FocusFilters = {
  quick: FocusQuickFilter;
  category: string | null;
  tag: string | null;
};

export function clampFocusCoordinate(value: number) {
  if (Number.isNaN(value)) return 0;
  return Number(Math.min(1, Math.max(0, value)).toFixed(4));
}

export function clampFocusPositionForNote({
  value,
  boardSize,
  noteSize,
}: {
  value: number;
  boardSize: number;
  noteSize: number;
}) {
  if (Number.isNaN(value)) return 0;
  if (!Number.isFinite(boardSize) || boardSize <= 0) {
    return clampFocusCoordinate(value);
  }

  const safeNoteSize = Number.isFinite(noteSize) ? Math.max(0, noteSize) : 0;
  const max = Math.max(0, (boardSize - safeNoteSize) / boardSize);
  return Number(Math.min(max, Math.max(0, value)).toFixed(4));
}

export function defaultFocusPosition(index: number) {
  const safeIndex = Math.max(0, Math.floor(index));
  return {
    x: clampFocusCoordinate(Math.min(0.82, 0.08 + (safeIndex % 4) * 0.21)),
    y: clampFocusCoordinate(
      Math.min(0.82, 0.1 + Math.floor(safeIndex / 4) * 0.18),
    ),
  };
}

export function memoIsImportant(
  memo: Pick<Memo, "font_size" | "is_bold" | "tags">,
) {
  return (
    memo.is_bold ||
    memo.font_size === "large" ||
    memo.tags.some((tag) => tag.name === "중요")
  );
}

export function filterFocusMemos(
  memos: Memo[],
  projects: Project[],
  filters: FocusFilters,
) {
  const projectById = new Map(projects.map((project) => [project.id, project]));

  return memos.filter((memo) => {
    if (filters.quick === "important" && !memoIsImportant(memo)) return false;
    if (filters.quick === "unlinked" && memo.project_id !== null) return false;

    if (filters.category) {
      const project =
        memo.project_id === null ? null : projectById.get(memo.project_id);
      if (project?.category !== filters.category) return false;
    }

    if (filters.tag && !memo.tags.some((tag) => tag.name === filters.tag)) {
      return false;
    }

    return true;
  });
}
