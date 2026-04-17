import { invoke } from "@tauri-apps/api/core";
import type {
  Project,
  Schedule,
  Memo,
  Client,
  BackupInfo,
} from "./types";

// 프로젝트
export const getProjects = (filter?: {
  priorities?: string[];
  categories?: string[];
}) => invoke<Project[]>("get_projects", { filter: filter ?? null });

export const createProject = (
  name: string,
  priority: string,
  category?: string,
  path?: string
) =>
  invoke<Project>("create_project", {
    name,
    priority,
    category: category ?? null,
    path: path ?? null,
  });

export const updateProject = (
  id: number,
  fields: {
    name?: string;
    priority?: string;
    category?: string;
    path?: string;
    evaluation?: string;
  }
) => invoke<Project>("update_project", { id, fields });

export const deleteProject = (id: number) =>
  invoke<void>("delete_project", { id });

export const reorderProjects = (ids: number[]) =>
  invoke<void>("reorder_projects", { ids });

export const searchProjects = (query: string) =>
  invoke<Project[]>("search_projects", { query });

// 일정
export const getSchedules = (month?: string) =>
  invoke<Schedule[]>("get_schedules", { month: month ?? null });

export const createSchedule = (data: {
  date: string;
  time?: string;
  location?: string;
  description?: string;
  notes?: string;
}) => invoke<Schedule>("create_schedule", { data });

export const updateSchedule = (
  id: number,
  data: {
    date: string;
    time?: string;
    location?: string;
    description?: string;
    notes?: string;
  }
) => invoke<Schedule>("update_schedule", { id, data });

export const deleteSchedule = (id: number) =>
  invoke<void>("delete_schedule", { id });

// 메모
export const getMemos = () => invoke<Memo[]>("get_memos");

export const createMemo = (data: {
  content: string;
  color?: string;
  project_id?: number;
}) => invoke<Memo>("create_memo", { data });

export const updateMemo = (
  id: number,
  fields: { content?: string; color?: string; project_id?: number | null }
) => invoke<Memo>("update_memo", { id, fields });

export const deleteMemo = (id: number) => invoke<void>("delete_memo", { id });

export const reorderMemos = (ids: number[]) =>
  invoke<void>("reorder_memos", { ids });

// 고객사
export const getClients = () => invoke<Client[]>("get_clients");

// 액션
export const openInGhostty = (path: string) =>
  invoke<void>("open_in_ghostty", { path });

export const openInFinder = (path: string) =>
  invoke<void>("open_in_finder", { path });

export const importExcel = (filePath: string, clearExisting: boolean) =>
  invoke<{ projects_imported: number }>("import_excel", {
    filePath,
    clearExisting,
  });

// 백업
export const backupDb = (destPath?: string) =>
  invoke<string>("backup_db", { destPath: destPath ?? null });

export const restoreDb = (srcPath: string) =>
  invoke<void>("restore_db", { srcPath });

export const listBackups = () => invoke<BackupInfo[]>("list_backups");

// AI
import type {
  AgentResult,
  AiServerState,
  AiSettings,
  ChatMessage as _CM,
  ToolCall,
} from "./types";

export const startAiServer = () => invoke<AiServerState>("start_ai_server");
export const stopAiServer = () => invoke<void>("stop_ai_server");
export const aiServerStatus = () => invoke<AiServerState>("ai_server_status");
export const aiChat = (messages: _CM[]) =>
  invoke<AgentResult>("ai_chat", { messages });
export const aiConfirm = (history: _CM[], call: ToolCall) =>
  invoke<AgentResult>("ai_confirm", { history, call });

// AI settings (provider + OpenAI key). Model selection is intentionally
// absent — the backend hard-codes OPENAI_MODEL and auto-detects local MLX.
export const getAiSettings = () => invoke<AiSettings>("get_ai_settings");
/** `openai_api_key` semantics — mirror the Rust side:
 *   • `undefined`  : leave stored key untouched
 *   • `""`         : clear
 *   • `"sk-..."`   : overwrite */
export const saveAiSettings = (input: {
  provider: "local" | "openai";
  openai_api_key?: string;
}) => invoke<AiSettings>("save_ai_settings", { input });

// UI scale (Cmd+=/-/0). Persisted in the settings KV table.
export const getUiScale = () => invoke<number>("get_ui_scale");
export const setUiScale = (scale: number) =>
  invoke<void>("set_ui_scale", { scale });

// Memo operations keyed by the user-facing #N badge instead of a raw id.
// The backend resolves N via sort_order OFFSET, so callers pass the number
// the user sees on the card.
export const updateMemoByNumber = (
  number: number,
  fields: { content?: string; color?: string; project_id?: number | null }
) => invoke<Memo>("update_memo_by_number", { number, fields });

export const deleteMemoByNumber = (number: number) =>
  invoke<void>("delete_memo_by_number", { number });
