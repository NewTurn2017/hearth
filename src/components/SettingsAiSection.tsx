// OpenAI-only AI settings. MLX was removed in 0.3.0; a missing key means
// AI features fail gracefully from the command palette — the rest of the
// app is unaffected.

import { useEffect, useState } from "react";
import { Loader2, Trash2 } from "lucide-react";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import * as api from "../api";

export function SettingsAiSection({ active }: { active: boolean }) {
  const toast = useToast();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [hasStoredKey, setHasStoredKey] = useState(false);

  useEffect(() => {
    if (!active) return;
    setLoading(true);
    setApiKeyInput("");
    api
      .getAiSettings()
      .then((s) => setHasStoredKey(s.has_openai_key))
      .catch((e) => toast.error(`설정 불러오기 실패: ${e}`))
      .finally(() => setLoading(false));
  }, [active, toast]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const result = await api.saveAiSettings({
        openai_api_key: apiKeyInput.length > 0 ? apiKeyInput : undefined,
      });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("AI 설정 저장됨");
      window.dispatchEvent(new CustomEvent("ai-settings:changed"));
    } catch (e) {
      toast.error(`저장 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleClearKey = async () => {
    setSaving(true);
    try {
      const result = await api.saveAiSettings({ openai_api_key: "" });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("저장된 API 키 삭제됨");
      window.dispatchEvent(new CustomEvent("ai-settings:changed"));
    } catch (e) {
      toast.error(`삭제 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-[13px] text-[var(--color-text-muted)] py-6">
        <Loader2 size={14} className="animate-spin" aria-hidden />
        <span>불러오는 중…</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <p className="text-[12px] text-[var(--color-text-muted)]">
        AI 명령 팔레트(⌘K → 자연어)는 OpenAI 를 사용합니다. 키는 선택 사항이며
        입력하지 않아도 Hearth 의 나머지 기능은 정상 동작합니다.
      </p>

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

      <div className="flex justify-end">
        <Button variant="primary" onClick={handleSave} disabled={saving}>
          {saving ? "저장 중…" : "저장"}
        </Button>
      </div>
    </div>
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
