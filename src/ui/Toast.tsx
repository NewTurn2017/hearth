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
