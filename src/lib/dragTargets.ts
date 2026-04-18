import { PRIORITIES, type Priority, type Project } from "../types";

export type DragTarget = { priority: Priority; overId: number | null };

const EMPTY_ZONE = /^priority-(P[0-4])-empty$/;
const GROUP_ZONE = /^priority-(P[0-4])-group$/;

/**
 * Map a dnd-kit `over.id` back to a concrete priority + (optional) card id.
 * - Number id → the project card at that id, if it belongs to a known group.
 * - `priority-P{n}-empty` string → the empty drop zone for that priority.
 * - `priority-P{n}-group` string → the whole-group drop zone (whitespace
 *   between / around cards), used so users don't have to land exactly on a
 *   card to change a project's priority. Insertion point becomes "append".
 * - Anything else → null (drop is ignored).
 */
export function deriveTarget(
  overId: string | number,
  projects: Project[]
): DragTarget | null {
  if (typeof overId === "string") {
    const empty = EMPTY_ZONE.exec(overId);
    if (empty) {
      const priority = empty[1] as Priority;
      if (!(PRIORITIES as readonly string[]).includes(priority)) return null;
      return { priority, overId: null };
    }
    const group = GROUP_ZONE.exec(overId);
    if (group) {
      const priority = group[1] as Priority;
      if (!(PRIORITIES as readonly string[]).includes(priority)) return null;
      return { priority, overId: null };
    }
    return null;
  }
  const card = projects.find((p) => p.id === overId);
  if (!card) return null;
  if (!(PRIORITIES as readonly string[]).includes(card.priority)) return null;
  return { priority: card.priority as Priority, overId };
}
