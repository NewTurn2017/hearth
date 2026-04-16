// src/ui/Tooltip.tsx
import { useState, type ReactElement } from "react";
import { cn } from "../lib/cn";

export function Tooltip({
  label,
  children,
  side = "top",
}: {
  label: string;
  children: ReactElement;
  side?: "top" | "bottom";
}) {
  const [visible, setVisible] = useState(false);
  return (
    <span
      className="relative inline-flex"
      onMouseEnter={() => setVisible(true)}
      onMouseLeave={() => setVisible(false)}
      onFocus={() => setVisible(true)}
      onBlur={() => setVisible(false)}
    >
      {children}
      {visible && (
        <span
          role="tooltip"
          className={cn(
            "absolute left-1/2 -translate-x-1/2 z-50 whitespace-nowrap",
            "px-2 py-1 rounded-[var(--radius-sm)] text-[11px]",
            "bg-[var(--color-surface-3)] text-[var(--color-text)] border border-[var(--color-border)]",
            "shadow-[var(--shadow-e2)] pointer-events-none",
            side === "top" ? "bottom-full mb-1" : "top-full mt-1"
          )}
        >
          {label}
        </span>
      )}
    </span>
  );
}
