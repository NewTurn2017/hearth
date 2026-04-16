import { useState } from "react";
import type { Schedule } from "../types";

export function ScheduleModal({
  schedule,
  initialDate,
  onSave,
  onDelete,
  onClose,
}: {
  schedule?: Schedule;
  initialDate?: string;
  onSave: (data: {
    date: string;
    time?: string;
    location?: string;
    description?: string;
    notes?: string;
  }) => void;
  onDelete?: () => void;
  onClose: () => void;
}) {
  const [date, setDate] = useState(schedule?.date ?? initialDate ?? "");
  const [time, setTime] = useState(schedule?.time ?? "");
  const [location, setLocation] = useState(schedule?.location ?? "");
  const [description, setDescription] = useState(schedule?.description ?? "");
  const [notes, setNotes] = useState(schedule?.notes ?? "");

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!date) return;
    onSave({
      date,
      time: time || undefined,
      location: location || undefined,
      description: description || undefined,
      notes: notes || undefined,
    });
  };

  const inputClass =
    "w-full px-2 py-1.5 rounded bg-[var(--bg-primary)] border border-[var(--border-color)] text-sm text-[var(--text-primary)] outline-none focus:border-[var(--accent)]";

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <form
        onSubmit={handleSubmit}
        className="bg-[var(--bg-secondary)] rounded-xl p-5 w-96 shadow-2xl"
      >
        <h3 className="text-lg font-semibold mb-4">
          {schedule ? "일정 수정" : "새 일정"}
        </h3>

        <div className="flex flex-col gap-3">
          <div>
            <label className="text-xs text-[var(--text-secondary)] mb-1 block">
              날짜
            </label>
            <input
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              className={inputClass}
              required
            />
          </div>
          <div>
            <label className="text-xs text-[var(--text-secondary)] mb-1 block">
              시간
            </label>
            <input
              type="text"
              value={time}
              onChange={(e) => setTime(e.target.value)}
              placeholder="예: 15:00"
              className={inputClass}
            />
          </div>
          <div>
            <label className="text-xs text-[var(--text-secondary)] mb-1 block">
              장소
            </label>
            <input
              type="text"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              className={inputClass}
            />
          </div>
          <div>
            <label className="text-xs text-[var(--text-secondary)] mb-1 block">
              내용
            </label>
            <input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className={inputClass}
            />
          </div>
          <div>
            <label className="text-xs text-[var(--text-secondary)] mb-1 block">
              비고
            </label>
            <input
              type="text"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              className={inputClass}
            />
          </div>
        </div>

        <div className="flex justify-between mt-5">
          <div>
            {schedule && onDelete && (
              <button
                type="button"
                onClick={onDelete}
                className="text-sm text-red-400 hover:text-red-300"
              >
                삭제
              </button>
            )}
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-sm rounded bg-[var(--bg-tertiary)] hover:bg-[var(--border-color)] transition-colors"
            >
              취소
            </button>
            <button
              type="submit"
              className="px-3 py-1.5 text-sm rounded bg-[var(--accent)] text-white hover:opacity-90 transition-opacity"
            >
              저장
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
