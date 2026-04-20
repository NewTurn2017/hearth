import type { Memo, Project } from "../types";
import type { MemoGroup } from "../lib/memoSequence";
import { MemoRow } from "./MemoRow";

export function MemoMatrix({
  groups,
  projects,
  sequence,
  highlightedId,
  onUpdate,
  onDelete,
}: {
  groups: MemoGroup[];
  projects: Project[];
  sequence: Map<number, number>;
  highlightedId: number | null;
  onUpdate: (id: number, fields: Record<string, unknown>) => void;
  onDelete: (id: number) => void;
}) {
  return (
    <div className="grid gap-4 grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4 auto-rows-fr flex-1 min-h-0">
      {groups.map((g) => {
        const key = g.kind === "project" ? `proj-${g.project.id}` : "etc";
        const title =
          g.kind === "project" ? g.project.name : "기타 · 프로젝트 미연결";
        const priority = g.kind === "project" ? g.project.priority : null;
        return (
          <Tile
            key={key}
            title={title}
            priority={priority}
            memos={g.memos}
            projects={projects}
            sequence={sequence}
            highlightedId={highlightedId}
            onUpdate={onUpdate}
            onDelete={onDelete}
          />
        );
      })}
    </div>
  );
}

function Tile({
  title,
  priority,
  memos,
  projects,
  sequence,
  highlightedId,
  onUpdate,
  onDelete,
}: {
  title: string;
  priority: string | null;
  memos: Memo[];
  projects: Project[];
  sequence: Map<number, number>;
  highlightedId: number | null;
  onUpdate: (id: number, fields: Record<string, unknown>) => void;
  onDelete: (id: number) => void;
}) {
  return (
    <section className="flex flex-col min-h-0 rounded-lg border border-[var(--color-border)] bg-[var(--color-surface-1)] overflow-hidden">
      <header className="flex items-center gap-2 px-3 py-2 border-b border-[var(--color-border)] bg-[var(--color-surface-2)]">
        <span className="font-semibold text-[13px] text-[var(--color-text-hi)] truncate">
          {title}
        </span>
        {priority && (
          <span className="text-[11px] text-[var(--color-text-muted)]">
            · {priority}
          </span>
        )}
        <span className="ml-auto text-[11px] text-[var(--color-text-dim)]">
          {memos.length}
        </span>
      </header>
      <div className="flex-1 min-h-0 overflow-y-auto p-1.5">
        {memos.map((m) => (
          <MemoRow
            key={m.id}
            memo={m}
            projects={projects}
            onUpdate={onUpdate}
            onDelete={onDelete}
            sequenceNumber={sequence.get(m.id) ?? 0}
            highlighted={m.id === highlightedId}
          />
        ))}
      </div>
    </section>
  );
}
