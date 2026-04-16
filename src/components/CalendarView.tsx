import { useState, useMemo } from "react";
import { Calendar, momentLocalizer } from "react-big-calendar";
import moment from "moment";
import "moment/locale/ko";
import "react-big-calendar/lib/css/react-big-calendar.css";
import { CalendarDays, Plus } from "lucide-react";
import { ScheduleModal } from "./ScheduleModal";
import { useSchedules } from "../hooks/useSchedules";
import type { Schedule } from "../types";
import { Button } from "../ui/Button";

moment.locale("ko");
const localizer = momentLocalizer(moment);

interface CalendarEvent {
  id: number;
  title: string;
  start: Date;
  end: Date;
  resource: Schedule;
}

const messages = {
  today: "오늘",
  previous: "이전",
  next: "다음",
  month: "월",
  week: "주",
  day: "일",
  agenda: "일정",
  date: "날짜",
  time: "시간",
  event: "일정",
  noEventsInRange: "이 기간에 일정이 없습니다.",
};

export function CalendarView() {
  const { schedules, create, update, remove } = useSchedules();
  const [modal, setModal] = useState<{
    schedule?: Schedule;
    initialDate?: string;
  } | null>(null);

  const events: CalendarEvent[] = useMemo(
    () =>
      schedules.map((s) => {
        const dateStr = s.time ? `${s.date}T${s.time}` : s.date;
        const start = new Date(dateStr);
        const end = new Date(start.getTime() + 60 * 60 * 1000);
        return {
          id: s.id,
          title: [s.description, s.location].filter(Boolean).join(" @ ") || "일정",
          start,
          end,
          resource: s,
        };
      }),
    [schedules]
  );

  const handleSelectSlot = ({ start }: { start: Date }) => {
    const dateStr = moment(start).format("YYYY-MM-DD");
    setModal({ initialDate: dateStr });
  };

  const handleSelectEvent = (event: CalendarEvent) => {
    setModal({ schedule: event.resource });
  };

  const handleSave = async (data: Parameters<typeof create>[0]) => {
    if (modal?.schedule) {
      await update(modal.schedule.id, data);
    } else {
      await create(data);
    }
    setModal(null);
  };

  const handleDelete = async () => {
    if (modal?.schedule) {
      await remove(modal.schedule.id);
      setModal(null);
    }
  };

  return (
    <div className="h-full">
      <div className="flex justify-between items-center mb-3">
        <h2 className="text-lg font-semibold text-[var(--color-text-hi)] flex items-center gap-2">
          <CalendarDays size={18} />
          캘린더
        </h2>
        <Button
          variant="primary"
          size="sm"
          leftIcon={Plus}
          onClick={() => setModal({ initialDate: moment().format("YYYY-MM-DD") })}
        >
          새 일정
        </Button>
      </div>
      <Calendar
        localizer={localizer}
        events={events}
        startAccessor="start"
        endAccessor="end"
        selectable
        onSelectSlot={handleSelectSlot}
        onSelectEvent={handleSelectEvent}
        style={{ height: "calc(100vh - 160px)" }}
        views={["month"]}
        defaultView="month"
        messages={messages}
      />
      {modal && (
        <ScheduleModal
          schedule={modal.schedule}
          initialDate={modal.initialDate}
          onSave={handleSave}
          onDelete={modal.schedule ? handleDelete : undefined}
          onClose={() => setModal(null)}
        />
      )}
    </div>
  );
}
