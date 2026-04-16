// src/command/types.ts
import type { LucideIcon } from "lucide-react";
import type { AiAction } from "../types";

/** Locally-executable quick action (Mode 1). */
export interface LocalCommand {
  id: string;
  label: string;
  hint?: string;
  icon: LucideIcon;
  /** If true, `run` is treated as a mutation and a confirm dialog is shown. */
  mutation?: boolean;
  /** Human-readable confirm text (used when mutation=true). */
  confirmMessage?: string;
  /** Executed on ⏎. Returns an optional undo function for Toast "Undo". */
  run: () => Promise<(() => void | Promise<void>) | void>;
}

/** Unified result item displayed in the palette list. */
export type PaletteItem =
  | { kind: "local"; cmd: LocalCommand }
  | { kind: "ai"; action: AiAction };
