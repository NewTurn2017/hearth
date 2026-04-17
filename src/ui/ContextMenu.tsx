import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";
import type { LucideIcon } from "lucide-react";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: LucideIcon;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
  /** Optional inline content rendered inside the menu row (e.g. a color-swatch
   *  row for "색상 변경"). When set, `label` is still used as the row header
   *  and `onSelect` is ignored. */
  inline?: ReactNode;
  /** Divider row — renders a thin separator and ignores all other fields. */
  separator?: boolean;
}

const PANEL_WIDTH = 208;
const MARGIN = 8;

export function ContextMenu({
  open,
  x,
  y,
  items,
  onClose,
}: {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const [clampedX, setClampedX] = useState(x);
  const [clampedY, setClampedY] = useState(y);

  // Clamp before paint so the menu does not flash at the requested coords and
  // then jump. `useLayoutEffect` is required — `useEffect` runs after paint.
  useLayoutEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    const h = panel?.offsetHeight ?? 240;
    const nextX = Math.min(x, window.innerWidth - PANEL_WIDTH - MARGIN);
    const nextY = Math.min(y, window.innerHeight - h - MARGIN);
    setClampedX(Math.max(MARGIN, nextX));
    setClampedY(Math.max(MARGIN, nextY));
  }, [open, x, y]);

  useEffect(() => {
    if (!open) return;
    const onDocPointer = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // `mousedown` beats `click` — important because the right-click that
    // opens the menu fires `contextmenu` first, then `mousedown` on release.
    document.addEventListener("mousedown", onDocPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      ref={panelRef}
      role="menu"
      style={{
        position: "fixed",
        left: clampedX,
        top: clampedY,
        minWidth: PANEL_WIDTH,
      }}
      className={cn(
        "z-[200] p-1 rounded-[var(--radius-md)]",
        "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
        "shadow-[var(--shadow-e3)] text-[13px]"
      )}
    >
      {items.map((it) => {
        if (it.separator) {
          return (
            <div
              key={it.id}
              role="separator"
              className="my-1 h-px bg-[var(--color-border)]"
            />
          );
        }
        if (it.inline) {
          return (
            <div key={it.id} className="px-2 py-1">
              <div className="text-[11px] text-[var(--color-text-dim)] mb-1">
                {it.label}
              </div>
              {it.inline}
            </div>
          );
        }
        return (
          <button
            key={it.id}
            role="menuitem"
            type="button"
            disabled={it.disabled}
            onClick={() => {
              if (it.disabled) return;
              // Short delay so the click lands before the menu tears down —
              // without it, the outside-click handler wins the race and the
              // `onSelect` callback never fires on fast trackpad clicks.
              setTimeout(() => {
                it.onSelect();
                onClose();
              }, 0);
            }}
            className={cn(
              "w-full flex items-center gap-2 px-2 h-8 rounded text-left",
              "transition-colors duration-[120ms]",
              it.disabled
                ? "opacity-50 cursor-not-allowed"
                : it.danger
                  ? "text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                  : "text-[var(--color-text)] hover:bg-[var(--color-surface-3)]"
            )}
          >
            {it.icon && <Icon icon={it.icon} size={14} />}
            <span>{it.label}</span>
          </button>
        );
      })}
    </div>,
    document.body
  );
}
