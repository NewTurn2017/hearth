// src/command/useCommandState.ts
import { useMemo, useState } from "react";
import type { LocalCommand } from "./types";

export type Mode = "local" | "ai";

export function deriveMode(
  query: string,
  localMatches: LocalCommand[]
): { mode: Mode; aiQuery: string } {
  const trimmed = query.trim();
  if (trimmed === "" || trimmed.startsWith("/")) {
    return { mode: "local", aiQuery: "" };
  }
  if (trimmed.startsWith("?")) {
    return { mode: "ai", aiQuery: trimmed.slice(1).trim() };
  }
  if (localMatches.length > 0) {
    return { mode: "local", aiQuery: "" };
  }
  return { mode: "ai", aiQuery: trimmed };
}

export function filterLocal(query: string, commands: LocalCommand[]): LocalCommand[] {
  const q = query.replace(/^\//, "").trim().toLowerCase();
  if (!q) return commands;
  return commands.filter((c) =>
    (c.label + " " + (c.hint ?? "")).toLowerCase().includes(q)
  );
}

export function useCommandState(commands: LocalCommand[]) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const localMatches = useMemo(() => filterLocal(query, commands), [query, commands]);
  const { mode, aiQuery } = useMemo(
    () => deriveMode(query, localMatches),
    [query, localMatches]
  );

  const reset = () => {
    setQuery("");
  };

  return {
    open,
    setOpen,
    query,
    setQuery,
    localMatches,
    mode,
    aiQuery,
    reset,
  };
}
