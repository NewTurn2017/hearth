import { describe, expect, it } from "vitest";
import type { Memo, Project } from "../../types";
import {
  clampFocusCoordinate,
  defaultFocusPosition,
  filterFocusMemos,
  memoIsImportant,
} from "../focusMemoLayout";

const baseMemo = (overrides: Partial<Memo>): Memo => ({
  id: 1,
  content: "memo",
  color: "yellow",
  project_id: null,
  sort_order: 0,
  font_size: "normal",
  is_bold: false,
  focus_x: null,
  focus_y: null,
  tags: [],
  created_at: "",
  updated_at: "",
  ...overrides,
});

const tag = (name: string) => ({
  id: name.length,
  name,
  color: "#999999",
  sort_order: 0,
  usage_count: 1,
  created_at: "",
  updated_at: "",
});

const project = (overrides: Partial<Project>): Project => ({
  id: 1,
  priority: "P2",
  number: null,
  name: "Project",
  category: null,
  path: null,
  evaluation: null,
  sort_order: 0,
  created_at: "",
  updated_at: "",
  ...overrides,
});

describe("focus memo layout helpers", () => {
  it("clamps normalized coordinates with stable precision", () => {
    expect(clampFocusCoordinate(Number.NaN)).toBe(0);
    expect(clampFocusCoordinate(-0.1)).toBe(0);
    expect(clampFocusCoordinate(1.4)).toBe(1);
    expect(clampFocusCoordinate(0.123456)).toBe(0.1235);
  });

  it("generates deterministic cascaded defaults inside the board", () => {
    expect(defaultFocusPosition(0)).toEqual({ x: 0.08, y: 0.1 });
    expect(defaultFocusPosition(3)).toEqual({ x: 0.71, y: 0.1 });
    expect(defaultFocusPosition(4)).toEqual({ x: 0.08, y: 0.28 });
    expect(defaultFocusPosition(100)).toEqual({ x: 0.08, y: 0.82 });
  });

  it("marks bold, large, or 중요-tagged memos as important", () => {
    expect(memoIsImportant(baseMemo({ is_bold: true }))).toBe(true);
    expect(memoIsImportant(baseMemo({ font_size: "large" }))).toBe(true);
    expect(memoIsImportant(baseMemo({ tags: [tag("중요")] }))).toBe(true);
    expect(memoIsImportant(baseMemo({ tags: [tag("나중에")] }))).toBe(false);
  });

  it("filters by quick important state", () => {
    const memos = [
      baseMemo({ id: 1, content: "plain" }),
      baseMemo({ id: 2, content: "bold", is_bold: true }),
      baseMemo({ id: 3, content: "large", font_size: "large" }),
      baseMemo({ id: 4, content: "tagged", tags: [tag("중요")] }),
    ];

    expect(
      filterFocusMemos(memos, [], {
        quick: "important",
        category: null,
        tag: null,
      }).map((memo) => memo.id),
    ).toEqual([2, 3, 4]);
  });

  it("filters by linked project category and memo tag", () => {
    const projects = [
      project({ id: 10, category: "Tools" }),
      project({ id: 11, category: "Lab" }),
    ];
    const memos = [
      baseMemo({ id: 1, project_id: 10, tags: [tag("UI")] }),
      baseMemo({ id: 2, project_id: 11, tags: [tag("UI")] }),
      baseMemo({ id: 3, project_id: 10, tags: [tag("Ops")] }),
    ];

    expect(
      filterFocusMemos(memos, projects, {
        quick: "all",
        category: "Tools",
        tag: "UI",
      }).map((memo) => memo.id),
    ).toEqual([1]);
  });
});
