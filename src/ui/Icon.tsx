// src/ui/Icon.tsx
import type { LucideIcon } from "lucide-react";
import { cn } from "../lib/cn";

export type IconSize = 14 | 16 | 18;

export function Icon({
  icon: IconCmp,
  size = 16,
  className,
}: {
  icon: LucideIcon;
  size?: IconSize;
  className?: string;
}) {
  return (
    <IconCmp
      size={size}
      strokeWidth={1.75}
      className={cn("shrink-0", className)}
      aria-hidden
    />
  );
}
