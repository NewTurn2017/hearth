import type { KeyboardEvent } from "react";
import { Input } from "../ui/Input";
import { PRIORITIES } from "../types";
import type { Priority } from "../types";
import { useCategories } from "../hooks/useCategories";

export type ProjectFormState = {
  name: string;
  priority: Priority;
  category: string; // free-form — empty = 없음
  path: string;
  evaluation: string;
};

export const emptyProjectForm = (): ProjectFormState => ({
  name: "",
  priority: "P2",
  category: "",
  path: "",
  evaluation: "",
});

const SELECT_CLASS =
  "h-9 flex-1 px-2 rounded-[var(--radius-md)] text-[13px] " +
  "bg-[var(--color-surface-2)] border border-[var(--color-border)] " +
  "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]";

export function ProjectFormFields({
  value,
  onChange,
  onSubmitShortcut,
  includeEvaluation = true,
  disableName = false,
  autoFocusName = false,
}: {
  value: ProjectFormState;
  onChange: (patch: Partial<ProjectFormState>) => void;
  onSubmitShortcut?: () => void;
  includeEvaluation?: boolean;
  disableName?: boolean;
  autoFocusName?: boolean;
}) {
  const { categories } = useCategories();

  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (!onSubmitShortcut) return;
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      onSubmitShortcut();
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <Input
        autoFocus={autoFocusName}
        placeholder="프로젝트 이름"
        value={value.name}
        disabled={disableName}
        onChange={(e) => onChange({ name: e.target.value })}
        onKeyDown={onKey}
      />
      <div className="flex gap-2">
        <select
          className={SELECT_CLASS}
          value={value.priority}
          onChange={(e) => onChange({ priority: e.target.value as Priority })}
        >
          {PRIORITIES.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <select
          className={SELECT_CLASS}
          value={value.category}
          onChange={(e) => onChange({ category: e.target.value })}
        >
          <option value="">카테고리 없음</option>
          {categories.map((c) => (
            <option key={c.id} value={c.name}>
              {c.name}
            </option>
          ))}
        </select>
      </div>
      <Input
        placeholder="경로 (선택)"
        value={value.path}
        onChange={(e) => onChange({ path: e.target.value })}
        onKeyDown={onKey}
      />
      {includeEvaluation && (
        <textarea
          className={
            "min-h-[96px] w-full px-2 py-1.5 rounded-[var(--radius-md)] text-[13px] " +
            "bg-[var(--color-surface-2)] border border-[var(--color-border)] " +
            "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
          }
          placeholder="평가, 메모, 진행 상황…"
          value={value.evaluation}
          onChange={(e) => onChange({ evaluation: e.target.value })}
        />
      )}
    </div>
  );
}
