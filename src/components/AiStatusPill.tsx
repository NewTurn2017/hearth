// Passive AI status pill. Shows whether an OpenAI key is configured.
// Clicking opens the settings AI tab.
import { useAiStatus } from "../hooks/useAiStatus";
import { cn } from "../lib/cn";

export function AiStatusPill() {
  const status = useAiStatus();

  const configured = status === "configured";
  const label = configured ? "AI 준비됨" : "AI 키 필요";
  const title = configured
    ? "OpenAI 키가 저장되어 있습니다. ⌘K 로 AI 명령을 사용할 수 있어요."
    : "OpenAI 키가 없습니다. 설정 → AI 탭에서 키를 입력하세요.";
  const dot = configured
    ? "bg-[var(--color-success)]"
    : "bg-[var(--color-text-dim)]";

  const onClick = () => {
    window.dispatchEvent(
      new CustomEvent("settings:open", { detail: { tab: "ai" } })
    );
  };

  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        "inline-flex items-center gap-1.5 h-7 px-2.5 rounded-[var(--radius-sm)]",
        "text-[11px] text-[var(--color-text-muted)]",
        "border border-[var(--color-border)] bg-[var(--color-surface-1)]",
        "transition-colors duration-[120ms]",
        "hover:bg-[var(--color-surface-2)] hover:text-[var(--color-text)]"
      )}
    >
      <span className={cn("w-1.5 h-1.5 rounded-full", dot)} aria-hidden />
      <span>{label}</span>
    </button>
  );
}
