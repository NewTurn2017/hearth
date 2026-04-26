import { useState } from "react";
import { Pencil, Palette, FolderInput, Trash2 } from "lucide-react";
import type { Memo, Project } from "../types";
import { MEMO_COLORS } from "../types";
import { cn } from "../lib/cn";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu, type ContextMenuItem } from "../ui/ContextMenu";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";

export function MemoRow({
  memo,
  projects,
  onUpdate,
  onDelete,
  sequenceNumber,
  highlighted,
}: {
  memo: Memo;
  projects: Project[];
  onUpdate: (id: number, fields: Record<string, unknown>) => void;
  onDelete: (id: number) => void;
  sequenceNumber: number;
  highlighted?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(memo.content);
  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const [pickerOpen, setPickerOpen] = useState(false);

  const colorDef =
    MEMO_COLORS.find((c) => c.name === memo.color) ?? MEMO_COLORS[0];

  const commitEdit = () => {
    if (content.trim() !== memo.content) {
      onUpdate(memo.id, { content: content.trim() });
    }
    setEditing(false);
  };

  const menuItems: ContextMenuItem[] = [
    {
      id: "edit",
      label: "편집",
      icon: Pencil,
      onSelect: () => setEditing(true),
    },
    {
      id: "color",
      label: "색상 변경",
      icon: Palette,
      onSelect: () => {},
      inline: (
        <div className="flex gap-1">
          {MEMO_COLORS.map((c) => (
            <button
              key={c.name}
              type="button"
              aria-label={`색상: ${c.name}`}
              onClick={() => {
                onUpdate(memo.id, { color: c.name });
                closeMenu();
              }}
              className={cn(
                "w-5 h-5 rounded-full border",
                c.name === memo.color
                  ? "border-[var(--color-brand-hi)]"
                  : "border-[var(--color-border)]",
              )}
              style={{ backgroundColor: c.bg }}
            />
          ))}
        </div>
      ),
    },
    {
      id: "move",
      label: "프로젝트 이동",
      icon: FolderInput,
      onSelect: () => setPickerOpen(true),
    },
    { id: "sep", label: "", separator: true, onSelect: () => {} },
    {
      id: "delete",
      label: "삭제",
      icon: Trash2,
      danger: true,
      onSelect: () => onDelete(memo.id),
    },
  ];

  const preview = memo.content || "(비어 있음)";

  return (
    <div
      data-memo-id={memo.id}
      onContextMenu={openMenu}
      className={cn(
        "group flex items-start gap-2 rounded-md px-2 py-1.5 text-[12.5px] cursor-pointer hover:bg-[var(--color-surface-2)]",
        highlighted && "find-highlight",
      )}
    >
      <span
        className="mt-[2px] shrink-0 inline-flex items-center justify-center min-w-[22px] h-[16px] rounded-sm px-1 text-[10px] font-semibold leading-none"
        style={{ backgroundColor: colorDef.bg, color: colorDef.text }}
        aria-label={`메모 번호 ${sequenceNumber}`}
      >
        #{sequenceNumber}
      </span>
      <div
        className="flex-1 min-w-0"
        onClick={() => !editing && setEditing(true)}
      >
        {editing ? (
          <textarea
            autoFocus
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onBlur={commitEdit}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setContent(memo.content);
                setEditing(false);
              } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
                commitEdit();
              }
            }}
            className="w-full bg-[var(--color-surface-1)] border border-[var(--color-border)] rounded px-2 py-1 outline-none text-[12.5px] resize-y min-h-[60px] text-[var(--color-text-hi)]"
          />
        ) : (
          <p className="line-clamp-2 whitespace-pre-wrap break-words leading-snug text-[var(--color-text)]">
            {preview}
          </p>
        )}
      </div>
      <ContextMenu
        open={menu.open}
        x={menu.x}
        y={menu.y}
        items={menuItems}
        onClose={closeMenu}
      />
      <MemoProjectPickerDialog
        open={pickerOpen}
        projects={projects}
        currentProjectId={memo.project_id}
        onClose={() => setPickerOpen(false)}
        onPick={(projectId) => {
          onUpdate(memo.id, { project_id: projectId });
          setPickerOpen(false);
        }}
      />
    </div>
  );
}
