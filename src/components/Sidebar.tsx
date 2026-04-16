import {
  PRIORITIES,
  CATEGORIES,
  PRIORITY_COLORS,
  PRIORITY_LABELS,
  CATEGORY_COLORS,
} from "../types";
import type { Priority, Category } from "../types";

export function Sidebar({
  activePriorities,
  activeCategories,
  onTogglePriority,
  onToggleCategory,
}: {
  activePriorities: Set<Priority>;
  activeCategories: Set<Category>;
  onTogglePriority: (p: Priority) => void;
  onToggleCategory: (c: Category) => void;
}) {
  return (
    <div className="w-48 shrink-0 bg-[var(--bg-secondary)] border-r border-[var(--border-color)] p-3 flex flex-col gap-4 overflow-y-auto">
      <div>
        <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase mb-2">
          우선순위
        </h3>
        <div className="flex flex-col gap-1">
          {PRIORITIES.map((p) => (
            <button
              key={p}
              onClick={() => onTogglePriority(p)}
              className={`flex items-center gap-2 px-2 py-1 rounded text-sm transition-colors ${
                activePriorities.has(p)
                  ? "bg-[var(--bg-tertiary)] text-[var(--text-primary)]"
                  : "text-[var(--text-secondary)] opacity-50"
              }`}
            >
              <span
                className="w-2.5 h-2.5 rounded-full shrink-0"
                style={{ backgroundColor: PRIORITY_COLORS[p] }}
              />
              {p} — {PRIORITY_LABELS[p]}
            </button>
          ))}
        </div>
      </div>

      <div>
        <h3 className="text-xs font-semibold text-[var(--text-secondary)] uppercase mb-2">
          카테고리
        </h3>
        <div className="flex flex-col gap-1">
          {CATEGORIES.map((c) => (
            <button
              key={c}
              onClick={() => onToggleCategory(c)}
              className={`flex items-center gap-2 px-2 py-1 rounded text-sm transition-colors ${
                activeCategories.has(c)
                  ? "bg-[var(--bg-tertiary)] text-[var(--text-primary)]"
                  : "text-[var(--text-secondary)] opacity-50"
              }`}
            >
              <span
                className="w-2.5 h-2.5 rounded-full shrink-0"
                style={{ backgroundColor: CATEGORY_COLORS[c] }}
              />
              {c}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
