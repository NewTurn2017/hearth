import type { Memo, Project, Priority } from "../types";

const PRIORITY_ORDER: Record<Priority, number> = {
  P0: 0,
  P1: 1,
  P2: 2,
  P3: 3,
  P4: 4,
};

export type MemoGroup =
  | { kind: "project"; project: Project; memos: Memo[] }
  | { kind: "etc"; memos: Memo[] };

/**
 * Assign a 1-based badge number to each memo by ascending sort_order.
 * The result stays stable across re-renders so the #N a user sees in the
 * UI is the same N they type into the AI tools.
 */
export function globalSequence(memos: Memo[]): Map<number, number> {
  return new Map(
    [...memos]
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((m, i) => [m.id, i + 1])
  );
}

/**
 * Bucket memos under their parent project, ordering groups by project
 * priority (P0 first) with `기타` always last. Empty groups are dropped
 * so the board never renders a lonely header.
 */
export function groupMemosByProject(
  memos: Memo[],
  projects: Project[]
): MemoGroup[] {
  const byProject = new Map<number, Memo[]>();
  const etc: Memo[] = [];

  const sortedMemos = [...memos].sort((a, b) => a.sort_order - b.sort_order);
  for (const m of sortedMemos) {
    if (m.project_id === null || m.project_id === undefined) {
      etc.push(m);
    } else {
      const list = byProject.get(m.project_id) ?? [];
      list.push(m);
      byProject.set(m.project_id, list);
    }
  }

  const orderedProjects = [...projects].sort((a, b) => {
    const pa = PRIORITY_ORDER[a.priority as Priority] ?? 99;
    const pb = PRIORITY_ORDER[b.priority as Priority] ?? 99;
    if (pa !== pb) return pa - pb;
    return a.sort_order - b.sort_order;
  });

  const groups: MemoGroup[] = [];
  for (const p of orderedProjects) {
    const list = byProject.get(p.id);
    if (list && list.length > 0) {
      groups.push({ kind: "project", project: p, memos: list });
    }
  }
  if (etc.length > 0) groups.push({ kind: "etc", memos: etc });
  return groups;
}
