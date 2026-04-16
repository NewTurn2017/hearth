// src/ui/Badge.tsx
import type { ReactNode } from "react";
import { cn } from "../lib/cn";

export function Badge({
  children,
  tone,
  className,
}: {
  children: ReactNode;
  /** CSS color string (hex/var) used for text + semi-transparent bg. */
  tone?: string;
  className?: string;
}) {
  const style = tone
    ? { color: tone, backgroundColor: `${tone}22` }
    : undefined;
  return (
    <span
      style={style}
      className={cn(
        "inline-flex items-center h-5 px-2 rounded-full text-[11px] font-medium",
        !tone && "bg-[var(--color-surface-3)] text-[var(--color-text-muted)]",
        className
      )}
    >
      {children}
    </span>
  );
}
