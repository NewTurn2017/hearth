export interface Project {
  id: number;
  priority: string;
  number: number | null;
  name: string;
  category: string | null;
  path: string | null;
  evaluation: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface Schedule {
  id: number;
  date: string;
  time: string | null;
  location: string | null;
  description: string | null;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface Memo {
  id: number;
  content: string;
  color: string;
  project_id: number | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface Client {
  id: number;
  company_name: string | null;
  ceo: string | null;
  phone: string | null;
  fax: string | null;
  email: string | null;
  offices: string | null;
  project_desc: string | null;
  status: string | null;
  created_at: string;
  updated_at: string;
}

export interface BackupInfo {
  path: string;
  filename: string;
  size_bytes: number;
  created: string;
}

export interface AiServerStatus {
  running: boolean;
  port: number;
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface ChatResponse {
  content: string;
  tool_calls: ToolCall[] | null;
}

export interface ToolCall {
  name: string;
  arguments: Record<string, unknown>;
}

export type Tab = "projects" | "calendar" | "memos";
export type Priority = "P0" | "P1" | "P2" | "P3" | "P4";
export type Category = "Active" | "Side" | "Lab" | "Tools" | "Lecture";

export const PRIORITIES: Priority[] = ["P0", "P1", "P2", "P3", "P4"];
export const CATEGORIES: Category[] = ["Active", "Side", "Lab", "Tools", "Lecture"];

export const PRIORITY_COLORS: Record<Priority, string> = {
  P0: "#ef4444",
  P1: "#f97316",
  P2: "#eab308",
  P3: "#3b82f6",
  P4: "#6b7280",
};

export const PRIORITY_LABELS: Record<Priority, string> = {
  P0: "긴급",
  P1: "높음",
  P2: "중간",
  P3: "낮음",
  P4: "참고",
};

export const CATEGORY_COLORS: Record<Category, string> = {
  Active: "#22c55e",
  Side: "#f97316",
  Lab: "#a855f7",
  Tools: "#6b7280",
  Lecture: "#3b82f6",
};

export const MEMO_COLORS = [
  { name: "yellow", bg: "#fef3c7", text: "#92400e" },
  { name: "pink", bg: "#fce7f3", text: "#9d174d" },
  { name: "blue", bg: "#dbeafe", text: "#1e40af" },
  { name: "green", bg: "#d1fae5", text: "#065f46" },
  { name: "purple", bg: "#ede9fe", text: "#5b21b6" },
];

// --- AI / Command Palette ---

export type ActionCommand =
  | "create_project"
  | "update_project"
  | "delete_project"
  | "create_schedule"
  | "update_schedule"
  | "delete_schedule"
  | "create_memo"
  | "update_memo"
  | "delete_memo"
  | "set_filter"
  | "focus_project";

export type ActionType = "mutation" | "navigation" | "info";

export interface AiAction {
  type: ActionType;
  label: string;
  command?: ActionCommand;
  args?: Record<string, unknown>;
}

export interface AiResponse {
  reply: string;
  actions: AiAction[];
}

export type AiServerState =
  | { kind: "idle" }
  | { kind: "starting" }
  | { kind: "running"; port: number }
  | { kind: "failed"; error: string };
