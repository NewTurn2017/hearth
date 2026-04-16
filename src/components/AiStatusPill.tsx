// Compact AI server status pill for the TopBar. Click-to-start when idle,
// click-to-stop when running. Uses `useAiStatus` which polls the backend,
// keeping this decoupled from the CommandPalette's `useAi` instance.
import { Loader2 } from "lucide-react";
import { useAiStatus } from "../hooks/useAiStatus";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import * as api from "../api";

interface Style {
  dot: string;
  label: string;
  title: string;
  action: "start" | "stop" | "none";
}

function styleFor(state: ReturnType<typeof useAiStatus>): Style {
  switch (state.kind) {
    case "running":
      return {
        dot: "bg-[var(--color-success)]",
        label: "AI 실행 중",
        title: `AI 서버 실행 중 (포트 ${state.port}). 클릭해서 중지.`,
        action: "stop",
      };
    case "starting":
      return {
        dot: "",
        label: "AI 시작 중",
        title: "MLX 모델 로드 중…",
        action: "none",
      };
    case "failed":
      return {
        dot: "bg-[var(--color-danger)]",
        label: "AI 오류",
        title: state.error,
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
  const { dot, label, title, action } = styleFor(state);

  const onClick = async () => {
    if (action === "start") {
      try {
        await api.startAiServer();
      } catch (e) {
        toast.error(`AI 시작 실패: ${e}`);
      }
    } else if (action === "stop") {
      try {
        await api.stopAiServer();
        toast.success("AI 서버 중지됨");
      } catch (e) {
        toast.error(`AI 중지 실패: ${e}`);
      }
    }
  };

  const disabled = action === "none";

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
      {state.kind === "starting" ? (
        <Loader2 size={11} className="animate-spin text-[var(--color-brand-hi)]" aria-hidden />
      ) : (
        <span className={cn("w-1.5 h-1.5 rounded-full", dot)} aria-hidden />
      )}
      <span>{label}</span>
    </button>
  );
}
