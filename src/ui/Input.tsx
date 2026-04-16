// src/ui/Input.tsx
import { forwardRef, type InputHTMLAttributes } from "react";
import { cn } from "../lib/cn";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...rest }, ref) {
    return (
      <input
        ref={ref}
        className={cn(
          "h-9 w-full px-3 rounded-[var(--radius-md)] text-[13px]",
          "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
          "text-[var(--color-text)] placeholder:text-[var(--color-text-dim)]",
          "focus:outline-none focus:border-[var(--color-brand-hi)]",
          "transition-colors duration-[120ms]",
          className
        )}
        {...rest}
      />
    );
  }
);
