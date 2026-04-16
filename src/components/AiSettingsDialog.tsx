// AI provider selector.
//
// Two backends are supported: local MLX (free, offline) and OpenAI
// (pay-per-call, no setup). Model selection is intentionally absent — the
// backend hard-codes the OpenAI model and auto-detects the local MLX model
// from the running process, so the dialog only persists the provider choice
// and an OpenAI API key.
//
// The key is write-only from the UI's perspective: the backend returns a
// `has_openai_key` boolean and never echoes the value back — the password
// input stays empty on load, and leaving it empty on save means "keep
// whatever is already stored".

import { useEffect, useState } from "react";
import { Loader2, Trash2 } from "lucide-react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import type { AiSettings } from "../types";
import * as api from "../api";

type Provider = AiSettings["provider"];

export function AiSettingsDialog({
  open,
  onClose,
  onSaved,
}: {
  open: boolean;
  onClose: () => void;
  /** Called after a successful save so the parent can invalidate cached AI
   *  state (e.g. `useAi.resetServerState()` so the status pill rechecks). */
  onSaved?: () => void;
}) {
  const toast = useToast();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [provider, setProvider] = useState<Provider>("local");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [hasStoredKey, setHasStoredKey] = useState(false);

  // Reload settings every time the dialog opens so we don't show stale values
  // if another surface changed them.
  useEffect(() => {
    if (!open) return;
    setLoading(true);
    setApiKeyInput("");
    api
      .getAiSettings()
      .then((s) => {
        setProvider(s.provider);
        setHasStoredKey(s.has_openai_key);
      })
      .catch((e) => toast.error(`설정 불러오기 실패: ${e}`))
      .finally(() => setLoading(false));
  }, [open, toast]);

  const handleSave = async () => {
    setSaving(true);
    try {
      // `apiKeyInput` is empty on load; only forward it when the user typed
      // something, otherwise the backend would clear a stored key.
      const result = await api.saveAiSettings({
        provider,
        openai_api_key: apiKeyInput.length > 0 ? apiKeyInput : undefined,
      });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("AI 설정 저장됨");
      // Nudge status surfaces (AiStatusPill etc.) so they re-probe with the
      // new provider immediately instead of waiting for the next 5s poll.
      window.dispatchEvent(new CustomEvent("ai-settings:changed"));
      onSaved?.();
      onClose();
    } catch (e) {
      toast.error(`저장 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleClearKey = async () => {
    setSaving(true);
    try {
      const result = await api.saveAiSettings({
        provider,
        openai_api_key: "",
      });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("저장된 API 키 삭제됨");
    } catch (e) {
      toast.error(`삭제 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const canSave = !saving && !loading;

  return (
    <Dialog open={open} onClose={onClose} labelledBy="ai-settings-title">
      <h2
        id="ai-settings-title"
        className="text-heading text-[var(--color-text-hi)] mb-1"
      >
        AI 설정
      </h2>
      <p className="text-[12px] text-[var(--color-text-dim)] mb-5">
        로컬 MLX 서버 또는 OpenAI API 중 선택하세요. 설정은 앱 데이터 폴더에 저장됩니다.
      </p>

      {loading ? (
        <div className="flex items-center gap-2 text-[13px] text-[var(--color-text-muted)] py-6">
          <Loader2 size={14} className="animate-spin" aria-hidden />
          <span>불러오는 중…</span>
        </div>
      ) : (
        <div className="flex flex-col gap-5">
          <Field label="제공자">
            <div className="grid grid-cols-2 gap-2">
              <ProviderOption
                active={provider === "local"}
                onClick={() => setProvider("local")}
                title="로컬 MLX"
                subtitle="오프라인 · 무료"
              />
              <ProviderOption
                active={provider === "openai"}
                onClick={() => setProvider("openai")}
                title="OpenAI"
                subtitle="API 키 필요 · 종량제"
              />
            </div>
          </Field>

          {provider === "openai" && (
            <Field
              label="OpenAI API 키"
              hint={
                hasStoredKey
                  ? "저장된 키가 있습니다. 새 키를 입력하면 덮어씁니다."
                  : "sk-로 시작하는 키. 로컬 DB에 평문으로 저장됩니다."
              }
              right={
                hasStoredKey ? (
                  <span className="text-[11px] text-[var(--color-success)] font-medium">
                    저장됨
                  </span>
                ) : null
              }
            >
              <div className="flex gap-2">
                <input
                  type="password"
                  value={apiKeyInput}
                  onChange={(e) => setApiKeyInput(e.target.value)}
                  placeholder={hasStoredKey ? "••••••••••••••••" : "sk-..."}
                  autoComplete="off"
                  spellCheck={false}
                  className={cn(
                    "flex-1 h-9 px-3 text-[13px] rounded-[var(--radius-md)]",
                    "bg-[var(--color-surface-2)] text-[var(--color-text)]",
                    "border border-[var(--color-border)]",
                    "focus:outline-none focus:border-[var(--color-brand-hi)]"
                  )}
                />
                {hasStoredKey && (
                  <button
                    type="button"
                    onClick={handleClearKey}
                    disabled={saving}
                    title="저장된 API 키 삭제"
                    className={cn(
                      "shrink-0 w-9 h-9 inline-flex items-center justify-center",
                      "rounded-[var(--radius-md)] border border-[var(--color-border)]",
                      "text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]",
                      "transition-colors duration-[120ms]",
                      "disabled:opacity-50 disabled:cursor-not-allowed"
                    )}
                    aria-label="저장된 API 키 삭제"
                  >
                    <Icon icon={Trash2} size={14} />
                  </button>
                )}
              </div>
            </Field>
          )}
        </div>
      )}

      <div className="flex justify-end gap-2 mt-6">
        <Button variant="secondary" onClick={onClose} disabled={saving}>
          취소
        </Button>
        <Button variant="primary" onClick={handleSave} disabled={!canSave} autoFocus>
          {saving ? "저장 중…" : "저장"}
        </Button>
      </div>
    </Dialog>
  );
}

function Field({
  label,
  hint,
  right,
  children,
}: {
  label: string;
  hint?: string;
  right?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <label className="text-[12px] font-medium text-[var(--color-text)]">
          {label}
        </label>
        {right}
      </div>
      {children}
      {hint && (
        <p className="text-[11px] text-[var(--color-text-dim)] mt-1.5">{hint}</p>
      )}
    </div>
  );
}

function ProviderOption({
  active,
  onClick,
  title,
  subtitle,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  subtitle: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "flex flex-col items-start gap-0.5 p-3 rounded-[var(--radius-md)] text-left",
        "border transition-colors duration-[120ms]",
        active
          ? "border-[var(--color-brand-hi)] bg-[var(--color-brand-soft)]"
          : "border-[var(--color-border)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)]"
      )}
    >
      <span
        className={cn(
          "text-[13px] font-medium",
          active ? "text-[var(--color-brand-hi)]" : "text-[var(--color-text)]"
        )}
      >
        {title}
      </span>
      <span className="text-[11px] text-[var(--color-text-dim)]">{subtitle}</span>
    </button>
  );
}
