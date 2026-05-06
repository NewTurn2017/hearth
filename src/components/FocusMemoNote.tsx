import { useState } from "react";
import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical } from "lucide-react";
import type { Memo, MemoTag, Project } from "../types";
import type { MemoUpdateInput } from "../api";
import { MEMO_COLORS } from "../types";
import { cn } from "../lib/cn";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu } from "../ui/ContextMenu";
import { Icon } from "../ui/Icon";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";
import { MemoTagPickerDialog } from "./MemoTagPickerDialog";
import { buildMemoActionItems, memoFontSizeClass } from "./memoActions";

export function FocusMemoNote({
  memo,
  projects,
  tags,
  x,
  y,
  sequenceNumber,
  highlighted,
  onUpdate,
  onDelete,
  onCreateTag,
}: {
  memo: Memo;
  projects: Project[];
  tags: MemoTag[];
  x: number;
  y: number;
  sequenceNumber: number;
  highlighted?: boolean;
  onUpdate: (id: number, fields: MemoUpdateInput) => void | Promise<unknown>;
  onDelete: (id: number) => void;
  onCreateTag: (name: string) => Promise<MemoTag>;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(memo.content);
  const [projectPickerOpen, setProjectPickerOpen] = useState(false);
  const [tagPickerOpen, setTagPickerOpen] = useState(false);
  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const { attributes, listeners, setNodeRef, transform, isDragging } =
    useDraggable({ id: memo.id });

  const colorDef =
    MEMO_COLORS.find((color) => color.name === memo.color) ?? MEMO_COLORS[0];
  const linkedProject = projects.find(
    (project) => project.id === memo.project_id,
  );

  const commitEdit = () => {
    if (content.trim() !== memo.content) {
      void onUpdate(memo.id, { content: content.trim() });
    }
    setEditing(false);
  };

  const menuItems = buildMemoActionItems({
    memo,
    onEdit: () => setEditing(true),
    onUpdate,
    onDelete,
    onOpenProjectPicker: () => setProjectPickerOpen(true),
    onOpenTagPicker: () => setTagPickerOpen(true),
    onCloseMenu: closeMenu,
  });

  return (
    <article
      ref={setNodeRef}
      data-memo-id={memo.id}
      onContextMenu={openMenu}
      className={cn(
        "absolute w-[210px] min-h-[138px] rounded-xl border border-black/10 p-3 shadow-[var(--shadow-e2)]",
        "focus:outline-none transition-shadow group",
        isDragging && "z-20 shadow-[var(--shadow-e3)]",
        highlighted && "find-highlight",
      )}
      style={{
        left: `${x * 100}%`,
        top: `${y * 100}%`,
        transform: CSS.Translate.toString(transform),
        backgroundColor: colorDef.bg,
        color: colorDef.text,
      }}
    >
      <div className="mb-2 flex items-center gap-2 pr-5 text-[10px] font-semibold opacity-70">
        <span aria-label={`메모 번호 ${sequenceNumber}`}>
          #{sequenceNumber}
        </span>
        {linkedProject && (
          <span className="truncate">{linkedProject.name}</span>
        )}
      </div>

      <button
        type="button"
        {...attributes}
        {...listeners}
        aria-label="포커스 메모 이동"
        className="absolute right-2 top-2 cursor-grab rounded p-0.5 opacity-50 hover:bg-black/10 hover:opacity-80"
      >
        <Icon icon={GripVertical} size={14} />
      </button>

      {editing ? (
        <textarea
          autoFocus
          value={content}
          onChange={(event) => setContent(event.target.value)}
          onBlur={commitEdit}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              setContent(memo.content);
              setEditing(false);
            } else if (
              event.key === "Enter" &&
              (event.metaKey || event.ctrlKey)
            ) {
              commitEdit();
            }
          }}
          className={cn(
            "h-[72px] w-full resize-none bg-transparent outline-none",
            memoFontSizeClass(memo.font_size),
            memo.is_bold && "font-semibold",
          )}
          style={{ color: colorDef.text }}
        />
      ) : (
        <p
          role="button"
          tabIndex={0}
          onClick={() => setEditing(true)}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault();
              setEditing(true);
            }
          }}
          className={cn(
            "line-clamp-4 min-h-[72px] cursor-pointer whitespace-pre-wrap [overflow-wrap:anywhere] focus:outline-none focus-visible:ring-2 focus-visible:ring-black/30",
            memoFontSizeClass(memo.font_size),
            memo.is_bold && "font-semibold",
          )}
        >
          {memo.content || "클릭하여 메모 작성..."}
        </p>
      )}

      {memo.tags.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1">
          {memo.tags.map((tag) => (
            <span
              key={tag.id}
              className="rounded-full bg-black/10 px-1.5 py-0.5 text-[10px] font-medium"
            >
              #{tag.name}
            </span>
          ))}
        </div>
      )}

      <ContextMenu
        open={menu.open}
        x={menu.x}
        y={menu.y}
        items={menuItems}
        onClose={closeMenu}
      />
      <MemoProjectPickerDialog
        open={projectPickerOpen}
        projects={projects}
        currentProjectId={memo.project_id}
        onClose={() => setProjectPickerOpen(false)}
        onPick={(projectId) => {
          void onUpdate(memo.id, { project_id: projectId });
          setProjectPickerOpen(false);
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
    </article>
  );
}
