// src/ui/Dialog.tsx
import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { cn } from "../lib/cn";

export function Dialog({
  open,
  onClose,
  children,
  className,
  labelledBy,
}: {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  className?: string;
  labelledBy?: string;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const prevFocused = useRef<Element | null>(null);
  // Keep onClose in a ref so the focus-trap effect does not re-run when the
  // parent passes a fresh callback identity on every render (which would
  // re-focus the panel and yank focus from inputs mid-typing).
  const onCloseRef = useRef(onClose);
  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!open) return;
    prevFocused.current = document.activeElement;
    const panel = panelRef.current;
    // Let `autoFocus` on inner inputs win if they already claimed focus.
    if (panel && !panel.contains(document.activeElement)) {
      panel.focus();
    }

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onCloseRef.current();
      }
      if (e.key === "Tab" && panel) {
        const focusables = panel.querySelectorAll<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      (prevFocused.current as HTMLElement | null)?.focus?.();
    };
  }, [open]);

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[100] flex items-center justify-center p-6 bg-black/50"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        tabIndex={-1}
        className={cn(
          "w-full max-w-md rounded-[var(--radius-xl)]",
          "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
          "shadow-[var(--shadow-e3)] p-5 outline-none",
          className
        )}
      >
        {children}
      </div>
    </div>,
    document.body
  );
}
