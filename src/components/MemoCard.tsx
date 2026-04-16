import { useState } from "react";
import { useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, X } from "lucide-react";
import type { Memo, Project } from "../types";
import { MEMO_COLORS } from "../types";
import { Icon } from "../ui/Icon";
import { Tooltip } from "../ui/Tooltip";

export function MemoCard({
  memo,
  projects,
  onUpdate,
  onDelete,
}: {
  memo: Memo;
  projects: Project[];
  onUpdate: (id: number, fields: Record<string, unknown>) => void;
  onDelete: (id: number) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [content, setContent] = useState(memo.content);
  const [showColors, setShowColors] = useState(false);

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

  return (
    <div
      ref={setNodeRef}
      style={{
        ...style,
        backgroundColor: colorDef.bg,
        color: colorDef.text,
      }}
      className="memo-card rounded-xl p-4 hover:shadow-xl transition-shadow relative group min-h-[140px] flex flex-col"
    >
      <Tooltip label="드래그하여 이동" side="top">
        <div
          {...attributes}
          {...listeners}
          className="absolute top-1 right-1 cursor-grab opacity-0 group-hover:opacity-60"
        >
          <Icon icon={GripVertical} size={14} />
        </div>
      </Tooltip>

      <div className="absolute top-1 left-1">
        <button
          onClick={() => setShowColors(!showColors)}
          className="w-4 h-4 rounded-full opacity-0 group-hover:opacity-60 transition-opacity"
          style={{ backgroundColor: colorDef.text + "40" }}
        />
        {showColors && (
          <div className="absolute top-5 left-0 flex gap-1 bg-white rounded-lg p-1 shadow-lg z-10">
            {MEMO_COLORS.map((c) => (
              <button
                key={c.name}
                onClick={() => {
                  onUpdate(memo.id, { color: c.name });
                  setShowColors(false);
                }}
                className="w-5 h-5 rounded-full border border-gray-200"
                style={{ backgroundColor: c.bg }}
              />
            ))}
          </div>
        )}
      </div>

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
    </div>
  );
}
