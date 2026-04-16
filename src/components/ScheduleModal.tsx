import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { Schedule } from "../types";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

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

  const isEdit = !!schedule;

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

  return (
    <Dialog open onClose={onClose} labelledBy="schedule-title">
      <form onSubmit={handleSubmit}>
        <h2
          id="schedule-title"
          className="text-heading text-[var(--color-text-hi)] mb-4"
        >
          일정 {isEdit ? "수정" : "추가"}
        </h2>

        <div className="flex flex-col gap-3">
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              날짜
            </label>
            <Input
              type="date"
              value={date}
              onChange={(e) => setDate(e.target.value)}
              required
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              시간
            </label>
            <Input
              type="text"
              value={time}
              onChange={(e) => setTime(e.target.value)}
              placeholder="예: 15:00"
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              장소
            </label>
            <Input
              type="text"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              내용
            </label>
            <Input
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              비고
            </label>
            <Input
              type="text"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
            />
          </div>
        </div>

        <div className="flex justify-between mt-5">
          <div>
            {isEdit && onDelete && (
              <Button
                type="button"
                variant="danger"
                size="sm"
                leftIcon={Trash2}
                onClick={onDelete}
              >
                삭제
              </Button>
            )}
          </div>
          <div className="flex gap-2">
            <Button type="button" variant="secondary" onClick={onClose}>
              취소
            </Button>
            <Button type="submit" variant="primary">
              저장
            </Button>
          </div>
        </div>
      </form>
    </Dialog>
  );
}
