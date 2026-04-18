// src/ui/Toast.tsx
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { CheckCircle2, AlertCircle, Info, RotateCcw, X } from "lucide-react";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";

type Kind = "success" | "error" | "info";

export interface ToastAction {
  label: string;
  run: () => void | Promise<void>;
}

interface ToastItem {
  id: number;
  kind: Kind;
  message: string;
  undo?: () => void | Promise<void>;
  sticky?: boolean;
  actions?: ToastAction[];
}

interface InfoOpts {
  sticky?: boolean;
  actions?: ToastAction[];
}

interface ToastApi {
  success: (message: string, opts?: { undo?: () => void | Promise<void> }) => void;
  error: (message: string) => void;
  info: (message: string, opts?: InfoOpts) => void;
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
    (item: Omit<ToastItem, "id">) => {
      const id = ++idRef.current;
      setItems((prev) => [...prev, { ...item, id }]);
      if (!item.sticky) {
        const timer = setTimeout(() => remove(id), TTL_MS);
        timers.current.set(id, timer);
      }
    },
    [remove]
  );

  // Memoize so the context value's identity is stable across re-renders.
  // Otherwise every toast re-renders ToastProvider → new `api` identity →
  // every consumer's `useEffect(..., [toast])` restarts (notably the
  // updater's 30s startup timer and 24h interval).
  const api = useMemo<ToastApi>(
    () => ({
      success: (message, opts) =>
        push({ kind: "success", message, undo: opts?.undo }),
      error: (message) => push({ kind: "error", message }),
      info: (message, opts) =>
        push({
          kind: "info",
          message,
          sticky: opts?.sticky,
          actions: opts?.actions,
        }),
    }),
    [push]
  );

  useEffect(() => () => timers.current.forEach(clearTimeout), []);

  return (
    <ToastCtx.Provider value={api}>
      {children}
      <div className="fixed bottom-4 right-4 z-[200] flex flex-col gap-2 pointer-events-none">
        {items.map((t) => {
          const iconMap = {
            success: CheckCircle2,
            error: AlertCircle,
            info: Info,
          } as const;
          const tintMap = {
            success: "text-[var(--color-success)]",
            error: "text-[var(--color-danger)]",
            info: "text-[var(--color-brand-hi)]",
          } as const;
          const borderMap = {
            success: "border-[var(--color-border)]",
            error: "border-[var(--color-danger)]",
            info: "border-[var(--color-brand-hi)]",
          } as const;
          return (
            <div
              key={t.id}
              className={cn(
                "pointer-events-auto flex items-center gap-2 min-w-[260px] max-w-[420px]",
                "px-3 py-2 rounded-[var(--radius-md)] border shadow-[var(--shadow-e2)]",
                "bg-[var(--color-surface-2)]",
                borderMap[t.kind]
              )}
            >
              <Icon icon={iconMap[t.kind]} size={16} className={tintMap[t.kind]} />
              <span className="flex-1 text-[12px] text-[var(--color-text)]">
                {t.message}
              </span>
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
              {t.actions?.map((a) => (
                <button
                  key={a.label}
                  onClick={async () => {
                    try {
                      await a.run();
                    } finally {
                      remove(t.id);
                    }
                  }}
                  className="inline-flex items-center text-[11px] text-[var(--color-brand-hi)] hover:underline whitespace-nowrap"
                >
                  {a.label}
                </button>
              ))}
              <button
                onClick={() => remove(t.id)}
                className="text-[var(--color-text-dim)] hover:text-[var(--color-text)]"
                aria-label="dismiss"
              >
                <Icon icon={X} size={14} />
              </button>
            </div>
          );
        })}
      </div>
    </ToastCtx.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastCtx);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}
