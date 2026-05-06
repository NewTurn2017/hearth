import { useEffect, useMemo, useState } from "react";
import type { Memo, MemoTag } from "../types";
import { cn } from "../lib/cn";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { Input } from "../ui/Input";
import { useToast } from "../ui/Toast";

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function MemoTagPickerDialog({
  open,
  memo,
  tags,
  onClose,
  onApply,
  onCreateTag,
}: {
  open: boolean;
  memo: Memo | null;
  tags: MemoTag[];
  onClose: () => void;
  onApply: (tagNames: string[]) => void | Promise<void>;
  onCreateTag: (name: string) => Promise<MemoTag>;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [newName, setNewName] = useState("");
  const [creating, setCreating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();

  useEffect(() => {
    if (!open || !memo) return;
    setSelected(new Set(memo.tags.map((tag) => tag.name)));
    setNewName("");
    setError(null);
    setCreating(false);
    setSaving(false);
  }, [memo, open]);

  const sortedTags = useMemo(
    () =>
      [...tags].sort(
        (a, b) => a.sort_order - b.sort_order || a.name.localeCompare(b.name),
      ),
    [tags],
  );

  const toggle = (name: string) => {
    setError(null);
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const createTag = async () => {
    const name = newName.trim();
    if (!name || creating || saving) return;
    setCreating(true);
    setError(null);
    try {
      const created = await onCreateTag(name);
      setSelected((prev) => new Set(prev).add(created.name));
      setNewName("");
    } catch (err) {
      const message = `태그 생성 실패: ${errorMessage(err)}`;
      setError(message);
      toast.error(message);
    } finally {
      setCreating(false);
    }
  };

  const apply = async () => {
    if (!memo || saving || creating) return;
    setSaving(true);
    setError(null);
    try {
      await onApply([...selected]);
      onClose();
    } catch (err) {
      const message = `태그 저장 실패: ${errorMessage(err)}`;
      setError(message);
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      open={open}
      onClose={saving || creating ? () => {} : onClose}
      labelledBy="memo-tag-picker-title"
    >
      <div className="space-y-4">
        <div>
          <h3
            id="memo-tag-picker-title"
            className="text-[15px] font-semibold text-[var(--color-text-hi)]"
          >
            메모 태그
          </h3>
          <p className="mt-1 text-[12px] text-[var(--color-text-muted)]">
            이 메모에 붙일 태그를 선택하세요.
          </p>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {sortedTags.length === 0 ? (
            <span className="text-[12px] text-[var(--color-text-dim)]">
              아직 태그가 없습니다.
            </span>
          ) : (
            sortedTags.map((tag) => {
              const active = selected.has(tag.name);
              return (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => toggle(tag.name)}
                  aria-pressed={active}
                  disabled={saving || creating}
                  className={cn(
                    "rounded-full border px-2 py-1 text-[12px] transition-colors disabled:opacity-60",
                    active
                      ? "border-[var(--color-brand-hi)] bg-[var(--color-surface-3)] text-[var(--color-text-hi)]"
                      : "border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text)]",
                  )}
                >
                  #{tag.name}
                </button>
              );
            })
          )}
        </div>

        <div className="flex gap-2">
          <Input
            value={newName}
            onChange={(event) => {
              setError(null);
              setNewName(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void createTag();
              }
            }}
            disabled={saving || creating}
            placeholder="새 태그 이름"
          />
          <Button
            type="button"
            size="sm"
            onClick={() => void createTag()}
            disabled={!newName.trim() || creating || saving}
          >
            {creating ? "추가 중" : "추가"}
          </Button>
        </div>

        {error && (
          <p role="alert" className="text-[12px] text-[var(--color-danger)]">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2">
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={onClose}
            disabled={saving || creating}
          >
            취소
          </Button>
          <Button
            type="button"
            size="sm"
            variant="primary"
            onClick={() => void apply()}
            disabled={!memo || saving || creating}
          >
            {saving ? "저장 중" : "적용"}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
