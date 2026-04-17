import { useDroppable } from "@dnd-kit/core";

export function EmptyDropZone({ id, label }: { id: string; label: string }) {
  const { setNodeRef, isOver } = useDroppable({ id });
  return (
    <div
      ref={setNodeRef}
      className={
        "rounded-[var(--radius-md)] border-2 border-dashed px-3 py-4 text-[12px] text-center transition-colors " +
        (isOver
          ? "border-[var(--color-brand)] bg-[var(--color-surface-3)] text-[var(--color-text)]"
          : "border-[var(--color-border)] text-[var(--color-text-dim)]")
      }
    >
      {label}
    </div>
  );
}
