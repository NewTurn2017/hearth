import {
  Bold,
  FolderInput,
  Pencil,
  Palette,
  Tags,
  Trash2,
  Type,
} from "lucide-react";
import type { Memo, MemoFontSize } from "../types";
import type { MemoUpdateInput } from "../api";
import { MEMO_COLORS } from "../types";
import { cn } from "../lib/cn";
import type { ContextMenuItem } from "../ui/ContextMenu";

const FONT_SIZE_LABELS: Record<MemoFontSize, string> = {
  small: "작게",
  normal: "보통",
  large: "크게",
};

export function memoFontSizeClass(size: MemoFontSize) {
  if (size === "small") return "text-[12px]";
  if (size === "large") return "text-[16px] leading-relaxed";
  return "text-sm";
}

export function memoTagNames(memo: Pick<Memo, "tags">) {
  return memo.tags.map((tag) => tag.name);
}

export function buildMemoActionItems({
  memo,
  onEdit,
  onUpdate,
  onDelete,
  onOpenProjectPicker,
  onOpenTagPicker,
  onCloseMenu,
}: {
  memo: Memo;
  onEdit: () => void;
  onUpdate: (id: number, fields: MemoUpdateInput) => void | Promise<unknown>;
  onDelete: (id: number) => void;
  onOpenProjectPicker: () => void;
  onOpenTagPicker: () => void;
  onCloseMenu: () => void;
}): ContextMenuItem[] {
  return [
    {
      id: "edit",
      label: "편집",
      icon: Pencil,
      onSelect: onEdit,
    },
    {
      id: "color",
      label: "색상 변경",
      icon: Palette,
      onSelect: () => {},
      inline: (
        <div className="flex gap-1">
          {MEMO_COLORS.map((color) => (
            <button
              key={color.name}
              type="button"
              aria-label={`색상: ${color.name}`}
              onClick={() => {
                onUpdate(memo.id, { color: color.name });
                onCloseMenu();
              }}
              className={cn(
                "w-5 h-5 rounded-full border",
                color.name === memo.color
                  ? "border-[var(--color-brand-hi)] ring-1 ring-[var(--color-brand-hi)]"
                  : "border-[var(--color-border)]",
              )}
              style={{ backgroundColor: color.bg }}
            />
          ))}
        </div>
      ),
    },
    {
      id: "move",
      label: "프로젝트 이동",
      icon: FolderInput,
      onSelect: onOpenProjectPicker,
    },
    {
      id: "font-size",
      label: "글씨 크기",
      icon: Type,
      onSelect: () => {},
      inline: (
        <div className="grid grid-cols-3 gap-1">
          {(Object.keys(FONT_SIZE_LABELS) as MemoFontSize[]).map((size) => (
            <button
              key={size}
              type="button"
              onClick={() => {
                onUpdate(memo.id, { font_size: size });
                onCloseMenu();
              }}
              className={cn(
                "h-6 rounded border px-1 text-[11px] transition-colors",
                memo.font_size === size
                  ? "border-[var(--color-brand-hi)] bg-[var(--color-surface-3)] text-[var(--color-text-hi)]"
                  : "border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text)]",
              )}
            >
              {FONT_SIZE_LABELS[size]}
            </button>
          ))}
        </div>
      ),
    },
    {
      id: "bold",
      label: memo.is_bold ? "굵게 해제" : "굵게 표시",
      icon: Bold,
      onSelect: () => onUpdate(memo.id, { is_bold: !memo.is_bold }),
    },
    {
      id: "tags",
      label: "태그 편집",
      icon: Tags,
      onSelect: onOpenTagPicker,
    },
    { id: "sep-delete", label: "", separator: true, onSelect: () => {} },
    {
      id: "delete",
      label: "삭제",
      icon: Trash2,
      danger: true,
      onSelect: () => onDelete(memo.id),
    },
  ];
}
