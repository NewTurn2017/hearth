import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, X } from "lucide-react";
import type { Memo, MemoTag, Project } from "../types";
import type { MemoUpdateInput } from "../api";
import { MEMO_COLORS } from "../types";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
import { cn } from "../lib/cn";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu } from "../ui/ContextMenu";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";
import { MemoTagPickerDialog } from "./MemoTagPickerDialog";
import { buildMemoActionItems, memoFontSizeClass } from "./memoActions";

export function MemoCard({
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

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: memo.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const linkedProject = projects.find((p) => p.id === memo.project_id);

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

  return (
    <div
      ref={setNodeRef}
      data-memo-id={memo.id}
      style={{
        ...style,
        backgroundColor: colorDef.bg,
        color: colorDef.text,
      }}
      className={cn(
        "memo-card rounded-xl p-5 hover:shadow-xl transition-shadow relative group min-h-[160px] flex flex-col",
        highlighted && "find-highlight",
      )}
      onContextMenu={openMenu}
    >
      <span
        className="absolute top-1.5 right-2 rounded-full bg-black/25 text-white px-1.5 py-[1px] text-[10px] font-semibold leading-none"
        aria-label={`메모 번호 ${sequenceNumber}`}
      >
        #{sequenceNumber}
      </span>

      <Tooltip label="드래그하여 이동" side="top">
        <div
          {...attributes}
          {...listeners}
          className="absolute top-6 right-2 cursor-grab opacity-0 group-hover:opacity-60"
        >
          <Icon icon={GripVertical} size={14} />
        </div>
      </Tooltip>

      <div className="flex-1 mt-3" onClick={() => setEditing(true)}>
        {editing ? (
          <textarea
            autoFocus
            value={content}
            onChange={(e) => setContent(e.target.value)}
            onBlur={commitEdit}
            className={cn(
              "w-full h-full min-h-[80px] bg-transparent outline-none resize-none",
              memoFontSizeClass(memo.font_size),
              memo.is_bold && "font-semibold",
            )}
            style={{ color: colorDef.text }}
          />
        ) : (
          <p
            className={cn(
              "whitespace-pre-wrap [overflow-wrap:anywhere] cursor-pointer",
              memoFontSizeClass(memo.font_size),
              memo.is_bold && "font-semibold",
            )}
          >
            {memo.content || "클릭하여 메모 작성..."}
          </p>
        )}
      </div>

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

      <div className="flex justify-between items-center mt-2 text-xs opacity-60">
        <span>{linkedProject?.name ?? ""}</span>
        <Tooltip label="삭제" side="top">
          <button
            onClick={() => onDelete(memo.id)}
            className="opacity-0 group-hover:opacity-100 hover:text-red-600 transition-opacity"
            aria-label="삭제"
          >
            <Icon icon={X} size={14} />
          </button>
        </Tooltip>
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
