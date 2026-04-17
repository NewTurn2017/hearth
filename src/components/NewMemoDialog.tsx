import { useEffect, useMemo, useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useToast } from "../ui/Toast";
import { useProjects } from "../hooks/useProjects";
import { PRIORITIES, MEMO_COLORS } from "../types";
import type { Priority } from "../types";
import { cn } from "../lib/cn";
import * as api from "../api";

const ALL_PRIORITIES = new Set<Priority>(PRIORITIES);

export function NewMemoDialog({
  open,
  onClose,
  defaultProjectId = null,
}: {
  open: boolean;
  onClose: () => void;
  /** Pre-selects the project dropdown. Null means "프로젝트 없음". */
  defaultProjectId?: number | null;
}) {
  const toast = useToast();
  const { projects } = useProjects(ALL_PRIORITIES, null);

  const [content, setContent] = useState("");
  const [projectId, setProjectId] = useState<number | null>(defaultProjectId);
  const [color, setColor] = useState(MEMO_COLORS[0].name);
  const [saving, setSaving] = useState(false);

  // Reset form state every time the dialog reopens so stale input from a
  // prior cancelled attempt never carries over.
  useEffect(() => {
    if (!open) return;
    setContent("");
    setProjectId(defaultProjectId);
    setColor(MEMO_COLORS[0].name);
  }, [open, defaultProjectId]);

  const grouped = useMemo(() => {
    const map = new Map<Priority, typeof projects>();
    for (const p of PRIORITIES) map.set(p, []);
    for (const p of projects) {
      if ((PRIORITIES as readonly string[]).includes(p.priority)) {
        map.get(p.priority as Priority)!.push(p);
      }
    }
    return map;
  }, [projects]);

  const canSubmit = content.trim().length > 0 && !saving;

  const submit = async () => {
    if (!canSubmit) return;
    setSaving(true);
    try {
      await api.createMemo({
        content: content.trim(),
        color,
        project_id: projectId ?? undefined,
      });
      window.dispatchEvent(new CustomEvent("memos:changed"));
      toast.success("메모 추가됨");
      onClose();
    } catch (e) {
      toast.error(`메모 추가 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const cancel = () => {
    onClose();
  };

  return (
    <Dialog open={open} onClose={cancel} labelledBy="new-memo-title">
      <h2
        id="new-memo-title"
        className="text-heading text-[var(--color-text-hi)] mb-4"
      >
        새 메모
      </h2>
      <div className="flex flex-col gap-4">
        <textarea
          autoFocus
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="메모 내용…"
          className={cn(
            "min-h-[120px] w-full px-2 py-1.5 rounded-[var(--radius-md)] text-[13px]",
            "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
            "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
          )}
        />

        <div>
          <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
            프로젝트
          </label>
          <select
            value={projectId ?? ""}
            onChange={(e) =>
              setProjectId(e.target.value === "" ? null : Number(e.target.value))
            }
            className={cn(
              "h-9 w-full px-2 rounded-[var(--radius-md)] text-[13px]",
              "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
              "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
            )}
          >
            <option value="">프로젝트 없음 (기타)</option>
            {PRIORITIES.map((pri) => {
              const items = grouped.get(pri) ?? [];
              if (items.length === 0) return null;
              return (
                <optgroup key={pri} label={`${pri}`}>
                  {items.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </optgroup>
              );
            })}
          </select>
        </div>

        <div>
          <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
            색상
          </label>
          <div className="flex gap-2">
            {MEMO_COLORS.map((c) => (
              <button
                key={c.name}
                type="button"
                aria-label={`색상: ${c.name}`}
                onClick={() => setColor(c.name)}
                className={cn(
                  "w-8 h-8 rounded-full border-2 transition-colors",
                  color === c.name
                    ? "border-[var(--color-brand-hi)]"
                    : "border-transparent"
                )}
                style={{ backgroundColor: c.bg }}
              />
            ))}
          </div>
        </div>
      </div>

      <div className="flex justify-end gap-2 mt-5">
        <Button variant="secondary" onClick={cancel} disabled={saving}>
          취소
        </Button>
        <Button variant="primary" onClick={submit} disabled={!canSubmit}>
          추가
        </Button>
      </div>
    </Dialog>
  );
}
