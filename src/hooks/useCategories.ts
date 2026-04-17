import { useCallback, useEffect, useState } from "react";
import type { CategoryRow } from "../types";
import * as api from "../api";

/**
 * Reactive store for user-editable project categories. Mirrors `useMemos`
 * conventions — every mutation dispatches `categories:changed` on `window`
 * so every other subscriber (Sidebar filter list, ProjectFormFields select,
 * ProjectCard category popover) refetches.
 *
 * The hook does **not** take filter arguments — the category list is small
 * and shared by every surface, so we keep a single reactive copy.
 */
export function useCategories() {
  const [categories, setCategories] = useState<CategoryRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getCategories();
      setCategories(data);
    } catch (e) {
      console.error("Failed to load categories:", e);
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
    window.addEventListener("categories:changed", onChanged);
    return () => window.removeEventListener("categories:changed", onChanged);
  }, [load]);

  const create = async (input: { name: string; color?: string }) => {
    const created = await api.createCategory(input);
    window.dispatchEvent(new CustomEvent("categories:changed"));
    return created;
  };

  const rename = async (id: number, name: string) => {
    const updated = await api.updateCategory(id, { name });
    // Project rows may have been cascaded — notify both listeners.
    window.dispatchEvent(new CustomEvent("categories:changed"));
    window.dispatchEvent(new CustomEvent("projects:changed"));
    return updated;
  };

  const recolor = async (id: number, color: string) => {
    const updated = await api.updateCategory(id, { color });
    window.dispatchEvent(new CustomEvent("categories:changed"));
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteCategory(id);
    window.dispatchEvent(new CustomEvent("categories:changed"));
  };

  const reorder = async (ids: number[]) => {
    await api.reorderCategories(ids);
    window.dispatchEvent(new CustomEvent("categories:changed"));
  };

  return {
    categories,
    loading,
    create,
    rename,
    recolor,
    remove,
    reorder,
    reload: load,
  };
}
