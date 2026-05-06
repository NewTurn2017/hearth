import { useCallback, useEffect, useState } from "react";
import type { MemoTag } from "../types";
import * as api from "../api";

/**
 * Reactive store for memo-only tags. Mutations broadcast `memo-tags:changed`
 * so every tag picker/filter reloads. Mutations that can change visible memo
 * labels, colors, or links also broadcast `memos:changed`.
 */
export function useMemoTags() {
  const [tags, setTags] = useState<MemoTag[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getMemoTags();
      setTags(data);
    } catch (e) {
      console.error("Failed to load memo tags:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const onChanged = () => {
      load();
    };
    window.addEventListener("memo-tags:changed", onChanged);
    return () => window.removeEventListener("memo-tags:changed", onChanged);
  }, [load]);

  const dispatchTagsChanged = () => {
    window.dispatchEvent(new CustomEvent("memo-tags:changed"));
  };

  const dispatchMemosChanged = () => {
    window.dispatchEvent(new CustomEvent("memos:changed"));
  };

  const create = async (input: { name: string; color?: string }) => {
    const created = await api.createMemoTag(input);
    dispatchTagsChanged();
    return created;
  };

  const rename = async (id: number, name: string) => {
    const updated = await api.updateMemoTag(id, { name });
    dispatchTagsChanged();
    dispatchMemosChanged();
    return updated;
  };

  const recolor = async (id: number, color: string) => {
    const updated = await api.updateMemoTag(id, { color });
    dispatchTagsChanged();
    dispatchMemosChanged();
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteMemoTag(id);
    dispatchTagsChanged();
    dispatchMemosChanged();
  };

  const reorder = async (ids: number[]) => {
    await api.reorderMemoTags(ids);
    dispatchTagsChanged();
  };

  return {
    tags,
    loading,
    create,
    rename,
    recolor,
    remove,
    reorder,
    reload: load,
  };
}
