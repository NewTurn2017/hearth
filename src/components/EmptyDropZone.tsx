import { useDroppable } from "@dnd-kit/core";

export function EmptyDropZone({ id, label }: { id: string; label: string }) {
  const { setNodeRef, isOver } = useDroppable({ id });
  return (
    <div
      ref={setNodeRef}
      className={
        "w-full min-h-[88px] flex items-center justify-center rounded-[var(--radius-md)] border-2 border-dashed px-3 py-6 text-[12px] transition-colors " +
        (isOver
          ? "border-[var(--color-brand)] bg-[var(--color-surface-3)] text-[var(--color-text)]"
          : "border-[var(--color-border)] text-[var(--color-text-dim)]")
      }
    >
      {label}
    </div>
  );
}
