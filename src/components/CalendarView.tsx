import { useEffect, useMemo, useState } from "react";
import { Calendar, momentLocalizer } from "react-big-calendar";
import withDragAndDrop from "react-big-calendar/lib/addons/dragAndDrop";
import moment from "moment";
import "moment/locale/ko";
import "react-big-calendar/lib/css/react-big-calendar.css";
import "react-big-calendar/lib/addons/dragAndDrop/styles.css";
import { CalendarDays, Plus } from "lucide-react";
import { ScheduleModal } from "./ScheduleModal";
import { useSchedules } from "../hooks/useSchedules";
import type { Schedule } from "../types";
import { Button } from "../ui/Button";

moment.locale("ko");
const localizer = momentLocalizer(moment);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const DnDCalendar = withDragAndDrop<CalendarEvent>(Calendar as any);

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
  showMore: (total: number) => `+${total}개 더보기`,
};

const formats = {
  monthHeaderFormat: (date: Date) => moment(date).format("YYYY년 M월"),
  weekdayFormat: (date: Date) => moment(date).format("ddd"),
  dateFormat: (date: Date) => moment(date).format("D"),
  dayHeaderFormat: (date: Date) => moment(date).format("M월 D일 dddd"),
  dayRangeHeaderFormat: ({ start, end }: { start: Date; end: Date }) =>
    `${moment(start).format("M월 D일")} – ${moment(end).format("M월 D일")}`,
  agendaHeaderFormat: ({ start, end }: { start: Date; end: Date }) =>
    `${moment(start).format("YYYY년 M월 D일")} – ${moment(end).format("YYYY년 M월 D일")}`,
  agendaDateFormat: (date: Date) => moment(date).format("M월 D일 (ddd)"),
  agendaTimeFormat: (date: Date) => moment(date).format("A h:mm"),
  agendaTimeRangeFormat: ({ start, end }: { start: Date; end: Date }) =>
    `${moment(start).format("A h:mm")} – ${moment(end).format("A h:mm")}`,
  eventTimeRangeFormat: ({ start, end }: { start: Date; end: Date }) =>
    `${moment(start).format("A h:mm")} – ${moment(end).format("A h:mm")}`,
  selectRangeFormat: ({ start, end }: { start: Date; end: Date }) =>
    `${moment(start).format("A h:mm")} – ${moment(end).format("A h:mm")}`,
  timeGutterFormat: (date: Date) => moment(date).format("A h:mm"),
};

export function CalendarView() {
  const { schedules, create, update, remove } = useSchedules();
  const [modal, setModal] = useState<{
    schedule?: Schedule;
    initialDate?: string;
  } | null>(null);
  // Controlled so prev/next/today toolbar buttons actually advance the view —
  // react-big-calendar's uncontrolled mode silently drops navigation events
  // when you don't pair `date` with `onNavigate`.
  const [currentDate, setCurrentDate] = useState<Date>(() => new Date());
  const [pendingFocusId, setPendingFocusId] = useState<number | null>(null);
  const [highlightedId, setHighlightedId] = useState<number | null>(null);

  // FindPalette emits `schedule:focus` with the target schedule id. On a cold
  // tab switch the `schedules` list hasn't loaded yet, so we just stash the
  // id here and let a second effect apply it once the data arrives.
  useEffect(() => {
    const onFocus = (e: Event) => {
      const detail = (e as CustomEvent<{ scheduleId?: number; date?: string }>)
        .detail;
      const id = detail?.scheduleId;
      if (typeof id !== "number") return;
      setPendingFocusId(id);
    };
    window.addEventListener("schedule:focus", onFocus);
    return () => window.removeEventListener("schedule:focus", onFocus);
  }, []);

  useEffect(() => {
    if (pendingFocusId === null) return;
    const found = schedules.find((s) => s.id === pendingFocusId);
    if (!found) return;
    setCurrentDate(moment(found.date).toDate());
    setHighlightedId(found.id);
    setPendingFocusId(null);
    const t = window.setTimeout(() => setHighlightedId(null), 2400);
    return () => window.clearTimeout(t);
  }, [pendingFocusId, schedules]);

  const events: CalendarEvent[] = useMemo(
    () =>
      schedules.map((s) => {
        const dateStr = s.time ? `${s.date}T${s.time}` : s.date;
        const start = new Date(dateStr);
        const end = new Date(start.getTime() + 60 * 60 * 1000);
        const hasReminder = s.remind_before_5min || s.remind_at_start;
        return {
          id: s.id,
          title:
            (hasReminder ? "🔔 " : "") +
            ([s.description, s.location].filter(Boolean).join(" @ ") || "일정"),
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

  const handleEventDrop = async ({
    event,
    start,
  }: {
    event: CalendarEvent;
    start: Date | string;
    end: Date | string;
  }) => {
    const s = event.resource;
    const newDate = moment(start).format("YYYY-MM-DD");
    await update(s.id, {
      date: newDate,
      time: s.time ?? undefined,
      location: s.location ?? undefined,
      description: s.description ?? undefined,
      notes: s.notes ?? undefined,
      remind_before_5min: s.remind_before_5min,
      remind_at_start: s.remind_at_start,
    });
  };

  return (
    <div className="flex flex-col flex-1 min-h-0">
      <div className="flex justify-between items-center mb-5">
        <h2 className="text-heading text-[var(--color-text-hi)] flex items-center gap-2">
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
      <div className="flex-1 min-h-0">
        <DnDCalendar
          localizer={localizer}
          events={events}
          startAccessor="start"
          endAccessor="end"
          selectable
          onSelectSlot={handleSelectSlot}
          onSelectEvent={handleSelectEvent}
          onEventDrop={handleEventDrop}
          draggableAccessor={() => true}
          style={{ height: "100%" }}
          views={["month"]}
          defaultView="month"
          date={currentDate}
          onNavigate={setCurrentDate}
          messages={messages}
          formats={formats}
          culture="ko"
          eventPropGetter={(event) =>
            event.id === highlightedId
              ? { className: "rbc-event-find-highlight" }
              : {}
          }
        />
      </div>
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
