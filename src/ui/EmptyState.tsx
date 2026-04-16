// src/ui/EmptyState.tsx
import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
}: {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center",
        "px-6 py-12 gap-2 text-[var(--color-text-muted)]",
        className
      )}
    >
      {icon && <Icon icon={icon} size={18} className="text-[var(--color-text-dim)] mb-1" />}
      <p className="text-[13px] text-[var(--color-text)]">{title}</p>
      {description && <p className="text-[12px]">{description}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}
