import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { Schedule } from "../types";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";

type SaveData = {
  date: string;
  time?: string;
  location?: string;
  description?: string;
  notes?: string;
  remind_before_5min?: boolean;
  remind_at_start?: boolean;
};

function onEnterSubmit(e: React.KeyboardEvent<HTMLInputElement>) {
  const native = e.nativeEvent as KeyboardEvent & { keyCode?: number };
  if (
    e.key === "Enter" &&
    !native.isComposing &&
    native.keyCode !== 229 &&
    !e.shiftKey
  ) {
    e.preventDefault();
    e.currentTarget.form?.requestSubmit();
  }
}

export function ScheduleModal({
  schedule,
  initialDate,
  onSave,
  onDelete,
  onClose,
}: {
  schedule?: Schedule;
  initialDate?: string;
  onSave: (data: SaveData) => void;
  onDelete?: () => void;
  onClose: () => void;
}) {
  const initialNotify =
    !!schedule && (
      !!schedule.time ||
      schedule.remind_before_5min ||
      schedule.remind_at_start
    );

  const [date, setDate] = useState(schedule?.date ?? initialDate ?? "");
  const [notify, setNotify] = useState(initialNotify);
  const [time, setTime] = useState(
    schedule?.time ?? (initialNotify ? "09:00" : "")
  );
  const [remindBefore5, setRemindBefore5] = useState(
    schedule?.remind_before_5min ?? true
  );
  const [remindAtStart, setRemindAtStart] = useState(
    schedule?.remind_at_start ?? false
  );
  const [location, setLocation] = useState(schedule?.location ?? "");
  const [description, setDescription] = useState(schedule?.description ?? "");
  const [notes, setNotes] = useState(schedule?.notes ?? "");

  const isEdit = !!schedule;
  const timeMissing = notify && !time;

  function toggleNotify() {
    const next = !notify;
    setNotify(next);
    if (next && !time) setTime("09:00");
    if (next && !remindBefore5 && !remindAtStart) setRemindBefore5(true);
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!date || timeMissing) return;
    onSave({
      date,
      time: notify ? time : undefined,
      location: location || undefined,
      description: description || undefined,
      notes: notes || undefined,
      remind_before_5min: notify ? remindBefore5 : false,
      remind_at_start: notify ? remindAtStart : false,
    });
  }

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
              onKeyDown={onEnterSubmit}
              required
            />
          </div>

          <label className="flex items-center gap-2 text-[13px] select-none">
            <input
              type="checkbox"
              checked={notify}
              onChange={toggleNotify}
              aria-label="알림 받기"
            />
            <span>알림 받기</span>
          </label>

          {notify && (
            <>
              <div>
                <label
                  htmlFor="schedule-time"
                  className="text-[11px] text-[var(--color-text-muted)] mb-1 block"
                >
                  시간
                </label>
                <Input
                  id="schedule-time"
                  type="time"
                  value={time}
                  onChange={(e) => setTime(e.target.value)}
                  onKeyDown={onEnterSubmit}
                  aria-label="시간"
                />
              </div>
              <div className="flex gap-4 text-[13px]">
                <label className="flex items-center gap-1.5 select-none">
                  <input
                    type="checkbox"
                    checked={remindBefore5}
                    onChange={(e) => setRemindBefore5(e.target.checked)}
                    aria-label="5분 전"
                  />
                  <span>5분 전</span>
                </label>
                <label className="flex items-center gap-1.5 select-none">
                  <input
                    type="checkbox"
                    checked={remindAtStart}
                    onChange={(e) => setRemindAtStart(e.target.checked)}
                    aria-label="정각"
                  />
                  <span>정각</span>
                </label>
              </div>
            </>
          )}

          <div>
            <label className="text-[11px] text-[var(--color-text-muted)] mb-1 block">
              장소
            </label>
            <Input
              type="text"
              value={location}
              onChange={(e) => setLocation(e.target.value)}
              onKeyDown={onEnterSubmit}
              aria-label="장소"
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
              onKeyDown={onEnterSubmit}
              aria-label="내용"
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
              onKeyDown={onEnterSubmit}
              aria-label="비고"
            />
          </div>

          {timeMissing && (
            <div className="text-[11px] text-[var(--color-danger)]">
              시간을 입력해 주세요.
            </div>
          )}
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
            <Button type="submit" variant="primary" disabled={timeMissing}>
              저장
            </Button>
          </div>
        </div>
      </form>
    </Dialog>
  );
}
