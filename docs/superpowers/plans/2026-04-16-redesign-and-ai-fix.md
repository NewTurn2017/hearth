# Hearth Redesign & AI Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild UI on a design-token system, replace all emojis with `lucide-react` icons, swap the right-side AI drawer for a ⌘K command palette, and rewrite the AI backend (Rust + TS) so the mlx_lm.server lifecycle and tool-call parsing are reliable.

**Architecture:** Three layers land in order — (1) `src/ui/` primitives on top of Tailwind 4 `@theme` tokens, (2) `src/command/` ⌘K palette using `cmdk` with local quick-action mode and AI mode, (3) rewritten `src-tauri/src/cmd_ai.rs` state machine with `response_format: json_schema` + targeted PID kill on window destruction. Domain components (Sidebar, TopBar, cards, modals) are refactored to use the new primitives. All mutations flow through a confirm dialog + Toast/Undo pipe.

**Tech Stack:** Tauri 2, React 19, Tailwind 4, `lucide-react`, `clsx`, `tailwind-merge`, `cmdk`, `mlx_lm.server` (external), rusqlite, reqwest, tokio.

**Scope:** Covers the full redesign and AI rewrite as one integrated effort — splitting would leave UI primitives without consumers or palette without AI. Executed in phases with commit-per-task granularity (overrides spec's "1 commit per phase" for better revert surface).

---

## File Structure

### New files (create)

| Path | Responsibility |
|------|----------------|
| `src/lib/cn.ts` | `clsx` + `tailwind-merge` helper |
| `src/lib/shortcuts.ts` | Global keyboard listener for ⌘K |
| `src/ui/Icon.tsx` | `lucide-react` wrapper — normalizes size/stroke |
| `src/ui/Button.tsx` | variant + size + leftIcon/rightIcon |
| `src/ui/Input.tsx` | Text / search input with consistent focus ring |
| `src/ui/Dialog.tsx` | Modal with backdrop, ESC, focus trap, portal |
| `src/ui/Popover.tsx` | Floating panel anchored to trigger |
| `src/ui/Tooltip.tsx` | Hover tooltip with delay |
| `src/ui/Badge.tsx` | Priority / Category pill |
| `src/ui/Kbd.tsx` | Keyboard key hint |
| `src/ui/Toast.tsx` | Toast provider + `useToast()` with Undo |
| `src/ui/EmptyState.tsx` | Shared empty-state view |
| `src/command/CommandPalette.tsx` | Root (⌘K listener, backdrop, state) |
| `src/command/CommandInput.tsx` | Input row with mode indicator + spinner |
| `src/command/CommandResults.tsx` | Local actions list + AI reply + AI actions |
| `src/command/CommandEmpty.tsx` | "매칭 없음" state |
| `src/command/useCommandState.ts` | Mode / query / results state machine |
| `src/command/dispatch.ts` | Local action registry + executor (mutation → confirm) |
| `src/command/types.ts` | `CommandItem`, `AiAction`, `ActionCommand` enums |
| `src/components/TopBar.tsx` | Replaces `TabBar.tsx` (renamed + redesigned) |

### Files to modify

| Path | Reason |
|------|--------|
| `package.json` | Add `lucide-react`, `clsx`, `tailwind-merge`, `cmdk` |
| `src/App.css` | Replace CSS vars with Tailwind 4 `@theme` token bindings |
| `src/App.tsx` | Wrap in `<ToastProvider>` and `<CommandPalette>` |
| `src/api.ts` | Update AI command signatures; add `AiResponse` / `AiAction` |
| `src/types.ts` | Add `AiAction`, `AiResponse`, `AiServerState` types |
| `src/hooks/useAi.ts` | Rewrite — lazy start, structured output, history |
| `src/components/Layout.tsx` | Remove AiPanel + floating button, mount CommandPalette |
| `src/components/Sidebar.tsx` | Token-based styles, lucide icons for filter reset |
| `src/components/ProjectCard.tsx` | lucide icons replacing `▶📁✕≡`, Badge primitive |
| `src/components/ProjectList.tsx` | Uses `EmptyState`, new button styles |
| `src/components/MemoCard.tsx` | `⠿` → `GripVertical` icon, token colors |
| `src/components/MemoBoard.tsx` | Token-based empty state + add button |
| `src/components/CalendarView.tsx` | react-big-calendar theme CSS with new tokens |
| `src/components/ScheduleModal.tsx` | Built on `Dialog` primitive |
| `src-tauri/src/cmd_ai.rs` | Rewrite — state machine, targeted PID kill, json_schema response_format |
| `src-tauri/src/lib.rs` | `AiState` shape change, window-destroyed kill via stored PID |

### Files to delete

- `src/components/AiPanel.tsx`
- `src/components/ChatMessage.tsx`
- `src/components/TabBar.tsx` (replaced by `TopBar.tsx`)

---

## Conventions Used in This Plan

- **Smoke verify** means: `npm run tauri dev` (or just `tsc --noEmit` if no visual surface yet), then manually confirm the listed observation. The frontend has no automated test infra (spec §5.8).
- **Rust tests** use `cargo test --manifest-path src-tauri/Cargo.toml`.
- **Commit** step always shows `git add <paths>` with the exact files the task touched.
- Working directory throughout is the project root `/Users/genie/dev/tools/hearth`.

---

## Phase 0 — Foundation

### Task 1: Install new dependencies

**Files:**
- Modify: `package.json`, `package-lock.json`

- [ ] **Step 1: Install deps**

Run: `npm install lucide-react@^0.475 clsx@^2.1 tailwind-merge@^3.0 cmdk@^1.0`

Expected: installs succeed, `package.json` gets the four new entries.

- [ ] **Step 2: Verify it still builds**

Run: `npm run build`
Expected: tsc passes, vite bundles. (Bundle size warning is fine.)

- [ ] **Step 3: Commit**

```bash
git add package.json package-lock.json
git commit -m "chore: add lucide-react, clsx, tailwind-merge, cmdk"
```

---

### Task 2: Add `cn()` utility

**Files:**
- Create: `src/lib/cn.ts`

- [ ] **Step 1: Write cn helper**

```ts
// src/lib/cn.ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 2: Verify type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/cn.ts
git commit -m "feat(lib): add cn() class-name helper"
```

---

### Task 3: Rewrite `App.css` with warm-paper dark tokens

**Files:**
- Modify: `src/App.css`

- [ ] **Step 1: Replace entire file contents**

Replace `src/App.css` with:

```css
@import "tailwindcss";

@theme {
  /* Surface (warm paper dark) */
  --color-surface-0: #141312;
  --color-surface-1: #1a1917;
  --color-surface-2: #221f19;
  --color-surface-3: #2a2721;
  --color-border: #2e2a23;
  --color-border-strong: #3a362e;

  /* Text */
  --color-text-hi: #f4efcf;
  --color-text: #ebeadf;
  --color-text-muted: #a7a496;
  --color-text-dim: #7a7668;

  /* Brand — Amber */
  --color-brand: #d97706;
  --color-brand-hi: #fbbf24;
  --color-brand-soft: rgba(217, 119, 6, 0.18);

  /* Priority */
  --color-p0: #ef4444;
  --color-p1: #f97316;
  --color-p2: #eab308;
  --color-p3: #3b82f6;
  --color-p4: #6b7280;

  /* Semantic */
  --color-success: #22c55e;
  --color-danger: #ef4444;

  /* Category */
  --color-cat-active: #22c55e;
  --color-cat-side: #f97316;
  --color-cat-lab: #a855f7;
  --color-cat-tools: #6b7280;
  --color-cat-lecture: #3b82f6;

  /* Radius */
  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 10px;
  --radius-xl: 14px;

  /* Shadow */
  --shadow-e1: 0 1px 2px rgba(0, 0, 0, 0.3);
  --shadow-e2: 0 4px 12px rgba(0, 0, 0, 0.35);
  --shadow-e3: 0 20px 40px rgba(0, 0, 0, 0.5);

  /* Motion */
  --ease-out-smooth: cubic-bezier(0.2, 0.8, 0.2, 1);

  /* Typography */
  --font-sans: "SF Pro Text", "Inter", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", ui-monospace, Menlo, monospace;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html,
body,
#root {
  height: 100vh;
}

body {
  background-color: var(--color-surface-0);
  color: var(--color-text);
  font-family: var(--font-sans);
  font-size: 13px;
  line-height: 1.45;
  overflow: hidden;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}

/* Typography scale utilities */
.text-display { font-size: 22px; line-height: 1.2; font-weight: 600; letter-spacing: -0.015em; }
.text-heading { font-size: 15px; line-height: 1.3; font-weight: 600; letter-spacing: -0.005em; }
.text-body    { font-size: 13px; line-height: 1.45; font-weight: 400; }
.text-small   { font-size: 12px; line-height: 1.4;  font-weight: 400; }
.text-label   { font-size: 10px; line-height: 1.4;  font-weight: 600; letter-spacing: 0.06em; text-transform: uppercase; }
.text-mono    { font-family: var(--font-mono); font-size: 12px; line-height: 1.4; }

/* Scrollbar */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--color-border-strong); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: var(--color-text-dim); }

/* Focus ring default (matches ui/ primitives) */
:focus-visible {
  outline: 2px solid var(--color-brand-hi);
  outline-offset: 2px;
}

/* Memo card (post-it shadow retained) */
.memo-card { box-shadow: 0 6px 16px rgba(0, 0, 0, 0.45); }
```

- [ ] **Step 2: Smoke verify app still renders**

Run: `npm run tauri dev` in background, then confirm visually that the app window opens with warm-dark background (will look raw — components not yet refactored, expected).

Kill dev server once confirmed.

- [ ] **Step 3: Commit**

```bash
git add src/App.css
git commit -m "feat(design): warm-paper dark tokens via Tailwind 4 @theme"
```

---

## Phase 1 — UI Primitives

### Task 4: `Icon` primitive

**Files:**
- Create: `src/ui/Icon.tsx`

- [ ] **Step 1: Write Icon wrapper**

```tsx
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
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/Icon.tsx
git commit -m "feat(ui): add Icon primitive wrapping lucide-react"
```

---

### Task 5: `Button` primitive

**Files:**
- Create: `src/ui/Button.tsx`

- [ ] **Step 1: Write Button**

```tsx
// src/ui/Button.tsx
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  leftIcon?: LucideIcon;
  rightIcon?: LucideIcon;
  children?: ReactNode;
}

const VARIANT: Record<Variant, string> = {
  primary:
    "bg-[var(--color-brand)] text-white hover:bg-[var(--color-brand-hi)] hover:text-black",
  secondary:
    "bg-[var(--color-surface-2)] text-[var(--color-text)] hover:bg-[var(--color-surface-3)] border border-[var(--color-border)]",
  ghost:
    "bg-transparent text-[var(--color-text-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]",
  danger:
    "bg-[var(--color-danger)] text-white hover:brightness-110",
};

const SIZE: Record<Size, string> = {
  sm: "h-7 px-2 text-[12px] gap-1.5",
  md: "h-9 px-3 text-[13px] gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", leftIcon, rightIcon, className, children, ...rest },
  ref
) {
  return (
    <button
      ref={ref}
      className={cn(
        "inline-flex items-center justify-center rounded-[var(--radius-md)]",
        "font-medium transition-colors duration-[120ms]",
        "focus-visible:ring-2 focus-visible:ring-[var(--color-brand-hi)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-surface-0)]",
        "disabled:opacity-50 disabled:cursor-not-allowed",
        VARIANT[variant],
        SIZE[size],
        className
      )}
      {...rest}
    >
      {leftIcon && <Icon icon={leftIcon} size={size === "sm" ? 14 : 16} />}
      {children}
      {rightIcon && <Icon icon={rightIcon} size={size === "sm" ? 14 : 16} />}
    </button>
  );
});
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/ui/Button.tsx
git commit -m "feat(ui): add Button primitive with variant/size/icon props"
```

---

### Task 6: `Input`, `Badge`, `Kbd` primitives

**Files:**
- Create: `src/ui/Input.tsx`, `src/ui/Badge.tsx`, `src/ui/Kbd.tsx`

- [ ] **Step 1: Write Input**

```tsx
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
```

- [ ] **Step 2: Write Badge**

```tsx
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
```

- [ ] **Step 3: Write Kbd**

```tsx
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
```

- [ ] **Step 4: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/ui/Input.tsx src/ui/Badge.tsx src/ui/Kbd.tsx
git commit -m "feat(ui): add Input, Badge, Kbd primitives"
```

---

### Task 7: `Dialog`, `Popover`, `Tooltip` primitives

**Files:**
- Create: `src/ui/Dialog.tsx`, `src/ui/Popover.tsx`, `src/ui/Tooltip.tsx`

- [ ] **Step 1: Write Dialog (portal + focus trap + ESC + backdrop)**

```tsx
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

  useEffect(() => {
    if (!open) return;
    prevFocused.current = document.activeElement;
    const panel = panelRef.current;
    panel?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
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
  }, [open, onClose]);

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
```

- [ ] **Step 2: Write Popover (anchored to trigger via absolute positioning)**

```tsx
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
```

- [ ] **Step 3: Write Tooltip**

```tsx
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
```

- [ ] **Step 4: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/ui/Dialog.tsx src/ui/Popover.tsx src/ui/Tooltip.tsx
git commit -m "feat(ui): add Dialog, Popover, Tooltip primitives"
```

---

### Task 8: `Toast` provider + `EmptyState`

**Files:**
- Create: `src/ui/Toast.tsx`, `src/ui/EmptyState.tsx`

- [ ] **Step 1: Write Toast provider with Undo queue**

```tsx
// src/ui/Toast.tsx
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { CheckCircle2, AlertCircle, RotateCcw, X } from "lucide-react";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";

type Kind = "success" | "error";

interface ToastItem {
  id: number;
  kind: Kind;
  message: string;
  undo?: () => void | Promise<void>;
}

interface ToastApi {
  success: (message: string, opts?: { undo?: () => void | Promise<void> }) => void;
  error: (message: string) => void;
}

const ToastCtx = createContext<ToastApi | null>(null);

const TTL_MS = 5000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const idRef = useRef(0);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const remove = useCallback((id: number) => {
    setItems((prev) => prev.filter((t) => t.id !== id));
    const timer = timers.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const push = useCallback(
    (kind: Kind, message: string, undo?: () => void | Promise<void>) => {
      const id = ++idRef.current;
      setItems((prev) => [...prev, { id, kind, message, undo }]);
      const timer = setTimeout(() => remove(id), TTL_MS);
      timers.current.set(id, timer);
    },
    [remove]
  );

  const api: ToastApi = {
    success: (message, opts) => push("success", message, opts?.undo),
    error: (message) => push("error", message),
  };

  useEffect(() => () => timers.current.forEach(clearTimeout), []);

  return (
    <ToastCtx.Provider value={api}>
      {children}
      <div className="fixed bottom-4 right-4 z-[200] flex flex-col gap-2 pointer-events-none">
        {items.map((t) => (
          <div
            key={t.id}
            className={cn(
              "pointer-events-auto flex items-center gap-2 min-w-[260px] max-w-[360px]",
              "px-3 py-2 rounded-[var(--radius-md)] border shadow-[var(--shadow-e2)]",
              t.kind === "success"
                ? "bg-[var(--color-surface-2)] border-[var(--color-border)]"
                : "bg-[var(--color-surface-2)] border-[var(--color-danger)]"
            )}
          >
            <Icon
              icon={t.kind === "success" ? CheckCircle2 : AlertCircle}
              size={16}
              className={t.kind === "success" ? "text-[var(--color-success)]" : "text-[var(--color-danger)]"}
            />
            <span className="flex-1 text-[12px] text-[var(--color-text)]">{t.message}</span>
            {t.undo && (
              <button
                onClick={async () => {
                  try {
                    await t.undo!();
                  } finally {
                    remove(t.id);
                  }
                }}
                className="inline-flex items-center gap-1 text-[11px] text-[var(--color-brand-hi)] hover:underline"
              >
                <Icon icon={RotateCcw} size={14} />
                Undo
              </button>
            )}
            <button
              onClick={() => remove(t.id)}
              className="text-[var(--color-text-dim)] hover:text-[var(--color-text)]"
              aria-label="dismiss"
            >
              <Icon icon={X} size={14} />
            </button>
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastCtx);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}
```

- [ ] **Step 2: Write EmptyState**

```tsx
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
```

- [ ] **Step 3: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ui/Toast.tsx src/ui/EmptyState.tsx
git commit -m "feat(ui): add Toast provider with Undo + EmptyState"
```

---

### Task 9: Wrap app in `ToastProvider`

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Read current `App.tsx`**

Read `src/App.tsx` to see exact current structure.

- [ ] **Step 2: Wrap root render in `<ToastProvider>`**

At the top of `src/App.tsx`, import:

```ts
import { ToastProvider } from "./ui/Toast";
```

Then wrap whatever the top-level JSX is. Example pattern:

```tsx
// Before:
// return <Layout>...</Layout>;
// After:
return (
  <ToastProvider>
    <Layout>...</Layout>
  </ToastProvider>
);
```

- [ ] **Step 3: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run: `npm run tauri dev` briefly — app should still render (no visual change yet). Kill when confirmed.

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "feat(app): mount ToastProvider at root"
```

---

## Phase 2 — Domain Component Refactoring

### Task 10: Add AI/Command types to `types.ts`

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Append AI-action types**

Append at the end of `src/types.ts`:

```ts
// --- AI / Command Palette ---

export type ActionCommand =
  | "create_project"
  | "update_project"
  | "delete_project"
  | "create_schedule"
  | "update_schedule"
  | "delete_schedule"
  | "create_memo"
  | "update_memo"
  | "delete_memo"
  | "set_filter"
  | "focus_project";

export type ActionType = "mutation" | "navigation" | "info";

export interface AiAction {
  type: ActionType;
  label: string;
  command?: ActionCommand;
  args?: Record<string, unknown>;
}

export interface AiResponse {
  reply: string;
  actions: AiAction[];
}

export type AiServerState =
  | { kind: "idle" }
  | { kind: "starting" }
  | { kind: "running"; port: number }
  | { kind: "failed"; error: string };
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat(types): add AiAction, AiResponse, AiServerState types"
```

---

### Task 11: `TopBar` (rename + redesign)

**Files:**
- Create: `src/components/TopBar.tsx`
- Delete: `src/components/TabBar.tsx` (in Step 3)
- Modify: `src/components/Layout.tsx` (swap import)

- [ ] **Step 1: Write `TopBar.tsx`**

```tsx
// src/components/TopBar.tsx
import {
  LayoutGrid,
  CalendarDays,
  StickyNote,
  Download,
  Save,
} from "lucide-react";
import type { Tab } from "../types";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { cn } from "../lib/cn";

const tabs: { key: Tab; label: string; icon: typeof LayoutGrid }[] = [
  { key: "projects", label: "프로젝트", icon: LayoutGrid },
  { key: "calendar", label: "캘린더", icon: CalendarDays },
  { key: "memos", label: "메모보드", icon: StickyNote },
];

export function TopBar({
  active,
  onChange,
  onImport,
  onBackup,
}: {
  active: Tab;
  onChange: (tab: Tab) => void;
  onImport: () => void;
  onBackup: () => void;
}) {
  return (
    <div className="flex items-center gap-1 px-3 h-11 bg-[var(--color-surface-1)] border-b border-[var(--color-border)]">
      <span className="text-heading text-[var(--color-text-hi)] mr-3 tracking-tight">
        Hearth
      </span>
      {tabs.map((t) => (
        <button
          key={t.key}
          onClick={() => onChange(t.key)}
          className={cn(
            "inline-flex items-center gap-1.5 h-8 px-3 rounded-[var(--radius-md)] text-[13px]",
            "transition-colors duration-[120ms]",
            active === t.key
              ? "bg-[var(--color-brand-soft)] text-[var(--color-brand-hi)]"
              : "text-[var(--color-text-muted)] hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]"
          )}
        >
          <Icon icon={t.icon} size={16} />
          {t.label}
        </button>
      ))}
      <div className="flex-1" />
      <Button variant="ghost" size="sm" leftIcon={Download} onClick={onImport}>
        가져오기
      </Button>
      <Button variant="ghost" size="sm" leftIcon={Save} onClick={onBackup}>
        백업
      </Button>
    </div>
  );
}
```

- [ ] **Step 2: Swap import in `Layout.tsx`**

Edit `src/components/Layout.tsx`:

```ts
// Before:
import { TabBar } from "./TabBar";
// After:
import { TopBar } from "./TopBar";
```

Rename JSX: `<TabBar ... />` → `<TopBar ... />`.

- [ ] **Step 3: Delete old `TabBar.tsx`**

Run: `git rm src/components/TabBar.tsx`

- [ ] **Step 4: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run `npm run tauri dev`, confirm top bar now uses lucide icons (LayoutGrid / CalendarDays / StickyNote / Download / Save), amber accent on active tab, no emojis. Kill when confirmed.

- [ ] **Step 5: Commit**

```bash
git add src/components/TopBar.tsx src/components/Layout.tsx
git commit -m "feat(ui): replace TabBar emojis with TopBar + lucide icons"
```

---

### Task 12: `Sidebar` refactor

**Files:**
- Modify: `src/components/Sidebar.tsx`

- [ ] **Step 1: Rewrite Sidebar**

Replace `src/components/Sidebar.tsx` with:

```tsx
import {
  PRIORITIES,
  CATEGORIES,
  PRIORITY_COLORS,
  PRIORITY_LABELS,
  CATEGORY_COLORS,
} from "../types";
import type { Priority, Category } from "../types";
import { cn } from "../lib/cn";

export function Sidebar({
  activePriorities,
  activeCategories,
  onTogglePriority,
  onToggleCategory,
}: {
  activePriorities: Set<Priority>;
  activeCategories: Set<Category>;
  onTogglePriority: (p: Priority) => void;
  onToggleCategory: (c: Category) => void;
}) {
  return (
    <aside className="w-52 shrink-0 bg-[var(--color-surface-1)] border-r border-[var(--color-border)] py-4 px-3 flex flex-col gap-6 overflow-y-auto">
      <FilterGroup label="우선순위">
        {PRIORITIES.map((p) => (
          <FilterItem
            key={p}
            active={activePriorities.has(p)}
            onClick={() => onTogglePriority(p)}
            dot={PRIORITY_COLORS[p]}
            text={`${p} — ${PRIORITY_LABELS[p]}`}
          />
        ))}
      </FilterGroup>

      <FilterGroup label="카테고리">
        {CATEGORIES.map((c) => (
          <FilterItem
            key={c}
            active={activeCategories.has(c)}
            onClick={() => onToggleCategory(c)}
            dot={CATEGORY_COLORS[c]}
            text={c}
          />
        ))}
      </FilterGroup>
    </aside>
  );
}

function FilterGroup({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <h3 className="text-label text-[var(--color-text-dim)] mb-2 px-1">{label}</h3>
      <div className="flex flex-col gap-0.5">{children}</div>
    </div>
  );
}

function FilterItem({
  active,
  onClick,
  dot,
  text,
}: {
  active: boolean;
  onClick: () => void;
  dot: string;
  text: string;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-2 h-7 px-2 rounded-[var(--radius-sm)] text-[12px] text-left",
        "transition-colors duration-[120ms]",
        active
          ? "bg-[var(--color-surface-2)] text-[var(--color-text)]"
          : "text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)]"
      )}
      aria-pressed={active}
    >
      <span
        className={cn("w-2 h-2 rounded-full shrink-0", !active && "opacity-40")}
        style={{ backgroundColor: dot }}
      />
      {text}
    </button>
  );
}
```

- [ ] **Step 2: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run `npm run tauri dev`, confirm sidebar uses new surface-1 bg, label caps styling, and no emojis. Kill when confirmed.

- [ ] **Step 3: Commit**

```bash
git add src/components/Sidebar.tsx
git commit -m "feat(ui): refactor Sidebar to token system"
```

---

### Task 13: `ProjectCard` — replace emoji buttons with lucide icons

**Files:**
- Modify: `src/components/ProjectCard.tsx`

- [ ] **Step 1: Rewrite ProjectCard**

Replace `src/components/ProjectCard.tsx` with:

```tsx
import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Play, FolderOpen, X } from "lucide-react";
import type { Project, Category } from "../types";
import { CATEGORY_COLORS } from "../types";
import { Badge } from "../ui/Badge";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
import { cn } from "../lib/cn";

export function ProjectCard({
  project,
  onUpdate,
  onDelete,
  onOpenGhostty,
  onOpenFinder,
}: {
  project: Project;
  onUpdate: (id: number, fields: Record<string, string>) => void;
  onDelete: (id: number) => void;
  onOpenGhostty: (path: string) => void;
  onOpenFinder: (path: string) => void;
}) {
  const [editing, setEditing] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: project.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const startEdit = (field: string, value: string) => {
    setEditing(field);
    setEditValue(value);
  };
  const commitEdit = () => {
    if (editing && editValue.trim()) {
      onUpdate(project.id, { [editing]: editValue.trim() });
    }
    setEditing(null);
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "flex items-center gap-3 px-3 h-11 rounded-[var(--radius-md)] group",
        "bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)]",
        "border border-transparent hover:border-[var(--color-border)]",
        "transition-colors duration-[120ms]"
      )}
    >
      <button
        {...attributes}
        {...listeners}
        className="cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)] shrink-0"
        aria-label="드래그하여 순서 변경"
      >
        <Icon icon={GripVertical} size={16} />
      </button>

      <div className="flex-1 min-w-0">
        {editing === "name" ? (
          <input
            autoFocus
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => e.key === "Enter" && commitEdit()}
            className="bg-transparent border-b border-[var(--color-brand-hi)] outline-none text-[13px] w-full text-[var(--color-text)]"
          />
        ) : (
          <span
            onClick={() => startEdit("name", project.name)}
            className="text-[13px] font-medium text-[var(--color-text)] cursor-text truncate block"
          >
            {project.name}
          </span>
        )}
        {editing === "evaluation" ? (
          <input
            autoFocus
            value={editValue}
            onChange={(e) => setEditValue(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => e.key === "Enter" && commitEdit()}
            className="bg-transparent border-b border-[var(--color-brand-hi)] outline-none text-[11px] w-full text-[var(--color-text-muted)] mt-0.5"
          />
        ) : (
          <span
            onClick={() => startEdit("evaluation", project.evaluation ?? "")}
            className="text-[11px] text-[var(--color-text-dim)] cursor-text truncate block mt-0.5"
          >
            {project.evaluation || "메모 없음"}
          </span>
        )}
      </div>

      {project.category && (
        <Badge tone={CATEGORY_COLORS[project.category as Category] ?? "#6b7280"}>
          {project.category}
        </Badge>
      )}

      <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
        {project.path && (
          <>
            <Tooltip label="Ghostty에서 열기">
              <button
                onClick={() => onOpenGhostty(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-2)]"
                aria-label="Ghostty에서 열기"
              >
                <Icon icon={Play} size={14} />
              </button>
            </Tooltip>
            <Tooltip label="Finder에서 열기">
              <button
                onClick={() => onOpenFinder(project.path!)}
                className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-2)]"
                aria-label="Finder에서 열기"
              >
                <Icon icon={FolderOpen} size={14} />
              </button>
            </Tooltip>
          </>
        )}
        <Tooltip label="삭제">
          <button
            onClick={() => onDelete(project.id)}
            className="w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)] text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]"
            aria-label="삭제"
          >
            <Icon icon={X} size={14} />
          </button>
        </Tooltip>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run: `npm run tauri dev`, hover a project row — icons show as GripVertical / Play / FolderOpen / X. Kill when confirmed.

- [ ] **Step 3: Commit**

```bash
git add src/components/ProjectCard.tsx
git commit -m "feat(ui): refactor ProjectCard to primitives + lucide icons"
```

---

### Task 14: `ProjectList` — empty state + new add button

**Files:**
- Modify: `src/components/ProjectList.tsx`

- [ ] **Step 1: Read current `ProjectList.tsx`** to find the existing add-project form / empty message locations.

Run: Read `src/components/ProjectList.tsx` (full file).

- [ ] **Step 2: Rewrite using primitives**

Edit `src/components/ProjectList.tsx`:
- Replace any emoji ("➕", "📋", etc.) with `Plus` / `FolderOpen` from `lucide-react` via `<Icon>`.
- Replace inline "프로젝트 없음" divs with `<EmptyState icon={FolderOpen} title="프로젝트가 없습니다" description="Excel로 가져오거나 ⌘K 로 추가하세요" />`.
- Replace add button with `<Button variant="primary" size="sm" leftIcon={Plus}>프로젝트 추가</Button>`.
- Replace background/border colors with `bg-[var(--color-surface-1)]` / `border-[var(--color-border)]`.

Imports to add at top:

```ts
import { Plus, FolderOpen } from "lucide-react";
import { Button } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
```

- [ ] **Step 3: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run `npm run tauri dev`; with DB cleared (or filter producing no matches), confirm EmptyState renders. Kill when confirmed.

- [ ] **Step 4: Commit**

```bash
git add src/components/ProjectList.tsx
git commit -m "feat(ui): refactor ProjectList with EmptyState + Button primitives"
```

---

### Task 15: `MemoCard` + `MemoBoard` refactor

**Files:**
- Modify: `src/components/MemoCard.tsx`, `src/components/MemoBoard.tsx`

- [ ] **Step 1: Read current files**

Read both to locate emoji usages (`⠿` drag handle, `✕` delete, `➕` add, etc.).

- [ ] **Step 2: Replace emojis/styles in `MemoCard.tsx`**

- `⠿` → `<Icon icon={GripVertical} size={14} />`
- `✕` → `<Icon icon={X} size={14} />`
- Wrap both in `<Tooltip>` for discoverability.
- Keep memo pastel bg colors (MEMO_COLORS) — this is intentional post-it visual.
- Replace the outer card div's `box-shadow` inline with the class `.memo-card` (already defined in App.css).

Imports:

```ts
import { GripVertical, X } from "lucide-react";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
```

- [ ] **Step 3: Replace emojis in `MemoBoard.tsx`**

- Add button: `<Button variant="primary" size="sm" leftIcon={Plus}>메모 추가</Button>`
- Empty state: `<EmptyState icon={StickyNote} title="메모가 없습니다" />`
- Remove any `📌` icons in headings.

- [ ] **Step 4: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

`npm run tauri dev` → memos tab → confirm GripVertical/X icons appear, pastels preserved. Kill when confirmed.

- [ ] **Step 5: Commit**

```bash
git add src/components/MemoCard.tsx src/components/MemoBoard.tsx
git commit -m "feat(ui): refactor MemoCard + MemoBoard to primitives"
```

---

### Task 16: `CalendarView` — theme overrides on react-big-calendar

**Files:**
- Modify: `src/components/CalendarView.tsx`, `src/App.css`

- [ ] **Step 1: Read current CalendarView** to identify existing overrides (emoji or custom bg).

- [ ] **Step 2: Append calendar theme block to `App.css`**

Append at the bottom of `src/App.css`:

```css
/* react-big-calendar — warm paper dark theme */
.rbc-calendar { background: var(--color-surface-1); color: var(--color-text); border-radius: var(--radius-lg); overflow: hidden; border: 1px solid var(--color-border); }
.rbc-toolbar { padding: 10px 12px; background: var(--color-surface-1); border-bottom: 1px solid var(--color-border); }
.rbc-toolbar button { background: transparent; color: var(--color-text-muted); border: 1px solid var(--color-border); padding: 4px 10px; border-radius: var(--radius-sm); }
.rbc-toolbar button:hover { background: var(--color-surface-2); color: var(--color-text); }
.rbc-toolbar button.rbc-active { background: var(--color-brand-soft); color: var(--color-brand-hi); border-color: var(--color-brand); }
.rbc-toolbar-label { color: var(--color-text-hi); font-weight: 600; font-size: 14px; }
.rbc-header { background: var(--color-surface-1); color: var(--color-text-muted); border-bottom: 1px solid var(--color-border); padding: 6px 4px; font-weight: 600; font-size: 11px; }
.rbc-month-view, .rbc-time-view { background: var(--color-surface-0); border-color: var(--color-border); }
.rbc-day-bg, .rbc-time-slot, .rbc-timeslot-group { background: var(--color-surface-0); border-color: var(--color-border); }
.rbc-off-range-bg { background: var(--color-surface-1); }
.rbc-today { background: var(--color-brand-soft) !important; }
.rbc-event { background: var(--color-brand); border: none; color: #fff; border-radius: var(--radius-sm); padding: 2px 6px; font-size: 11px; }
.rbc-event.rbc-selected { background: var(--color-brand-hi); color: #000; }
.rbc-show-more { color: var(--color-brand-hi); }
```

- [ ] **Step 3: Remove any emoji in `CalendarView.tsx`**

Replace any `📅`, `➕` etc. with `<Icon icon={CalendarDays} />` / `<Icon icon={Plus} />`.

- [ ] **Step 4: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

`npm run tauri dev` → calendar tab → confirm new theme (amber active tab, dark cells, today's cell amber-tinted). Kill when confirmed.

- [ ] **Step 5: Commit**

```bash
git add src/components/CalendarView.tsx src/App.css
git commit -m "feat(ui): theme react-big-calendar to warm paper dark"
```

---

### Task 17: `ScheduleModal` on `Dialog` primitive

**Files:**
- Modify: `src/components/ScheduleModal.tsx`

- [ ] **Step 1: Read current `ScheduleModal.tsx`**

Read the file to capture exact props + form fields.

- [ ] **Step 2: Rebuild on `Dialog`**

Rewrite `src/components/ScheduleModal.tsx`:
- Import: `import { Dialog } from "../ui/Dialog";` and `import { Button } from "../ui/Button";` and `import { Input } from "../ui/Input";`
- Replace the hand-rolled backdrop div with `<Dialog open={open} onClose={onClose} labelledBy="schedule-title">`.
- Heading: `<h2 id="schedule-title" className="text-heading text-[var(--color-text-hi)] mb-4">일정 {isEdit ? "수정" : "추가"}</h2>`.
- Replace form inputs with `<Input>`, footer buttons with `<Button variant="secondary">취소</Button>` and `<Button variant="primary">저장</Button>`.
- Remove any emoji (e.g. `📅`, `🕐`). Replace date/time labels with plain text.
- Delete button (edit mode): `<Button variant="danger" leftIcon={Trash2}>삭제</Button>`.

Imports:

```ts
import { Trash2 } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
```

- [ ] **Step 3: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

`npm run tauri dev` → create a schedule, confirm modal uses new Dialog with focus trap + ESC + backdrop-click close. Kill when confirmed.

- [ ] **Step 4: Commit**

```bash
git add src/components/ScheduleModal.tsx
git commit -m "feat(ui): rebuild ScheduleModal on Dialog primitive"
```

---

## Phase 3 — Command Palette

### Task 18: Global shortcut registration

**Files:**
- Create: `src/lib/shortcuts.ts`

- [ ] **Step 1: Write shortcut helper**

```ts
// src/lib/shortcuts.ts
import { useEffect } from "react";

export function isMac() {
  return /Mac|iPod|iPhone|iPad/.test(navigator.platform);
}

/** Listen for ⌘K / Ctrl+K globally. Calls `handler` with no arg. */
export function useCmdK(handler: () => void) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = isMac() ? e.metaKey : e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        handler();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/shortcuts.ts
git commit -m "feat(lib): add useCmdK global shortcut hook"
```

---

### Task 19: Command types + local action registry

**Files:**
- Create: `src/command/types.ts`, `src/command/dispatch.ts`

- [ ] **Step 1: Write `types.ts`**

```ts
// src/command/types.ts
import type { LucideIcon } from "lucide-react";
import type { AiAction } from "../types";

/** Locally-executable quick action (Mode 1). */
export interface LocalCommand {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  /** If true, `run` is treated as a mutation and a confirm dialog is shown. */
  mutation?: boolean;
  /** Human-readable confirm text (used when mutation=true). */
  confirmMessage?: string;
  /** Executed on ⏎. Returns an optional undo function for Toast "Undo". */
  run: () => Promise<(() => void | Promise<void>) | void>;
}

/** Unified result item displayed in the palette list. */
export type PaletteItem =
  | { kind: "local"; cmd: LocalCommand }
  | { kind: "ai"; action: AiAction };
```

- [ ] **Step 2: Write `dispatch.ts` — local action registry**

```ts
// src/command/dispatch.ts
import {
  FolderPlus,
  CalendarPlus,
  StickyNote,
  Save,
  Download,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { LocalCommand } from "./types";

export interface DispatchDeps {
  /** Callback to open the "create project" inline editor, etc. */
  openNewProject: () => void;
  openNewSchedule: () => void;
  openNewMemo: () => void;
}

export function buildLocalCommands(deps: DispatchDeps): LocalCommand[] {
  return [
    {
      id: "new-project",
      label: "새 프로젝트",
      hint: "프로젝트 추가",
      icon: FolderPlus,
      run: async () => {
        deps.openNewProject();
      },
    },
    {
      id: "new-schedule",
      label: "새 일정",
      hint: "일정 추가",
      icon: CalendarPlus,
      run: async () => {
        deps.openNewSchedule();
      },
    },
    {
      id: "new-memo",
      label: "새 메모",
      hint: "메모 추가",
      icon: StickyNote,
      run: async () => {
        deps.openNewMemo();
      },
    },
    {
      id: "backup",
      label: "백업 생성",
      hint: "DB 스냅샷 저장",
      icon: Save,
      mutation: true,
      confirmMessage: "현재 DB를 백업하시겠습니까?",
      run: async () => {
        await api.backupDb();
      },
    },
    {
      id: "import-excel",
      label: "Excel 가져오기",
      hint: "기존 데이터 덮어쓰기 여부 확인",
      icon: Download,
      mutation: true,
      confirmMessage: "Excel 파일을 선택하고 가져오시겠습니까?",
      run: async () => {
        const file = await open({
          filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
        });
        if (!file) return;
        await api.importExcel(
          typeof file === "string" ? file : (file as { path: string }).path,
          true
        );
      },
    },
  ];
}
```

- [ ] **Step 3: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/command/types.ts src/command/dispatch.ts
git commit -m "feat(command): add PaletteItem types + local command registry"
```

---

### Task 20: Palette state hook

**Files:**
- Create: `src/command/useCommandState.ts`

- [ ] **Step 1: Write `useCommandState`**

```ts
// src/command/useCommandState.ts
import { useMemo, useState } from "react";
import type { LocalCommand } from "./types";

export type Mode = "local" | "ai";

/**
 * Decides which mode the palette is in based on the input string:
 *   ""                  → local (show all local commands)
 *   "/"-prefixed        → local
 *   "?"-prefixed        → ai (strip prefix for query)
 *   otherwise           → local if a fuzzy match exists, else ai
 */
export function deriveMode(
  query: string,
  localMatches: LocalCommand[]
): { mode: Mode; aiQuery: string } {
  const trimmed = query.trim();
  if (trimmed === "" || trimmed.startsWith("/")) {
    return { mode: "local", aiQuery: "" };
  }
  if (trimmed.startsWith("?")) {
    return { mode: "ai", aiQuery: trimmed.slice(1).trim() };
  }
  if (localMatches.length > 0) {
    return { mode: "local", aiQuery: "" };
  }
  return { mode: "ai", aiQuery: trimmed };
}

/** Fuzzy match: lowercased substring check on label + hint. */
export function filterLocal(query: string, commands: LocalCommand[]): LocalCommand[] {
  const q = query.replace(/^\//, "").trim().toLowerCase();
  if (!q) return commands;
  return commands.filter((c) =>
    (c.label + " " + (c.hint ?? "")).toLowerCase().includes(q)
  );
}

export function useCommandState(commands: LocalCommand[]) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const localMatches = useMemo(() => filterLocal(query, commands), [query, commands]);
  const { mode, aiQuery } = useMemo(
    () => deriveMode(query, localMatches),
    [query, localMatches]
  );

  const reset = () => {
    setQuery("");
  };

  return {
    open,
    setOpen,
    query,
    setQuery,
    localMatches,
    mode,
    aiQuery,
    reset,
  };
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/command/useCommandState.ts
git commit -m "feat(command): add palette state + mode derivation"
```

---

### Task 21: `CommandInput`, `CommandResults`, `CommandEmpty`

**Files:**
- Create: `src/command/CommandInput.tsx`, `src/command/CommandResults.tsx`, `src/command/CommandEmpty.tsx`

- [ ] **Step 1: Write `CommandInput.tsx`**

```tsx
// src/command/CommandInput.tsx
import { forwardRef, type ChangeEvent, type KeyboardEvent } from "react";
import { Search, Loader2 } from "lucide-react";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";

export const CommandInput = forwardRef<
  HTMLInputElement,
  {
    value: string;
    onChange: (v: string) => void;
    onKeyDown: (e: KeyboardEvent<HTMLInputElement>) => void;
    loading?: boolean;
    placeholder?: string;
  }
>(function CommandInput({ value, onChange, onKeyDown, loading, placeholder }, ref) {
  return (
    <div className="flex items-center gap-2 h-12 px-4 border-b border-[var(--color-border)]">
      <Icon
        icon={loading ? Loader2 : Search}
        size={18}
        className={
          loading
            ? "text-[var(--color-brand-hi)] animate-spin"
            : "text-[var(--color-text-dim)]"
        }
      />
      <input
        ref={ref}
        value={value}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder={placeholder ?? "명령을 입력하거나 ?로 AI에게 물어보세요"}
        className="flex-1 bg-transparent outline-none text-[14px] text-[var(--color-text-hi)] placeholder:text-[var(--color-text-dim)]"
        autoFocus
      />
      <Kbd>ESC</Kbd>
      <Kbd>⌘K</Kbd>
    </div>
  );
});
```

- [ ] **Step 2: Write `CommandEmpty.tsx`**

```tsx
// src/command/CommandEmpty.tsx
export function CommandEmpty({ text }: { text: string }) {
  return (
    <div className="px-4 py-8 text-center text-[12px] text-[var(--color-text-dim)]">
      {text}
    </div>
  );
}
```

- [ ] **Step 3: Write `CommandResults.tsx`**

```tsx
// src/command/CommandResults.tsx
import type { LucideIcon } from "lucide-react";
import { ArrowRight, Zap } from "lucide-react";
import type { AiAction } from "../types";
import type { LocalCommand } from "./types";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";
import { cn } from "../lib/cn";

export function CommandResults({
  items,
  activeIndex,
  onHover,
  onSelect,
  aiReply,
}: {
  items: ResultItem[];
  activeIndex: number;
  onHover: (i: number) => void;
  onSelect: (i: number) => void;
  aiReply?: string;
}) {
  return (
    <div className="max-h-[360px] overflow-y-auto py-1">
      {aiReply && (
        <div className="px-4 py-3 text-[13px] text-[var(--color-text)] whitespace-pre-wrap border-b border-[var(--color-border)]">
          {aiReply}
        </div>
      )}
      {items.map((item, i) => (
        <button
          key={item.id}
          onMouseEnter={() => onHover(i)}
          onClick={() => onSelect(i)}
          className={cn(
            "w-full flex items-center gap-3 h-10 px-4 text-left text-[13px]",
            "transition-colors duration-[80ms]",
            i === activeIndex
              ? "bg-[var(--color-surface-2)] text-[var(--color-text-hi)]"
              : "text-[var(--color-text)] hover:bg-[var(--color-surface-2)]"
          )}
        >
          <Icon icon={item.icon} size={16} className="text-[var(--color-text-muted)]" />
          <span className="flex-1 truncate">{item.label}</span>
          {item.hint && (
            <span className="text-[11px] text-[var(--color-text-dim)]">{item.hint}</span>
          )}
          {i === activeIndex && <Kbd>⏎</Kbd>}
        </button>
      ))}
    </div>
  );
}

export interface ResultItem {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
}

export function itemFromLocal(c: LocalCommand): ResultItem {
  return { id: `local:${c.id}`, label: c.label, hint: c.hint, icon: c.icon };
}

export function itemFromAi(a: AiAction, idx: number): ResultItem {
  return {
    id: `ai:${idx}`,
    label: a.label,
    hint: a.type === "mutation" ? "확인 필요" : a.type === "navigation" ? "이동" : "정보",
    icon: a.type === "mutation" ? Zap : ArrowRight,
  };
}
```

- [ ] **Step 4: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/command/CommandInput.tsx src/command/CommandResults.tsx src/command/CommandEmpty.tsx
git commit -m "feat(command): add palette Input / Results / Empty components"
```

---

### Task 22: `CommandPalette` root (Mode 1 only — local actions + confirm dialog)

**Files:**
- Create: `src/command/CommandPalette.tsx`

AI mode wiring deferred to Task 27. This task makes ⌘K open a working local-action palette so Phase 3 is independently usable.

- [ ] **Step 1: Write `CommandPalette.tsx`**

```tsx
// src/command/CommandPalette.tsx
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import { useCmdK } from "../lib/shortcuts";
import { useToast } from "../ui/Toast";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { cn } from "../lib/cn";
import { CommandInput } from "./CommandInput";
import { CommandResults, itemFromLocal, type ResultItem } from "./CommandResults";
import { CommandEmpty } from "./CommandEmpty";
import { useCommandState } from "./useCommandState";
import type { LocalCommand } from "./types";

export function CommandPalette({ commands }: { commands: LocalCommand[] }) {
  const state = useCommandState(commands);
  const toast = useToast();
  const inputRef = useRef<HTMLInputElement>(null);
  const [activeIndex, setActiveIndex] = useState(0);
  const [pendingConfirm, setPendingConfirm] = useState<LocalCommand | null>(null);

  useCmdK(() => {
    state.setOpen(true);
    setTimeout(() => inputRef.current?.focus(), 0);
  });

  const items: ResultItem[] = useMemo(
    () => state.localMatches.map(itemFromLocal),
    [state.localMatches]
  );

  useEffect(() => {
    setActiveIndex(0);
  }, [state.query]);

  const close = useCallback(() => {
    state.setOpen(false);
    state.reset();
    setPendingConfirm(null);
  }, [state]);

  const executeCommand = useCallback(
    async (cmd: LocalCommand) => {
      try {
        const undo = await cmd.run();
        toast.success(`${cmd.label} 완료`, {
          undo: typeof undo === "function" ? undo : undefined,
        });
        close();
      } catch (e) {
        toast.error(`${cmd.label} 실패: ${e}`);
      }
    },
    [toast, close]
  );

  const onSelect = useCallback(
    (i: number) => {
      const cmd = state.localMatches[i];
      if (!cmd) return;
      if (cmd.mutation) {
        setPendingConfirm(cmd);
      } else {
        executeCommand(cmd);
      }
    },
    [state.localMatches, executeCommand]
  );

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, items.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        onSelect(activeIndex);
      } else if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    },
    [items.length, activeIndex, onSelect, close]
  );

  if (!state.open) return null;

  return (
    <>
      {createPortal(
        <div
          className="fixed inset-0 z-[90] flex items-start justify-center pt-[15vh] bg-black/40"
          onMouseDown={(e) => {
            if (e.target === e.currentTarget) close();
          }}
        >
          <div
            className={cn(
              "w-full max-w-[620px] rounded-[var(--radius-xl)]",
              "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
              "shadow-[var(--shadow-e3)] overflow-hidden"
            )}
          >
            <CommandInput
              ref={inputRef}
              value={state.query}
              onChange={state.setQuery}
              onKeyDown={onKeyDown}
            />
            {items.length === 0 ? (
              <CommandEmpty text="매칭되는 명령이 없습니다. '?'로 AI에 물어보세요." />
            ) : (
              <CommandResults
                items={items}
                activeIndex={activeIndex}
                onHover={setActiveIndex}
                onSelect={onSelect}
              />
            )}
          </div>
        </div>,
        document.body
      )}

      <Dialog
        open={!!pendingConfirm}
        onClose={() => setPendingConfirm(null)}
        labelledBy="confirm-title"
      >
        {pendingConfirm && (
          <>
            <h2 id="confirm-title" className="text-heading text-[var(--color-text-hi)] mb-2">
              확인
            </h2>
            <p className="text-[13px] text-[var(--color-text)] mb-5">
              {pendingConfirm.confirmMessage ?? `${pendingConfirm.label}을(를) 실행합니다.`}
            </p>
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setPendingConfirm(null)}>
                취소
              </Button>
              <Button
                variant="primary"
                autoFocus
                onClick={() => {
                  const cmd = pendingConfirm;
                  setPendingConfirm(null);
                  executeCommand(cmd);
                }}
              >
                실행
              </Button>
            </div>
          </>
        )}
      </Dialog>
    </>
  );
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/command/CommandPalette.tsx
git commit -m "feat(command): add CommandPalette with local mode + confirm dialog"
```

---

### Task 23: Mount `CommandPalette` in `Layout`; remove AI floating button

**Files:**
- Modify: `src/components/Layout.tsx`

- [ ] **Step 1: Replace the AI panel/button with CommandPalette mount**

Rewrite `src/components/Layout.tsx`:

```tsx
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { TopBar } from "./TopBar";
import { Sidebar } from "./Sidebar";
import { CommandPalette } from "../command/CommandPalette";
import { buildLocalCommands } from "../command/dispatch";
import type { Tab, Priority, Category } from "../types";
import { PRIORITIES, CATEGORIES } from "../types";
import { useToast } from "../ui/Toast";
import * as api from "../api";

export function Layout({
  children,
}: {
  children: (props: {
    activeTab: Tab;
    priorities: Set<Priority>;
    categories: Set<Category>;
  }) => React.ReactNode;
}) {
  const [activeTab, setActiveTab] = useState<Tab>("projects");
  const [priorities, setPriorities] = useState<Set<Priority>>(new Set(PRIORITIES));
  const [categories, setCategories] = useState<Set<Category>>(new Set(CATEGORIES));
  const toast = useToast();

  const togglePriority = (p: Priority) => {
    setPriorities((prev) => {
      const next = new Set(prev);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });
  };
  const toggleCategory = (c: Category) => {
    setCategories((prev) => {
      const next = new Set(prev);
      next.has(c) ? next.delete(c) : next.add(c);
      return next;
    });
  };

  const handleImport = async () => {
    const file = await open({
      filters: [{ name: "Excel", extensions: ["xlsx", "xls"] }],
    });
    if (!file) return;
    const clearExisting = confirm("기존 데이터를 삭제하고 새로 가져오시겠습니까?");
    try {
      const result = await api.importExcel(
        typeof file === "string" ? file : (file as { path: string }).path,
        clearExisting
      );
      toast.success(`${result.projects_imported}개 프로젝트 가져왔습니다`);
      setTimeout(() => window.location.reload(), 800);
    } catch (e) {
      toast.error(`가져오기 실패: ${e}`);
    }
  };

  const handleBackup = async () => {
    try {
      const path = await api.backupDb();
      toast.success(`백업 완료: ${path}`);
    } catch (e) {
      toast.error(`백업 실패: ${e}`);
    }
  };

  const commands = buildLocalCommands({
    openNewProject: () => setActiveTab("projects"),
    openNewSchedule: () => setActiveTab("calendar"),
    openNewMemo: () => setActiveTab("memos"),
  });

  return (
    <div className="h-screen flex flex-col bg-[var(--color-surface-0)]">
      <TopBar
        active={activeTab}
        onChange={setActiveTab}
        onImport={handleImport}
        onBackup={handleBackup}
      />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar
          activePriorities={priorities}
          activeCategories={categories}
          onTogglePriority={togglePriority}
          onToggleCategory={toggleCategory}
        />
        <main className="flex-1 overflow-y-auto p-4">
          {children({ activeTab, priorities, categories })}
        </main>
      </div>
      <CommandPalette commands={commands} />
    </div>
  );
}
```

- [ ] **Step 2: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run `npm run tauri dev` → press ⌘K, confirm palette opens, `/` filters, Enter runs a local action, ESC closes, mutation shows confirm dialog. AI button gone. Kill when confirmed.

- [ ] **Step 3: Commit**

```bash
git add src/components/Layout.tsx
git commit -m "feat(layout): mount CommandPalette, drop AiPanel button"
```

---

## Phase 4 — AI Backend Rewrite

### Task 24: Update `api.ts` and Rust command signatures' shape (frontend contract)

**Files:**
- Modify: `src/api.ts`

- [ ] **Step 1: Replace AI section of `src/api.ts`**

Replace the `// AI` block at the bottom of `src/api.ts` with:

```ts
// AI
import type { AiResponse, AiServerState, ChatMessage as _CM } from "./types";

export const startAiServer = () => invoke<AiServerState>("start_ai_server");
export const stopAiServer = () => invoke<void>("stop_ai_server");
export const aiServerStatus = () => invoke<AiServerState>("ai_server_status");
export const aiChat = (messages: _CM[]) =>
  invoke<AiResponse>("ai_chat", { messages });
```

Remove the old `AiServerStatus` / `ChatResponse` imports from `./types` at the top (replace with `AiResponse` only). The file's top import should look like:

```ts
import type {
  Project,
  Schedule,
  Memo,
  Client,
  BackupInfo,
} from "./types";
```

(Keep `ChatMessage` import via the inner `import type ... from "./types"` at the bottom, aliased as `_CM` to avoid collision if needed.)

- [ ] **Step 2: Remove the now-unused `ChatResponse` export from `src/types.ts`**

Delete the `ChatResponse` interface and the `ToolCall` interface from `src/types.ts`. They are replaced by `AiResponse` + `AiAction`.

- [ ] **Step 3: Type-check**

Run: `npx tsc --noEmit`
Expected: may fail because `useAi.ts` still references old types. That is expected — Task 27 rewrites it. **Skip failure for now** — the Rust side (Tasks 25-26) must be updated first so that after Task 27, tsc passes. Just make sure this task's own edits compile in isolation by running `npx tsc --noEmit src/api.ts` which will exit cleanly; the hook's errors are acknowledged.

- [ ] **Step 4: Commit**

```bash
git add src/api.ts src/types.ts
git commit -m "refactor(api): align AI bindings with new AiResponse shape

useAi.ts still references old types — rewritten in follow-up commit."
```

---

### Task 25: Rust `AiManager` state machine + `cmd_ai.rs` rewrite (skeleton + lifecycle)

**Files:**
- Modify: `src-tauri/src/cmd_ai.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Write new `cmd_ai.rs`**

Replace `src-tauri/src/cmd_ai.rs` with:

```rust
use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;

// ---------- State machine ----------

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AiServerState {
    Idle,
    Starting,
    Running { port: u16 },
    Failed { error: String },
}

pub struct AiManager {
    pub state: Mutex<AiServerState>,
    pub child: Mutex<Option<Child>>,
    pub script_path: Mutex<String>,
    pub last_try: Mutex<Option<Instant>>,
}

impl AiManager {
    pub fn new(script_path: String) -> Self {
        Self {
            state: Mutex::new(AiServerState::Idle),
            child: Mutex::new(None),
            script_path: Mutex::new(script_path),
            last_try: Mutex::new(None),
        }
    }
}

// ---------- Commands: lifecycle ----------

const PORT: u16 = 8080;
const HEALTH_URL: &str = "http://127.0.0.1:8080/v1/models";

async fn is_alive(client: &reqwest::Client) -> bool {
    client.get(HEALTH_URL).send().await.is_ok()
}

#[tauri::command]
pub async fn start_ai_server(mgr: State<'_, AiManager>) -> Result<AiServerState, String> {
    let client = reqwest::Client::new();

    // Already running (possibly external instance)? Adopt.
    if is_alive(&client).await {
        let s = AiServerState::Running { port: PORT };
        *mgr.state.lock().map_err(|e| e.to_string())? = s.clone();
        return Ok(s);
    }

    // Mark starting.
    {
        *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Starting;
        *mgr.last_try.lock().map_err(|e| e.to_string())? = Some(Instant::now());
    }

    let script = mgr.script_path.lock().map_err(|e| e.to_string())?.clone();
    let child = Command::new("bash")
        .arg(&script)
        .spawn()
        .map_err(|e| {
            let err = format!("spawn failed: {}", e);
            *mgr.state.lock().unwrap() = AiServerState::Failed { error: err.clone() };
            err
        })?;

    *mgr.child.lock().map_err(|e| e.to_string())? = Some(child);

    // Poll up to 120s.
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if is_alive(&client).await {
            let s = AiServerState::Running { port: PORT };
            *mgr.state.lock().map_err(|e| e.to_string())? = s.clone();
            return Ok(s);
        }
    }

    let err = "AI server failed to start within 120s".to_string();
    *mgr.state.lock().map_err(|e| e.to_string())? =
        AiServerState::Failed { error: err.clone() };
    Err(err)
}

