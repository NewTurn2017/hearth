// User-editable category list.
//
// Rename: blur commits via `useCategories.rename`, which fires a cascading
// UPDATE on `projects.category` under the hood + dispatches
// `projects:changed`. Delete refuses when the row has dependents — the
// backend's `delete_category` returns a Korean error, the ✕ button is
// disabled + tooltipped before we even send the request.

import { useState, useRef, useEffect } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Plus, X } from "lucide-react";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { Popover } from "../ui/Popover";
import { Tooltip } from "../ui/Tooltip";
import { useToast } from "../ui/Toast";
import { useCategories } from "../hooks/useCategories";
import type { CategoryRow } from "../types";
import { cn } from "../lib/cn";

const PRESET_COLORS = [
  "#22c55e",
  "#f97316",
  "#a855f7",
  "#6b7280",
  "#3b82f6",
  "#ef4444",
  "#eab308",
  "#14b8a6",
  "#ec4899",
  "#0ea5e9",
];

export function SettingsCategoriesSection() {
  const toast = useToast();
  const { categories, create, rename, recolor, remove, reorder } =
    useCategories();
  const [adding, setAdding] = useState(false);
  const [newName, setNewName] = useState("");

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = async (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over || active.id === over.id) return;
    const fromIdx = categories.findIndex((c) => c.id === Number(active.id));
    const toIdx = categories.findIndex((c) => c.id === Number(over.id));
    if (fromIdx < 0 || toIdx < 0) return;
    const next = [...categories];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    try {
      await reorder(next.map((c) => c.id));
    } catch (err) {
      toast.error(`순서 저장 실패: ${err}`);
    }
  };

  const commitAdd = async () => {
    const name = newName.trim();
    if (!name) {
      setAdding(false);
      setNewName("");
      return;
    }
    try {
      await create({ name });
      toast.success(`${name} 추가됨`);
    } catch (err) {
      toast.error(`추가 실패: ${err}`);
    } finally {
      setAdding(false);
      setNewName("");
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={categories.map((c) => c.id)}
          strategy={verticalListSortingStrategy}
        >
          <ul className="flex flex-col rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-surface-2)] divide-y divide-[var(--color-border)]">
            {categories.map((c) => (
              <CategoryRowItem
                key={c.id}
                category={c}
                onRename={(n) =>
                  rename(c.id, n).catch((e) => toast.error(`이름 변경 실패: ${e}`))
                }
                onRecolor={(col) =>
                  recolor(c.id, col).catch((e) =>
                    toast.error(`색 변경 실패: ${e}`)
                  )
                }
                onDelete={() =>
                  remove(c.id)
                    .then(() => toast.success(`${c.name} 삭제됨`))
                    .catch((e) => toast.error(`삭제 실패: ${e}`))
                }
              />
            ))}
          </ul>
        </SortableContext>
      </DndContext>

      {adding ? (
        <div className="flex gap-2">
          <input
            autoFocus
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onBlur={commitAdd}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitAdd();
              if (e.key === "Escape") {
                setAdding(false);
                setNewName("");
              }
            }}
            placeholder="카테고리 이름"
            className={cn(
              "flex-1 h-9 px-3 text-[13px] rounded-[var(--radius-md)]",
              "bg-[var(--color-surface-2)] text-[var(--color-text)]",
              "border border-[var(--color-border)]",
              "focus:outline-none focus:border-[var(--color-brand-hi)]"
            )}
          />
        </div>
      ) : (
        <Button
          variant="secondary"
          size="sm"
          leftIcon={Plus}
          onClick={() => setAdding(true)}
        >
          카테고리 추가
        </Button>
      )}
    </div>
  );
}

