// Picker dialog for "프로젝트 이동" in the memo context menu.
//
// The spec chose a picker dialog over a nested submenu to sidestep hover-
// timing bugs. Selecting "기타 (연결 해제)" resolves to null so the caller
// can explicitly detach via `{ project_id: null }`.

import { useMemo } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { PRIORITIES } from "../types";
import type { Priority, Project } from "../types";
import { cn } from "../lib/cn";

export function MemoProjectPickerDialog({
  open,
  projects,
  currentProjectId,
  onClose,
  onPick,
}: {
  open: boolean;
  projects: Project[];
  currentProjectId: number | null;
  onClose: () => void;
  /** `null` → detach. */
  onPick: (projectId: number | null) => void;
}) {
  const grouped = useMemo(() => {
    const map = new Map<Priority, Project[]>();
    for (const p of PRIORITIES) map.set(p, []);
    for (const p of projects) {
      if ((PRIORITIES as readonly string[]).includes(p.priority)) {
        map.get(p.priority as Priority)!.push(p);
      }
    }
    return map;
  }, [projects]);

  return (
    <Dialog open={open} onClose={onClose} labelledBy="memo-picker-title">
      <h2
        id="memo-picker-title"
        className="text-heading text-[var(--color-text-hi)] mb-3"
      >
        프로젝트 이동
      </h2>
      <div className="flex flex-col gap-1 max-h-[320px] overflow-y-auto">
        <PickerRow
          active={currentProjectId === null}
          onClick={() => onPick(null)}
          label="기타 (연결 해제)"
        />
        {PRIORITIES.map((pri) => {
          const items = grouped.get(pri) ?? [];
          if (items.length === 0) return null;
          return (
            <div key={pri} className="flex flex-col">
              <div className="text-[11px] text-[var(--color-text-dim)] px-2 mt-2 mb-1">
                {pri}
              </div>
              {items.map((p) => (
                <PickerRow
                  key={p.id}
                  active={currentProjectId === p.id}
                  onClick={() => onPick(p.id)}
                  label={p.name}
                />
              ))}
            </div>
          );
        })}
      </div>
      <div className="flex justify-end mt-4">
        <Button variant="secondary" onClick={onClose}>
          취소
        </Button>
      </div>
    </Dialog>
  );
}

function PickerRow({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "h-8 px-2 text-[13px] text-left rounded-[var(--radius-sm)]",
        "transition-colors duration-[120ms]",
        active
          ? "bg-[var(--color-brand-soft)] text-[var(--color-brand-hi)]"
          : "text-[var(--color-text)] hover:bg-[var(--color-surface-3)]"
      )}
    >
      {label}
    </button>
  );
}
