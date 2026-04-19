// src/components/FindPalette.tsx
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import Fuse from "fuse.js";
import { Search, FolderKanban, StickyNote, Calendar as CalendarIcon } from "lucide-react";
import * as api from "../api";
import { Icon } from "../ui/Icon";
import { Kbd } from "../ui/Kbd";
import { cn } from "../lib/cn";
import type { Project, Memo, Schedule, Tab } from "../types";
import { PRIORITY_LABELS, type Priority } from "../types";

type FindItem =
  | { kind: "project"; id: number; title: string; subtitle: string; raw: Project }
  | { kind: "memo"; id: number; title: string; subtitle: string; raw: Memo }
  | { kind: "schedule"; id: number; title: string; subtitle: string; raw: Schedule };

const KIND_META: Record<FindItem["kind"], { label: string; icon: typeof Search }> = {
  project: { label: "프로젝트", icon: FolderKanban },
  memo: { label: "메모", icon: StickyNote },
  schedule: { label: "일정", icon: CalendarIcon },
};

function buildItems(projects: Project[], memos: Memo[], schedules: Schedule[]): FindItem[] {
  const items: FindItem[] = [];
  for (const p of projects) {
    const priLabel = PRIORITY_LABELS[p.priority as Priority] ?? p.priority;
    const subParts = [priLabel, p.category ?? "", p.evaluation ?? ""].filter(Boolean);
    items.push({
      kind: "project",
      id: p.id,
      title: p.name,
      subtitle: subParts.join(" · "),
      raw: p,
    });
  }
  for (const m of memos) {
    const firstLine = m.content.split("\n")[0] ?? "";
    items.push({
      kind: "memo",
      id: m.id,
      title: firstLine || "(빈 메모)",
      subtitle: m.content.length > firstLine.length ? m.content.slice(0, 120) : "",
      raw: m,
    });
  }
  for (const s of schedules) {
    const title = s.description ?? "(제목 없음)";
    const when = [s.date, s.time].filter(Boolean).join(" ");
    const sub = [when, s.location ?? "", s.notes ?? ""].filter(Boolean).join(" · ");
    items.push({
      kind: "schedule",
      id: s.id,
      title,
      subtitle: sub,
      raw: s,
    });
  }
  return items;
}

/** Per-kind field weights — titles dominate, body/notes are tiebreakers. */
const FUSE_KEYS = [
  { name: "title", weight: 0.7 },
  { name: "subtitle", weight: 0.3 },
];

export function FindPalette({
  open,
  onClose,
  onNavigate,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (tab: Tab) => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [items, setItems] = useState<FindItem[]>([]);

  // Reload snapshot each time the palette opens — data hooks push changed
  // events we'd otherwise have to subscribe to individually. Re-fetching on
  // open is cheap compared to keeping three event listeners alive.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    void Promise.all([api.getProjects(), api.getMemos(), api.getSchedules()])
      .then(([projects, memos, schedules]) => {
        if (!cancelled) setItems(buildItems(projects, memos, schedules));
      })
      .catch(() => {
        if (!cancelled) setItems([]);
      });
    setTimeout(() => inputRef.current?.focus(), 0);
    return () => {
      cancelled = true;
    };
  }, [open]);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setActiveIndex(0);
    }
  }, [open]);

  const fuse = useMemo(
    () =>
      new Fuse(items, {
        keys: FUSE_KEYS,
        threshold: 0.4,
        ignoreLocation: true,
        includeScore: false,
      }),
    [items]
  );

  const results = useMemo(() => {
    const q = query.trim();
    if (!q) return items.slice(0, 50);
    return fuse.search(q, { limit: 50 }).map((r) => r.item);
  }, [query, fuse, items]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query]);

  const onSelect = useCallback(
    (item: FindItem) => {
      // Switch tab first, then dispatch the focus/open event on the next
      // tick so the target tab's listener is mounted before the event fires.
      // Without the defer, a cold tab switch drops the event.
      const dispatchFocus = () => {
        if (item.kind === "project") {
          window.dispatchEvent(
            new CustomEvent("project:focus", { detail: { projectId: item.id } })
          );
        } else if (item.kind === "memo") {
          window.dispatchEvent(
            new CustomEvent("memo:focus", { detail: { memoId: item.id } })
          );
        } else {
          window.dispatchEvent(
            new CustomEvent("schedule:focus", {
              detail: { scheduleId: item.id, date: item.raw.date },
            })
          );
        }
      };
      if (item.kind === "project") onNavigate("projects");
      else if (item.kind === "memo") onNavigate("memos");
      else onNavigate("calendar");
      setTimeout(dispatchFocus, 0);
      onClose();
    },
    [onNavigate, onClose]
  );

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLInputElement>) => {
      if (e.nativeEvent.isComposing) return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const item = results[activeIndex];
        if (item) onSelect(item);
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [results, activeIndex, onSelect, onClose]
  );

  if (!open) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-[95] flex items-start justify-center pt-[15vh] bg-black/40"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className={cn(
          "w-full max-w-[620px] rounded-[var(--radius-xl)]",
          "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
          "shadow-[var(--shadow-e3)] overflow-hidden"
        )}
      >
        <div className="flex items-center gap-2 h-12 px-4 border-b border-[var(--color-border)]">
          <Icon icon={Search} size={18} className="text-[var(--color-text-dim)]" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="프로젝트·메모·일정에서 검색…"
            className="flex-1 bg-transparent outline-none text-[14px] text-[var(--color-text-hi)] placeholder:text-[var(--color-text-dim)]"
            autoFocus
          />
          <span className="text-[11px] text-[var(--color-text-muted)]">
            {results.length}건
          </span>
          <Kbd>ESC</Kbd>
          <Kbd>⌘F</Kbd>
        </div>
        <div className="max-h-[50vh] overflow-y-auto">
          {results.length === 0 ? (
            <div className="px-4 py-8 text-center text-[13px] text-[var(--color-text-muted)]">
              {query.trim() ? "일치하는 항목이 없습니다." : "검색어를 입력하세요."}
            </div>
          ) : (
            <ul role="listbox">
              {results.map((item, i) => {
                const meta = KIND_META[item.kind];
                const active = i === activeIndex;
                return (
                  <li
                    key={`${item.kind}-${item.id}`}
                    role="option"
                    aria-selected={active}
                    onMouseEnter={() => setActiveIndex(i)}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      onSelect(item);
                    }}
                    className={cn(
                      "flex items-center gap-3 px-4 py-2.5 cursor-pointer",
                      active
                        ? "bg-[var(--color-surface-2)]"
                        : "hover:bg-[var(--color-surface-2)]"
                    )}
                  >
                    <Icon
                      icon={meta.icon}
                      size={16}
                      className="text-[var(--color-text-dim)] shrink-0"
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-baseline gap-2">
                        <span className="text-[13px] text-[var(--color-text-hi)] truncate">
                          {item.title}
                        </span>
                        <span className="text-[10px] uppercase tracking-wide text-[var(--color-text-muted)] shrink-0">
                          {meta.label}
                        </span>
                      </div>
                      {item.subtitle && (
                        <div className="text-[11px] text-[var(--color-text-muted)] truncate">
                          {item.subtitle}
                        </div>
                      )}
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}
