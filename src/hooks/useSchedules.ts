import { useState, useEffect, useCallback } from "react";
import type { Schedule } from "../types";
import * as api from "../api";

export function useSchedules() {
  const [schedules, setSchedules] = useState<Schedule[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getSchedules();
      setSchedules(data);
    } catch (e) {
      console.error("Failed to load schedules:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // External writers (AI agent approvals) broadcast this instead of
  // prop-drilling. Without the listener, a mutation-confirm would update
  // the DB but the calendar stays visually stale until the next remount.
  useEffect(() => {
    const onChanged = () => {
      load();
    };
    window.addEventListener("schedules:changed", onChanged);
    return () => window.removeEventListener("schedules:changed", onChanged);
  }, [load]);

  const create = async (data: Parameters<typeof api.createSchedule>[0]) => {
    const created = await api.createSchedule(data);
    setSchedules((prev) => [...prev, created]);
    return created;
  };

  const update = async (
    id: number,
    data: Parameters<typeof api.updateSchedule>[1]
  ) => {
    const updated = await api.updateSchedule(id, data);
    setSchedules((prev) =>
      prev.map((s) => (s.id === updated.id ? updated : s))
    );
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteSchedule(id);
    setSchedules((prev) => prev.filter((s) => s.id !== id));
  };

  return { schedules, loading, create, update, remove, reload: load };
}
