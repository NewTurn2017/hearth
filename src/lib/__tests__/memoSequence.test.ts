import { describe, it, expect } from "vitest";
import { globalSequence, groupMemosByProject } from "../memoSequence";
import type { Memo, Project } from "../../types";

const mkMemo = (
  id: number,
  sort_order: number,
  project_id: number | null = null
): Memo => ({
  id,
  sort_order,
  project_id,
  content: `memo${id}`,
  color: "yellow",
  created_at: "",
  updated_at: "",
});

const mkProj = (
  id: number,
  priority: Project["priority"],
  sort_order = 0
): Project => ({
  id,
  priority,
  number: null,
  name: `p${id}`,
  category: null,
  path: null,
  evaluation: null,
  sort_order,
  created_at: "",
  updated_at: "",
});

describe("globalSequence", () => {
  it("assigns 1..N by sort_order", () => {
    const memos = [mkMemo(10, 2), mkMemo(20, 0), mkMemo(30, 1)];
    const seq = globalSequence(memos);
    expect(seq.get(20)).toBe(1);
    expect(seq.get(30)).toBe(2);
    expect(seq.get(10)).toBe(3);
  });

  it("is stable when memos are already ordered", () => {
    const memos = [mkMemo(1, 0), mkMemo(2, 1), mkMemo(3, 2)];
    const seq = globalSequence(memos);
    expect([...seq.entries()]).toEqual([
      [1, 1],
      [2, 2],
      [3, 3],
    ]);
  });
});

describe("groupMemosByProject", () => {
  it("groups by project priority then sort_order, with 기타 trailing", () => {
    const projects = [
      mkProj(1, "P2", 0),
      mkProj(2, "P0", 0),
      mkProj(3, "P0", 1),
    ];
    const memos = [
      mkMemo(10, 0, 1),
      mkMemo(11, 1, 2),
      mkMemo(12, 2, 3),
      mkMemo(13, 3, null),
    ];
    const groups = groupMemosByProject(memos, projects);
    expect(
      groups.map((g) => (g.kind === "project" ? g.project.id : "etc"))
    ).toEqual([2, 3, 1, "etc"]);
    expect(groups[0].memos.map((m) => m.id)).toEqual([11]);
    expect(groups[3].memos.map((m) => m.id)).toEqual([13]);
  });

  it("omits empty groups", () => {
    const projects = [mkProj(1, "P0", 0), mkProj(2, "P1", 0)];
    const memos = [mkMemo(10, 0, 1)];
    const groups = groupMemosByProject(memos, projects);
    expect(groups).toHaveLength(1);
    expect(groups[0].kind).toBe("project");
  });
});
