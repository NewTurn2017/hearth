// Stub — Task 20 replaces this with the full picker implementation.
import type { Project } from "../types";

export function MemoProjectPickerDialog({
  open,
  onClose,
}: {
  open: boolean;
  projects: Project[];
  currentProjectId: number | null;
  onClose: () => void;
  onPick: (projectId: number | null) => void;
}) {
  if (!open) return null;
  // Placeholder UI — the real dialog arrives in Task 20.
  return (
    <div aria-hidden className="sr-only" onClick={onClose}>
      picker stub
    </div>
  );
}
