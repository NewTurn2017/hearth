import { useState, useEffect, useCallback } from "react";
import type { Memo } from "../types";
import * as api from "../api";

export function useMemos() {
  const [memos, setMemos] = useState<Memo[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getMemos();
      setMemos(data);
    } catch (e) {
      console.error("Failed to load memos:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  // External writers (AI agent approvals) broadcast this instead of
  // prop-drilling. Without the listener, a mutation-confirm would update
  // the DB but the board stays visually stale until the next remount.
  useEffect(() => {
    const onChanged = () => {
      load();
    };
    window.addEventListener("memos:changed", onChanged);
    return () => window.removeEventListener("memos:changed", onChanged);
  }, [load]);

  const create = async (data: Parameters<typeof api.createMemo>[0]) => {
    const created = await api.createMemo(data);
    setMemos((prev) => [...prev, created]);
    return created;
  };

  const update = async (
    id: number,
    fields: Parameters<typeof api.updateMemo>[1]
  ) => {
    const updated = await api.updateMemo(id, fields);
    setMemos((prev) => prev.map((m) => (m.id === updated.id ? updated : m)));
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteMemo(id);
    setMemos((prev) => prev.filter((m) => m.id !== id));
  };

  const reorder = async (ids: number[]) => {
    await api.reorderMemos(ids);
    setMemos((prev) => {
      const ordered = ids
        .map((id) => prev.find((m) => m.id === id))
        .filter(Boolean) as Memo[];
      return ordered.map((m, i) => ({ ...m, sort_order: i }));
    });
  };

  return { memos, loading, create, update, remove, reorder, reload: load };
}