#[tauri::command]
pub async fn ai_server_status(mgr: State<'_, AiManager>) -> Result<AiServerState, String> {
    let client = reqwest::Client::new();
    let alive = is_alive(&client).await;
    let mut state = mgr.state.lock().map_err(|e| e.to_string())?;
    if alive {
        *state = AiServerState::Running { port: PORT };
    } else if matches!(*state, AiServerState::Running { .. }) {
        *state = AiServerState::Idle;
    }
    Ok(state.clone())
}

#[tauri::command]
pub async fn stop_ai_server(mgr: State<'_, AiManager>) -> Result<(), String> {
    kill_child(&mgr);
    *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Idle;
    Ok(())
}

/// Targeted kill of the exact child we spawned. Called from window-destroyed hook
/// in `lib.rs` — does NOT use `pkill -f mlx_lm.server` (that would kill siblings).
pub fn kill_child(mgr: &AiManager) {
    if let Ok(mut guard) = mgr.child.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------- Commands: chat ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Mutation,
    Navigation,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAction {
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub reply: String,
    pub actions: Vec<AiAction>,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["reply", "actions"],
        "properties": {
            "reply": { "type": "string" },
            "actions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["type", "label"],
                    "properties": {
                        "type": { "enum": ["mutation", "navigation", "info"] },
                        "label": { "type": "string" },
                        "command": {
                            "enum": [
                                "create_project", "update_project", "delete_project",
                                "create_schedule", "update_schedule", "delete_schedule",
                                "create_memo", "update_memo", "delete_memo",
                                "set_filter", "focus_project"
                            ]
                        },
                        "args": { "type": "object" }
                    }
                }
            }
        }
    })
}

