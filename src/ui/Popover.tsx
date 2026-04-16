// src/ui/Popover.tsx
import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "../lib/cn";

export function Popover({
  trigger,
  children,
  className,
}: {
  trigger: (props: { onClick: () => void; "aria-expanded": boolean }) => ReactNode;
  children: ReactNode;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDocClick);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocClick);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative inline-block">
      {trigger({ onClick: () => setOpen((v) => !v), "aria-expanded": open })}
      {open && (
        <div
          role="dialog"
          className={cn(
            "absolute right-0 mt-1 z-50 min-w-[180px] p-1",
            "rounded-[var(--radius-md)] bg-[var(--color-surface-2)]",
            "border border-[var(--color-border)] shadow-[var(--shadow-e2)]",
            className
          )}
        >
          {children}
        </div>
      )}
    </div>
  );
}
