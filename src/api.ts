import { invoke } from "@tauri-apps/api/core";
import type {
  Project,
  Schedule,
  Memo,
  Client,
  BackupInfo,
  AiServerStatus,
  ChatMessage,
  ChatResponse,
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
export const startAiServer = () =>
  invoke<AiServerStatus>("start_ai_server");

export const stopAiServer = () => invoke<void>("stop_ai_server");

export const aiServerStatus = () =>
  invoke<AiServerStatus>("ai_server_status");

export const aiChat = (messages: ChatMessage[]) =>
  invoke<ChatResponse>("ai_chat", { messages });
