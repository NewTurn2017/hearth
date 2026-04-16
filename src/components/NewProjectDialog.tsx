import { useState, type KeyboardEvent } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { useToast } from "../ui/Toast";
import { PRIORITIES, CATEGORIES } from "../types";
import type { Priority, Category } from "../types";
import * as api from "../api";

const SELECT_CLASS =
  "h-9 flex-1 px-2 rounded-[var(--radius-md)] text-[13px] " +
  "bg-[var(--color-surface-2)] border border-[var(--color-border)] " +
  "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]";

export function NewProjectDialog({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const toast = useToast();
  const [name, setName] = useState("");
  const [priority, setPriority] = useState<Priority>("P2");
  const [category, setCategory] = useState<Category | "">("");
  const [path, setPath] = useState("");
  const [saving, setSaving] = useState(false);

  const reset = () => {
    setName("");
    setPriority("P2");
    setCategory("");
    setPath("");
  };

  const submit = async () => {
    const trimmed = name.trim();
    if (!trimmed || saving) return;
    setSaving(true);
    try {
      const created = await api.createProject(
        trimmed,
        priority,
        category || undefined,
        path.trim() || undefined
      );
      window.dispatchEvent(new CustomEvent("projects:changed"));
      toast.success(`${created.name} 추가됨`, {
        undo: async () => {
          await api.deleteProject(created.id);
          window.dispatchEvent(new CustomEvent("projects:changed"));
        },
      });
      reset();
      onClose();
    } catch (e) {
      toast.error(`생성 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const cancel = () => {
    reset();
    onClose();
  };

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  };

  return (
    <Dialog open={open} onClose={cancel} labelledBy="new-project-title">
      <h2
        id="new-project-title"
        className="text-heading text-[var(--color-text-hi)] mb-4"
      >
        새 프로젝트
      </h2>
      <div className="flex flex-col gap-3">
        <Input
          autoFocus
          placeholder="프로젝트 이름"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={onKey}
        />
        <div className="flex gap-2">
          <select
            className={SELECT_CLASS}
            value={priority}
            onChange={(e) => setPriority(e.target.value as Priority)}
          >
            {PRIORITIES.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
          <select
            className={SELECT_CLASS}
            value={category}
            onChange={(e) => setCategory(e.target.value as Category | "")}
          >
            <option value="">카테고리 없음</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </div>
        <Input
          placeholder="경로 (선택)"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          onKeyDown={onKey}
        />
      </div>
      <div className="flex justify-end gap-2 mt-5">
        <Button variant="secondary" onClick={cancel} disabled={saving}>
          취소
        </Button>
        <Button
          variant="primary"
          onClick={submit}
          disabled={saving || !name.trim()}
        >
          생성
        </Button>
      </div>
    </Dialog>
  );
}
