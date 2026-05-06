import { useState } from "react";
import type { Memo, MemoTag, Project } from "../types";
import type { MemoUpdateInput } from "../api";
import { MEMO_COLORS } from "../types";
import { cn } from "../lib/cn";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu } from "../ui/ContextMenu";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";
import { MemoTagPickerDialog } from "./MemoTagPickerDialog";
import { buildMemoActionItems, memoFontSizeClass } from "./memoActions";

export function MemoRow({
  memo,
  projects,
  tags,
  onUpdate,
  onDelete,
  onCreateTag,
  sequenceNumber,
  highlighted,
}: {
  memo: Memo;
  projects: Project[];
  tags: MemoTag[];
  onUpdate: (id: number, fields: MemoUpdateInput) => void | Promise<unknown>;
  onDelete: (id: number) => void;
  onCreateTag: (name: string) => Promise<MemoTag>;
  sequenceNumber: number;
  highlighted?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(memo.content);
  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [tagPickerOpen, setTagPickerOpen] = useState(false);

  const colorDef =
    MEMO_COLORS.find((c) => c.name === memo.color) ?? MEMO_COLORS[0];

  const commitEdit = () => {
    if (content.trim() !== memo.content) {
      onUpdate(memo.id, { content: content.trim() });
    }
    setEditing(false);
  };

  const menuItems = buildMemoActionItems({
    memo,
    onEdit: () => setEditing(true),
    onUpdate,
    onDelete,
    onOpenProjectPicker: () => setPickerOpen(true),
    onOpenTagPicker: () => setTagPickerOpen(true),
    onCloseMenu: closeMenu,
  });

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
            className={cn(
              "w-full bg-[var(--color-surface-1)] border border-[var(--color-border)] rounded px-2 py-1 outline-none resize-y min-h-[60px] text-[var(--color-text-hi)]",
              memoFontSizeClass(memo.font_size),
              memo.is_bold && "font-semibold",
            )}
          />
        ) : (
          <div className="space-y-1">
            <p
              className={cn(
                "line-clamp-2 whitespace-pre-wrap [overflow-wrap:anywhere] leading-snug text-[var(--color-text)]",
                memoFontSizeClass(memo.font_size),
                memo.is_bold && "font-semibold",
              )}
            >
              {preview}
            </p>
            {memo.tags.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {memo.tags.map((tag) => (
                  <span
                    key={tag.id}
                    className="rounded-full bg-[var(--color-surface-2)] px-1.5 py-0.5 text-[10px] text-[var(--color-text-muted)]"
                  >
                    #{tag.name}
                  </span>
                ))}
              </div>
            )}
          </div>
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
      <MemoTagPickerDialog
        open={tagPickerOpen}
        memo={memo}
        tags={tags}
        onClose={() => setTagPickerOpen(false)}
        onCreateTag={onCreateTag}
        onApply={async (tagNames) => {
          await onUpdate(memo.id, { tag_names: tagNames });
        }}
      />
    </div>
  );
}
