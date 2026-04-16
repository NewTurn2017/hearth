// src/command/executeAiAction.ts
import type { AiAction } from "../types";
import * as api from "../api";

type Args = Record<string, unknown>;

export async function executeAiAction(
  action: AiAction,
  onNavigate?: (kind: "filter" | "focusProject", payload: Args) => void
): Promise<(() => Promise<void>) | undefined> {
  const args = (action.args ?? {}) as Args;

  switch (action.command) {
    case "create_project": {
      const created = await api.createProject(
        String(args.name ?? ""),
        String(args.priority ?? "P2"),
        args.category ? String(args.category) : undefined,
        args.path ? String(args.path) : undefined
      );
      return async () => {
        await api.deleteProject(created.id);
      };
    }
    case "update_project": {
      const id = Number(args.id);
      const fields = (args.fields ?? {}) as Record<string, string>;
      await api.updateProject(id, fields);
      return;
    }
    case "delete_project": {
      await api.deleteProject(Number(args.id));
      return;
    }
    case "create_schedule": {
      const data = args as { date: string; time?: string; location?: string; description?: string; notes?: string };
      const created = await api.createSchedule(data);
      return async () => {
        await api.deleteSchedule(created.id);
      };
    }
    case "update_schedule": {
      const id = Number(args.id);
      const fields = args.fields as { date: string; time?: string; location?: string; description?: string; notes?: string };
      await api.updateSchedule(id, fields);
      return;
    }
    case "delete_schedule": {
      await api.deleteSchedule(Number(args.id));
      return;
    }
    case "create_memo": {
      const data = args as { content: string; color?: string; project_id?: number };
      const created = await api.createMemo(data);
      return async () => {
        await api.deleteMemo(created.id);
      };
    }
    case "update_memo": {
      const id = Number(args.id);
      const fields = args.fields as { content?: string; color?: string; project_id?: number | null };
      await api.updateMemo(id, fields);
      return;
    }
    case "delete_memo": {
      await api.deleteMemo(Number(args.id));
      return;
    }
    case "set_filter": {
      onNavigate?.("filter", args);
      return;
    }
    case "focus_project": {
      onNavigate?.("focusProject", args);
      return;
    }
    default:
      return;
  }
}