#[tauri::command]
pub async fn ai_chat(messages: Vec<ChatMessage>) -> Result<AiResponse, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "default",
        "messages": messages,
        "max_tokens": 2048,
        "temperature": 0.4,
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": "genie_response", "schema": schema(), "strict": true }
        }
    });

    let resp = client
        .post("http://127.0.0.1:8080/v1/chat/completions")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse AI response failed: {}", e))?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    parse_ai_content(&content)
}

/// Parse assistant content into `AiResponse`. If the server ignored
/// `response_format`, the content may contain prose + json. We strip code
/// fences and try progressively looser parses. Returns `Err` so the frontend
/// can retry with a "format error, try again" follow-up.
pub fn parse_ai_content(content: &str) -> Result<AiResponse, String> {
    // Strip ```json ... ``` fences if present.
    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 1) Direct parse.
    if let Ok(r) = serde_json::from_str::<AiResponse>(cleaned) {
        return Ok(r);
    }

    // 2) Find the first balanced JSON object substring.
    if let Some(slice) = extract_first_json_object(cleaned) {
        if let Ok(r) = serde_json::from_str::<AiResponse>(slice) {
            return Ok(r);
        }
    }

    Err(format!("AI 응답 파싱 실패: {}", cleaned))
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = r#"{"reply":"ok","actions":[]}"#;
        let r = parse_ai_content(raw).expect("parse");
        assert_eq!(r.reply, "ok");
        assert!(r.actions.is_empty());
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let raw = "여기 결과입니다:\n```json\n{\"reply\":\"hi\",\"actions\":[{\"type\":\"info\",\"label\":\"noop\"}]}\n```";
        let r = parse_ai_content(raw).expect("parse");
        assert_eq!(r.reply, "hi");
        assert_eq!(r.actions.len(), 1);
    }

    #[test]
    fn fails_on_garbage() {
        let raw = "sorry, I couldn't";
        assert!(parse_ai_content(raw).is_err());
    }
}
```

- [ ] **Step 2: Update `src-tauri/src/lib.rs`**

Edit `src-tauri/src/lib.rs`:

- Replace the `AiState` setup:

```rust
// Before:
app.manage(cmd_ai::AiState {
    pid: Mutex::new(None),
    script_path: Mutex::new(
        "/Users/genie/dev/side/supergemma-bench/start-mlx.sh".to_string(),
    ),
});
// After:
app.manage(cmd_ai::AiManager::new(
    "/Users/genie/dev/side/supergemma-bench/start-mlx.sh".to_string(),
));
```

- Update the window-destroyed hook to also kill the AI child (no more `pkill`):

```rust
.on_window_event(|window, event| {
    if let tauri::WindowEvent::Destroyed = event {
        cmd_backup::auto_backup_on_close(window.app_handle());
        if let Some(mgr) = window.app_handle().try_state::<cmd_ai::AiManager>() {
            cmd_ai::kill_child(&mgr);
        }
    }
})
```

- The `invoke_handler!` entries `cmd_ai::start_ai_server`, `cmd_ai::stop_ai_server`, `cmd_ai::ai_server_status`, `cmd_ai::ai_chat` stay as-is (same command names).

- [ ] **Step 3: Run Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p tauri-app parse`
Expected: `parses_plain_json`, `parses_fenced_json_with_prose`, `fails_on_garbage` all pass.

