// src/ui/Popover.tsx
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { cn } from "../lib/cn";

type Side = "left" | "right";

export function Popover({
  trigger,
  children,
  className,
}: {
  trigger: (props: { onClick: () => void; "aria-expanded": boolean }) => ReactNode;
  /** Either static content or a render-prop that receives `close` — pickers
   *  use the callback to dismiss the popover after a selection. */
  children: ReactNode | ((api: { close: () => void }) => ReactNode);
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  // Default: popover grows rightward (anchored at trigger's left edge).
  // After open, we measure and flip if it would overflow the viewport.
  const [side, setSide] = useState<Side>("left");
  const rootRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const close = () => setOpen(false);

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

  // Reset to the default side every time the popover reopens so a previous
  // flip on a different target doesn't bias the next measurement.
  useLayoutEffect(() => {
    if (!open) {
      setSide("left");
      return;
    }
    const el = panelRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const viewportW = window.innerWidth;
    const margin = 8;
    if (side === "left" && rect.right > viewportW - margin) {
      // Popover overflows the right edge → anchor to trigger's right edge
      // instead so it grows leftward.
      setSide("right");
    } else if (side === "right" && rect.left < margin) {
      setSide("left");
    }
  }, [open, side]);

  return (
    <div ref={rootRef} className="relative inline-block">
      {trigger({ onClick: () => setOpen((v) => !v), "aria-expanded": open })}
      {open && (
        <div
          ref={panelRef}
          role="dialog"
          className={cn(
            "absolute mt-1 z-50 min-w-[180px] p-1",
            side === "left" ? "left-0" : "right-0",
            "rounded-[var(--radius-md)] bg-[var(--color-surface-2)]",
            "border border-[var(--color-border)] shadow-[var(--shadow-e2)]",
            className
          )}
        >
          {typeof children === "function" ? children({ close }) : children}
        </div>
      )}
    </div>
  );
}
