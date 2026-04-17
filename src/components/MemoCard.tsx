import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, X, Pencil, Palette, FolderInput, Trash2 } from "lucide-react";
import type { Memo, Project } from "../types";
import { MEMO_COLORS } from "../types";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";
import { cn } from "../lib/cn";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu, type ContextMenuItem } from "../ui/ContextMenu";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";

export function MemoCard({
  memo,
  projects,
  onUpdate,
  onDelete,
  sequenceNumber,
}: {
  memo: Memo;
  projects: Project[];
  onUpdate: (id: number, fields: Record<string, unknown>) => void;
  onDelete: (id: number) => void;
  sequenceNumber: number;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(memo.content);
  const { menu, open: openMenu, close: closeMenu } = useContextMenu();
  const [pickerOpen, setPickerOpen] = useState(false);

  const colorDef = MEMO_COLORS.find((c) => c.name === memo.color) ?? MEMO_COLORS[0];

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
                "w-6 h-6 rounded-full border",
                c.name === memo.color
                  ? "border-[var(--color-brand-hi)]"
                  : "border-[var(--color-border)]"
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

  return (
    <div
      ref={setNodeRef}
      style={{
        ...style,
        backgroundColor: colorDef.bg,
        color: colorDef.text,
      }}
      className="memo-card rounded-xl p-5 hover:shadow-xl transition-shadow relative group min-h-[160px] flex flex-col"
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
            className="w-full h-full min-h-[80px] bg-transparent outline-none text-sm resize-none"
            style={{ color: colorDef.text }}
          />
        ) : (
          <p className="text-sm whitespace-pre-wrap cursor-pointer">
            {memo.content || "클릭하여 메모 작성..."}
          </p>
        )}
      </div>

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
          // null detaches — backend's Option<Option<i64>> shape serializes null
          // explicitly as Some(None). Passing undefined would leave the field
          // out of the payload entirely, so we always pass the key.
          onUpdate(memo.id, { project_id: projectId });
          setPickerOpen(false);
        }}
      />
    </div>
  );
}
