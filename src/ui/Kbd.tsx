// src/ui/Kbd.tsx
import type { ReactNode } from "react";
import { cn } from "../lib/cn";

export function Kbd({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-flex items-center h-5 min-w-[20px] px-1.5 rounded border",
        "bg-[var(--color-surface-1)] border-[var(--color-border-strong)]",
        "text-[11px] text-[var(--color-text-muted)] font-mono",
        className
      )}
    >
      {children}
    </kbd>
  );
}