function CategoryRowItem({
  category,
  onRename,
  onRecolor,
  onDelete,
}: {
  category: CategoryRow;
  onRename: (name: string) => void;
  onRecolor: (color: string) => void;
  onDelete: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: category.id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const [draftName, setDraftName] = useState(category.name);
  const inputRef = useRef<HTMLInputElement>(null);

  // Keep local draft synced to upstream name changes (e.g. after a successful
  // rename) so an unrelated re-render doesn't overwrite in-progress input.
  useEffect(() => {
    setDraftName(category.name);
  }, [category.name]);

  const disableDelete = category.usage_count > 0;

  return (
    <li
      ref={setNodeRef}
      style={style}
      className="flex items-center gap-2 px-3 h-11"
    >
      <button
        {...attributes}
        {...listeners}
        type="button"
        aria-label="드래그하여 순서 변경"
        className="cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)]"
      >
        <Icon icon={GripVertical} size={14} />
      </button>

      <Popover
        trigger={({ onClick, "aria-expanded": ae }) => (
          <button
            type="button"
            onClick={onClick}
            aria-expanded={ae}
            aria-label="색상 변경"
            className="w-5 h-5 rounded-full border border-[var(--color-border)] shrink-0"
            style={{ backgroundColor: category.color }}
          />
        )}
      >
        {({ close }) => (
          <ColorPicker
            value={category.color}
            onChange={(c) => {
              onRecolor(c);
              close();
            }}
          />
        )}
      </Popover>

      <input
        ref={inputRef}
        value={draftName}
        onChange={(e) => setDraftName(e.target.value)}
        onBlur={() => {
          const next = draftName.trim();
          if (next && next !== category.name) onRename(next);
          else setDraftName(category.name);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") inputRef.current?.blur();
          if (e.key === "Escape") {
            setDraftName(category.name);
            inputRef.current?.blur();
          }
        }}
        className={cn(
          "flex-1 h-8 px-2 text-[13px] rounded-[var(--radius-sm)]",
          "bg-transparent border border-transparent hover:border-[var(--color-border)]",
          "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]",
          "focus:bg-[var(--color-surface-1)]"
        )}
      />

      <span className="text-[11px] text-[var(--color-text-dim)] tabular-nums shrink-0">
        {category.usage_count}개
      </span>

      <Tooltip
        label={disableDelete ? "사용 중인 카테고리는 삭제할 수 없습니다" : "삭제"}
        side="top"
      >
        <button
          type="button"
          onClick={() => !disableDelete && onDelete()}
          disabled={disableDelete}
          aria-label="삭제"
          className={cn(
            "w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)]",
            disableDelete
              ? "text-[var(--color-text-dim)] cursor-not-allowed"
              : "text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]"
          )}
        >
          <Icon icon={X} size={14} />
        </button>
      </Tooltip>
    </li>
  );
}

function ColorPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (c: string) => void;
}) {
  const [custom, setCustom] = useState(value);
  return (
    <div className="flex flex-col gap-2 p-2">
      <div className="grid grid-cols-5 gap-2">
        {PRESET_COLORS.map((c) => (
          <button
            key={c}
            type="button"
            onClick={() => onChange(c)}
            aria-label={`색상 ${c}`}
            className={cn(
              "w-6 h-6 rounded-full border transition-transform",
              c.toLowerCase() === value.toLowerCase()
                ? "border-[var(--color-brand-hi)] scale-110"
                : "border-[var(--color-border)]"
            )}
            style={{ backgroundColor: c }}
          />
        ))}
      </div>
      <div className="flex gap-1 items-center">
        <input
          value={custom}
          onChange={(e) => setCustom(e.target.value)}
          placeholder="#rrggbb"
          className={cn(
            "flex-1 h-7 px-2 text-[12px] font-mono rounded-[var(--radius-sm)]",
            "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
            "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
          )}
        />
        <button
          type="button"
          onClick={() => {
            if (/^#[0-9a-fA-F]{6}$/.test(custom)) onChange(custom);
          }}
          className="text-[11px] px-2 h-7 rounded-[var(--radius-sm)] text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-3)]"
        >
          적용
        </button>
      </div>
    </div>
  );
}
