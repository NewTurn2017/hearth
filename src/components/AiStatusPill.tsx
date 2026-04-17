// Compact AI server status pill for the TopBar. Click-to-start when idle,
// click-to-stop when running. Uses `useAiStatus` which polls the backend
// every 5s, keeping this decoupled from the CommandPalette's `useAi`
// instance. An ephemeral `pending` overlay (starting/stopping) gives
// immediate feedback for the click→poll gap and disables the button so
// double-clicks cannot race the teardown.
import { useState } from "react";
import { Loader2 } from "lucide-react";
import { useAiStatus } from "../hooks/useAiStatus";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import * as api from "../api";

type DisplayKind = "idle" | "starting" | "running" | "failed" | "stopping";
type Pending = "starting" | "stopping" | null;

interface Style {
  dot: string;
  label: string;
  title: string;
  action: "start" | "stop" | "none";
}

function styleFor(kind: DisplayKind, port?: number, error?: string): Style {
  switch (kind) {
    case "running":
      return {
        dot: "bg-[var(--color-success)]",
        label: "AI 실행 중",
        title: `AI 서버 실행 중 (포트 ${port}). 클릭해서 중지.`,
        action: "stop",
      };
    case "starting":
      return {
        dot: "",
        label: "AI 시작 중",
        title: "MLX 모델 로드 중…",
        action: "none",
      };
    case "stopping":
      return {
        dot: "",
        label: "AI 중지 중",
        title: "AI 서버 종료 중…",
        action: "none",
      };
    case "failed":
      return {
        dot: "bg-[var(--color-danger)]",
        label: "AI 오류",
        title: error ?? "AI 오류",
        action: "start",
      };
    case "idle":
    default:
      return {
        dot: "bg-[var(--color-text-dim)]",
        label: "AI 대기",
        title: "AI 서버가 꺼져 있습니다. 클릭해서 시작.",
        action: "start",
      };
  }
}

export function AiStatusPill() {
  const state = useAiStatus();
  const toast = useToast();
  const [pending, setPending] = useState<Pending>(null);

  // Pending overlay wins over the polled server state so the pill reflects
  // the click immediately, even before the poll catches up.
  const displayKind: DisplayKind = pending ?? state.kind;
  const port = state.kind === "running" ? state.port : undefined;
  const error = state.kind === "failed" ? state.error : undefined;
  const { dot, label, title, action } = styleFor(displayKind, port, error);

  const onClick = async () => {
    if (pending) return;
    if (action === "start") {
      setPending("starting");
      try {
        await api.startAiServer();
      } catch (e) {
        toast.error(`AI 시작 실패: ${e}`);
      } finally {
        setPending(null);
        window.dispatchEvent(new CustomEvent("ai-server:changed"));
      }
    } else if (action === "stop") {
      setPending("stopping");
      try {
        await api.stopAiServer();
        toast.success("AI 서버 중지됨");
      } catch (e) {
        toast.error(`AI 중지 실패: ${e}`);
      } finally {
        setPending(null);
        window.dispatchEvent(new CustomEvent("ai-server:changed"));
      }
    }
  };

  const disabled = action === "none" || pending !== null;
  const showSpinner =
    displayKind === "starting" || displayKind === "stopping";

  return (
    <button
      type="button"
      title={title}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
      className={cn(
        "inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[var(--radius-sm)]",
        "text-[11px] text-[var(--color-text-muted)]",
        "border border-[var(--color-border)] bg-[var(--color-surface-1)]",
        "transition-colors duration-[120ms]",
        disabled
          ? "cursor-default opacity-80"
          : "hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]"
      )}
    >
      {showSpinner ? (
        <Loader2 size={11} className="animate-spin text-[var(--color-brand-hi)]" aria-hidden />
      ) : (
        <span className={cn("w-1.5 h-1.5 rounded-full", dot)} aria-hidden />
      )}
      <span>{label}</span>
    </button>
  );
}
