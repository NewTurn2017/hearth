import { useState, useEffect, useCallback } from "react";
import type { Project, Priority, Category } from "../types";
import * as api from "../api";

export function useProjects(
  priorities: Set<Priority>,
  category: Category | null
) {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      // `category === null` means 전체 보기 — send no category filter so the
      // backend doesn't emit a `category IN (...)` clause (which would
      // exclude rows where category IS NULL).
      const data = await api.getProjects({
        priorities: [...priorities],
        categories: category ? [category] : [],
      });
      setProjects(data);
    } catch (e) {
      console.error("Failed to load projects:", e);
    } finally {
      setLoading(false);
    }
  }, [priorities, category]);

  useEffect(() => {
    load();
  }, [load]);

  // External writers (NewProjectDialog, AI actions) can broadcast this event
  // instead of wiring through props.
  useEffect(() => {
    const onChanged = () => {
      load();
    };
    window.addEventListener("projects:changed", onChanged);
    return () => window.removeEventListener("projects:changed", onChanged);
  }, [load]);

  const update = async (
    id: number,
    fields: Parameters<typeof api.updateProject>[1]
  ) => {
    const updated = await api.updateProject(id, fields);
    setProjects((prev) => prev.map((p) => (p.id === updated.id ? updated : p)));
    return updated;
  };

  const create = async (
    name: string,
    priority: string,
    category?: string,
    path?: string
  ) => {
    const created = await api.createProject(name, priority, category, path);
    setProjects((prev) => [...prev, created]);
    return created;
  };

  const remove = async (id: number) => {
    await api.deleteProject(id);
    setProjects((prev) => prev.filter((p) => p.id !== id));
  };

  const reorder = async (_priority: string, ids: number[]) => {
    await api.reorderProjects(ids);
    setProjects((prev) => {
      const updated = [...prev];
      ids.forEach((id, i) => {
        const idx = updated.findIndex((p) => p.id === id);
        if (idx !== -1) updated[idx] = { ...updated[idx], sort_order: i };
      });
      return updated.sort((a, b) => a.sort_order - b.sort_order);
    });
  };

  return { projects, loading, update, create, remove, reorder, reload: load };
}