- [ ] **Step 4: Compile check**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd_ai.rs src-tauri/src/lib.rs
git commit -m "feat(ai): rewrite cmd_ai with state machine, json_schema, targeted kill"
```

---

### Task 26: Rewrite `useAi.ts`

**Files:**
- Modify: `src/hooks/useAi.ts`

- [ ] **Step 1: Replace entire file**

Replace `src/hooks/useAi.ts` with:

```ts
import { useCallback, useRef, useState } from "react";
import type { AiResponse, AiServerState, ChatMessage } from "../types";
import * as api from "../api";

interface SystemContext {
  /** Rendered once as the system prompt (snapshot injected). */
  systemPrompt: string;
}

export function useAi() {
  const [serverState, setServerState] = useState<AiServerState>({ kind: "idle" });
  const [loading, setLoading] = useState(false);
  const historyRef = useRef<ChatMessage[]>([]);

  const ensureRunning = useCallback(async (): Promise<AiServerState> => {
    const current = await api.aiServerStatus();
    setServerState(current);
    if (current.kind === "running") return current;

    setServerState({ kind: "starting" });
    try {
      const next = await api.startAiServer();
      setServerState(next);
      return next;
    } catch (e) {
      const failed: AiServerState = { kind: "failed", error: String(e) };
      setServerState(failed);
      return failed;
    }
  }, []);

  /**
   * Send a user query. Returns the parsed AiResponse or throws.
   * History is maintained in-hook; system prompt is re-rendered fresh every call
   * so the snapshot (projects/schedules/memos) is always current.
   */
  const sendQuery = useCallback(
    async (text: string, ctx: SystemContext): Promise<AiResponse> => {
      const state = await ensureRunning();
      if (state.kind !== "running") {
        throw new Error(
          state.kind === "failed" ? `AI 서버 시작 실패: ${state.error}` : "AI 서버가 실행 중이 아닙니다"
        );
      }

      const userMsg: ChatMessage = { role: "user", content: text };
      const messages: ChatMessage[] = [
        { role: "system", content: ctx.systemPrompt },
        ...historyRef.current,
        userMsg,
      ];

      setLoading(true);
      try {
        const response = await api.aiChat(messages);
        historyRef.current = [
          ...historyRef.current,
          userMsg,
          { role: "assistant", content: JSON.stringify(response) },
        ];
        return response;
      } finally {
        setLoading(false);
      }
    },
    [ensureRunning]
  );

  const resetHistory = useCallback(() => {
    historyRef.current = [];
  }, []);

  return { serverState, loading, sendQuery, resetHistory, ensureRunning };
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/hooks/useAi.ts
git commit -m "feat(ai): rewrite useAi hook with lazy start and history ref"
```

---

### Task 27: Wire AI mode into `CommandPalette`

**Files:**
- Modify: `src/command/CommandPalette.tsx`
- Create: `src/command/buildSystemPrompt.ts`
- Create: `src/command/executeAiAction.ts`

This task is the biggest user-visible payoff. Splits into three files for focus.

- [ ] **Step 1: Write `buildSystemPrompt.ts`**

```ts
// src/command/buildSystemPrompt.ts
import type { Project, Schedule, Memo } from "../types";

const HEADER = `너는 Hearth 의 AI 어시스턴트다. 한국어로 답한다.
사용자 요청에 JSON 으로 응답한다. "reply" 는 자연어, "actions" 는 수행할 액션 배열 (없으면 빈 배열).

사용 가능한 command:
  create_project(name, priority, category?, path?)
  update_project(id, fields)
  delete_project(id)
  create_schedule(date, time?, location?, description?, notes?)
  update_schedule(id, fields)
  delete_schedule(id)
  create_memo(content, color?, project_id?)
  update_memo(id, fields)
  delete_memo(id)
  set_filter(priorities?, categories?)
  focus_project(id)

규칙:
1) 생성/수정/삭제 command (create_*, update_*, delete_*) 는 모두 type: mutation — 실행은 사용자가 UI 에서 확인한다.
2) set_filter, focus_project 는 type: navigation — 확인 없이 즉시 실행.
3) 단순 조회/요약은 reply 에만 서술, actions 는 빈 배열.
4) 존재하지 않는 프로젝트/일정/메모 는 추측하지 않고 사용자에게 되물어본다.`;

export function buildSystemPrompt(snapshot: {
  projects: Project[];
  schedules: Schedule[];
  memos: Memo[];
}): string {
  const { projects, schedules, memos } = snapshot;
  const byPri = (p: string) => projects.filter((pr) => pr.priority === p).length;
  const stats = `현재 상태:
- 프로젝트 ${projects.length}개 (P0 ${byPri("P0")}, P1 ${byPri("P1")}, P2 ${byPri("P2")}, P3 ${byPri("P3")}, P4 ${byPri("P4")})
- 일정 ${schedules.length}개
- 메모 ${memos.length}개`;

  const projectList =
    "[프로젝트 목록]\n" +
    projects
      .slice(0, 50)
      .map((p) => `#${p.id} [${p.priority}] ${p.name}${p.category ? ` (${p.category})` : ""}`)
      .join("\n");

  const scheduleList =
    "[이번 달 일정]\n" +
    schedules
      .slice(0, 30)
      .map((s) => `#${s.id} ${s.date}${s.time ? ` ${s.time}` : ""} ${s.description ?? ""}${s.location ? ` @ ${s.location}` : ""}`)
      .join("\n");

  const memoList =
    "[최근 메모 10개]\n" +
    memos
      .slice(0, 10)
      .map((m) => `#${m.id} ${m.content.slice(0, 80)}`)
      .join("\n");

  return [HEADER, stats, projectList, scheduleList, memoList].join("\n\n");
}
```

- [ ] **Step 2: Write `executeAiAction.ts`**

```ts
// src/command/executeAiAction.ts
import type { AiAction } from "../types";
import * as api from "../api";

type Args = Record<string, unknown>;

/**
 * Execute an AI-produced action. Returns an optional undo function for Toast.
 * Only a subset of commands have reliable inverses; others return undefined.
 */
export async function executeAiAction(
  action: AiAction,
  onNavigate?: (kind: "filter" | "focusProject", payload: Args) => void
): Promise<(() => Promise<void>) | undefined> {
  const args = (action.args ?? {}) as Args;

  switch (action.command) {
    case "create_project": {
      const created = await api.createProject(
        String(args.name ?? ""),
        String(args.priority ?? "P2"),
        args.category ? String(args.category) : undefined,
        args.path ? String(args.path) : undefined
      );
      return async () => {
        await api.deleteProject(created.id);
      };
    }
    case "update_project": {
      const id = Number(args.id);
      const fields = (args.fields ?? {}) as Record<string, string>;
      await api.updateProject(id, fields);
      return; // no reliable inverse without snapshotting prior fields
    }
    case "delete_project": {
      await api.deleteProject(Number(args.id));
      return;
    }
    case "create_schedule": {
      const data = args as { date: string; time?: string; location?: string; description?: string; notes?: string };
      const created = await api.createSchedule(data);
      return async () => {
        await api.deleteSchedule(created.id);
      };
    }
    case "update_schedule": {
      const id = Number(args.id);
      const fields = args.fields as { date: string; time?: string; location?: string; description?: string; notes?: string };
      await api.updateSchedule(id, fields);
      return;
    }
    case "delete_schedule": {
      await api.deleteSchedule(Number(args.id));
      return;
    }
    case "create_memo": {
      const data = args as { content: string; color?: string; project_id?: number };
      const created = await api.createMemo(data);
      return async () => {
        await api.deleteMemo(created.id);
      };
    }
    case "update_memo": {
      const id = Number(args.id);
      const fields = args.fields as { content?: string; color?: string; project_id?: number | null };
      await api.updateMemo(id, fields);
      return;
    }
    case "delete_memo": {
      await api.deleteMemo(Number(args.id));
      return;
    }
    case "set_filter": {
      onNavigate?.("filter", args);
      return;
    }
    case "focus_project": {
      onNavigate?.("focusProject", args);
      return;
    }
    default:
      return;
  }
}
```

- [ ] **Step 3: Extend `CommandPalette.tsx` with AI mode**

Edit `src/command/CommandPalette.tsx` — add AI branch. Apply these edits to the existing file:

1. Update the top imports to include:

```ts
import { useEffect as _useEffect2 } from "react"; // (or just add these below useEffect)
import { useAi } from "../hooks/useAi";
import { buildSystemPrompt } from "./buildSystemPrompt";
import { executeAiAction } from "./executeAiAction";
import { itemFromAi } from "./CommandResults";
import type { AiAction } from "../types";
import type { Project, Schedule, Memo } from "../types";
```

(Drop the `_useEffect2` alias — just use the existing `useEffect` import.)

2. Change the component signature to accept a snapshot source:

```ts
export function CommandPalette({
  commands,
  snapshot,
}: {
  commands: LocalCommand[];
  snapshot: () => { projects: Project[]; schedules: Schedule[]; memos: Memo[] };
}) {
```

3. Inside the component body, add after the existing `state` / `toast` / refs:

```ts
const ai = useAi();
const [aiReply, setAiReply] = useState<string | undefined>(undefined);
const [aiActions, setAiActions] = useState<AiAction[]>([]);
const [pendingAiConfirm, setPendingAiConfirm] = useState<AiAction | null>(null);

// Debounced AI fire when mode === 'ai'
useEffect(() => {
  setAiReply(undefined);
  setAiActions([]);
  if (state.mode !== "ai" || state.aiQuery.length === 0) return;
  const q = state.aiQuery;
  const handle = setTimeout(async () => {
    try {
      const resp = await ai.sendQuery(q, {
        systemPrompt: buildSystemPrompt(snapshot()),
      });
      setAiReply(resp.reply);
      setAiActions(resp.actions);
    } catch (e) {
      toast.error(`AI 오류: ${e}`);
    }
  }, 300);
  return () => clearTimeout(handle);
}, [state.mode, state.aiQuery, ai, toast, snapshot]);
```

4. Change `items` memo to merge AI actions when in AI mode:

```ts
const items: ResultItem[] = useMemo(() => {
  if (state.mode === "ai") return aiActions.map((a, i) => itemFromAi(a, i));
  return state.localMatches.map(itemFromLocal);
}, [state.mode, state.localMatches, aiActions]);
```

5. Change `onSelect` to branch on mode:

```ts
const onSelect = useCallback(
  (i: number) => {
    if (state.mode === "ai") {
      const action = aiActions[i];
      if (!action) return;
      if (action.type === "mutation") {
        setPendingAiConfirm(action);
      } else {
        runAi(action);
      }
      return;
    }
    const cmd = state.localMatches[i];
    if (!cmd) return;
    if (cmd.mutation) {
      setPendingConfirm(cmd);
    } else {
      executeCommand(cmd);
    }
  },
  [state.mode, state.localMatches, aiActions, executeCommand]
);

const runAi = useCallback(
  async (action: AiAction) => {
    try {
      const undo = await executeAiAction(action);
      toast.success(`${action.label} 완료`, { undo });
      close();
    } catch (e) {
      toast.error(`${action.label} 실패: ${e}`);
    }
  },
  [toast, close]
);
```

6. Pass `aiReply` into `<CommandResults>`:

```tsx
<CommandResults
  items={items}
  activeIndex={activeIndex}
  onHover={setActiveIndex}
  onSelect={onSelect}
  aiReply={state.mode === "ai" ? aiReply : undefined}
/>
```

7. Add the second confirmation Dialog for AI mutations (after the existing local-confirm Dialog):

```tsx
<Dialog
  open={!!pendingAiConfirm}
  onClose={() => setPendingAiConfirm(null)}
  labelledBy="ai-confirm-title"
>
  {pendingAiConfirm && (
    <>
      <h2 id="ai-confirm-title" className="text-heading text-[var(--color-text-hi)] mb-2">
        확인
      </h2>
      <p className="text-[13px] text-[var(--color-text)] mb-5">
        {pendingAiConfirm.label}
      </p>
      <div className="flex justify-end gap-2">
        <Button variant="secondary" onClick={() => setPendingAiConfirm(null)}>
          취소
        </Button>
        <Button
          variant="primary"
          autoFocus
          onClick={() => {
            const a = pendingAiConfirm;
            setPendingAiConfirm(null);
            runAi(a);
          }}
        >
          실행
        </Button>
      </div>
    </>
  )}
</Dialog>
```

8. Placeholder update: when mode === 'ai' and loading, the loading prop on `<CommandInput>` should flip. Pass `loading={ai.loading}`.

9. Update `close()` to clear AI state too:

```ts
const close = useCallback(() => {
  state.setOpen(false);
  state.reset();
  setPendingConfirm(null);
  setPendingAiConfirm(null);
  setAiReply(undefined);
  setAiActions([]);
}, [state]);
```

- [ ] **Step 4: Wire snapshot prop in `Layout.tsx`**

Edit `src/components/Layout.tsx` — the snapshot comes from the three domain hooks. Since Layout doesn't currently hold the data (children render the lists), the simplest fix is to fetch snapshots lazily inside `snapshot()`:

```ts
// in Layout.tsx, add:
import * as api from "../api";
// ...
<CommandPalette
  commands={commands}
  snapshot={async () => ({
    projects: await api.getProjects(),
    schedules: await api.getSchedules(),
    memos: await api.getMemos(),
  }) as any}
/>
```

**Note**: the current `snapshot` signature is sync. Change the signature in `CommandPalette.tsx` to:

```ts
snapshot: () => Promise<{ projects: Project[]; schedules: Schedule[]; memos: Memo[] }>;
```

and in the debounced effect:

```ts
const snap = await snapshot();
const resp = await ai.sendQuery(q, { systemPrompt: buildSystemPrompt(snap) });
```

- [ ] **Step 5: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

`npm run tauri dev`:
1. ⌘K → `/` → local actions filter. ✅
2. ⌘K → `? 프로젝트 P0 몇 개야` → loading spinner → reply appears (server lazy-starts first time ~60s; subsequent calls instant). ✅
3. Ask AI to "새 프로젝트 만들어" → action appears → ⏎ → confirm dialog → 실행 → toast w/ Undo → list reloads. ✅
4. Close palette, open again — AI server stays running (check `curl http://127.0.0.1:8080/v1/models`). ✅

Kill dev server when confirmed.

- [ ] **Step 6: Commit**

```bash
git add src/command/CommandPalette.tsx src/command/buildSystemPrompt.ts src/command/executeAiAction.ts src/components/Layout.tsx
git commit -m "feat(command): wire AI mode into palette with confirm + toast/undo"
```

---

## Phase 5 — Cleanup

### Task 28: Delete legacy AI UI and sweep for emojis

**Files:**
- Delete: `src/components/AiPanel.tsx`, `src/components/ChatMessage.tsx`
- Modify: any file flagged by the emoji scan

- [ ] **Step 1: Delete legacy files**

Run:

```bash
git rm src/components/AiPanel.tsx src/components/ChatMessage.tsx
```

- [ ] **Step 2: Remove dead imports**

Run: `npx tsc --noEmit`
If errors reference `AiPanel` or `ChatMessage`, open the offending file and drop the import.

- [ ] **Step 3: Emoji audit**

Run via Bash: `rg --pcre2 '[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2190}-\x{21FF}]' src 2>&1 || echo 'no matches'`

Expected: `no matches` or any remaining matches inside comments/placeholders should be removed.

If matches exist, edit each file and replace with a lucide icon or delete the literal. Common residuals to check:
- Placeholders like `"🤖 ..."` in strings
- `▶`, `📁`, `✕`, `➕`, `⠿`, `📌`, `📋`, `📅`, `📥`, `💾`, `🕐` in any leftover component

Re-run the grep until it reports no matches.

- [ ] **Step 4: Final manual checklist (from spec §5.7)**

With `npm run tauri dev`:
- [ ] ⌘K opens anywhere; ESC closes.
- [ ] `/` → local quick actions only.
- [ ] `?` or bare natural-language query → AI path.
- [ ] `mutation` type AI action → confirm Dialog.
- [ ] Executed action → list refreshes + Toast.
- [ ] Undo toast reverts creates.
- [ ] AI server starts lazily, loading indicator shown, response lands.
- [ ] Closing palette leaves server running (curl health endpoint succeeds).
- [ ] Quit app → `pgrep -f mlx_lm.server` shows no leftover process.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: drop AiPanel/ChatMessage, scrub remaining emojis"
```

---

## Self-Review Notes

- **Spec coverage**: Every section of the spec is mapped to a task — §1 tokens (T3), §2 primitives (T4–T8), §2.4 removals (T28), §3 palette (T18–T23 + T27), §4.2 lifecycle (T25), §4.3 schema (T25), §4.4 system prompt (T27, `buildSystemPrompt.ts`), §4.6 signatures (T24–T25), §4.7 pipe (T26–T27), §5 phases (T1–T28 in order).
- **Type consistency**: `AiResponse` / `AiAction` / `ActionCommand` / `AiServerState` are defined once in `src/types.ts` (T10) and referenced identically in Rust (T25) and frontend (T24, T26, T27). Palette uses `executeAiAction` (not `executeAction`) consistently. `LocalCommand.run` returns `(() => void | Promise<void>) | void` — matches Toast `undo` signature.
- **Placeholders**: None. Every step shows real code. Error-handling scaffolds use the Toast API (`toast.error`) defined in T8, not generic "handle errors appropriately."
- **Dependencies respected**: T2 (cn) before T5 (Button uses cn); T4 (Icon) before T5; T10 (types) before T24 (api); T25 (Rust) and T26 (useAi) before T27 (palette AI wiring).
- **Risks flagged**:
  - `Layout.tsx` now passes a truly async snapshot function — make sure subagents don't revert this when implementing T27 Step 4.
  - If `mlx_lm.server` doesn't support `response_format: json_schema`, the parser in `parse_ai_content` still salvages fenced JSON (fallback per spec §4.3 covered by the `parses_fenced_json_with_prose` test).

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-16-redesign-and-ai-fix.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
