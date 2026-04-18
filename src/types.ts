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
  remind_before_5min: boolean;
  remind_at_start: boolean;
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

// DB-shaped category row. Note: this is a different type from the legacy
// `Category` string-union below — the union stays in the file as the seed
// source-of-truth and as a fallback when `useCategories` is still loading.
export interface CategoryRow {
  id: number;
  name: string;
  color: string;
  sort_order: number;
  usage_count: number;
  created_at: string;
  updated_at: string;
}

// OpenAI-shaped chat message. Assistant turns can omit `content` when they
// emit `tool_calls`; `role: "tool"` turns carry the tool result keyed by
// `tool_call_id`. `tool_calls` stays `unknown[]` on the TS side because the
// server echoes its own structure verbatim and we don't need to introspect.
export interface ChatMessage {
  role: "user" | "assistant" | "system" | "tool";
  content?: string;
  name?: string;
  tool_calls?: unknown[];
  tool_call_id?: string;
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

/** Tool invocation parsed from a model response. Mirrors `ai_tools::ToolCall`
 *  on the Rust side; `arguments` is pre-decoded from the OpenAI wire string. */
export interface ToolCall {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
}

/** Result of one turn of the agent loop.
 *  - `final`  : model stopped requesting tools; `client_intents` are any UI
 *               navigation tools collected along the way (set_filter etc.).
 *  - `pending`: model asked to perform a mutation; `history` must be passed
 *               back to `ai_confirm` along with the approved `call` so the
 *               loop can resume. */
export type AgentResult =
  | { kind: "final"; reply: string; client_intents: ToolCall[] }
  | { kind: "pending"; call: ToolCall; label: string; history: ChatMessage[] };

export type AiServerState =
  | { kind: "idle" }
  | { kind: "starting" }
  | { kind: "running"; port: number }
  | { kind: "failed"; error: string };

/** Persisted AI provider configuration. The raw OpenAI API key never crosses
 *  this boundary — `has_openai_key` tells the UI whether to show a "stored"
 *  badge instead of a blank password field. Model selection is not exposed:
 *  the backend uses a hard-coded OpenAI model and auto-detects the local MLX
 *  model from the running process. */
export interface AiSettings {
  provider: "local" | "openai";
  has_openai_key: boolean;
}
