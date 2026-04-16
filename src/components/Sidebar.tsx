import { PRIORITIES, CATEGORIES, PRIORITY_COLORS, PRIORITY_LABELS, CATEGORY_COLORS } from "../types";
import type { Priority, Category } from "../types";
import { cn } from "../lib/cn";

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
    <aside className="w-56 shrink-0 bg-[var(--color-surface-1)] border-r border-[var(--color-border)] py-5 px-3 flex flex-col gap-7 overflow-y-auto">
      <FilterGroup label="우선순위">
        {PRIORITIES.map((p) => (
          <FilterItem
            key={p}
            active={activePriorities.has(p)}
            onClick={() => onTogglePriority(p)}
            dot={PRIORITY_COLORS[p]}
            text={`${p} — ${PRIORITY_LABELS[p]}`}
          />
        ))}
      </FilterGroup>

      <FilterGroup label="카테고리">
        {CATEGORIES.map((c) => (
          <FilterItem
            key={c}
            active={activeCategories.has(c)}
            onClick={() => onToggleCategory(c)}
            dot={CATEGORY_COLORS[c]}
            text={c}
          />
        ))}
      </FilterGroup>
    </aside>
  );
}

function FilterGroup({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <h3 className="text-label text-[var(--color-text-dim)] mb-2 px-1">{label}</h3>
      <div className="flex flex-col gap-0.5">{children}</div>
    </div>
  );
}

function FilterItem({
  active,
  onClick,
  dot,
  text,
}: {
  active: boolean;
  onClick: () => void;
  dot: string;
  text: string;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        "flex items-center gap-2 h-7 px-2 rounded-[var(--radius-sm)] text-[12px] text-left",
        "transition-colors duration-[120ms]",
        active
          ? "bg-[var(--color-surface-2)] text-[var(--color-text)]"
          : "text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)]"
      )}
      aria-pressed={active}
    >
      <span
        className={cn("w-2 h-2 rounded-full shrink-0", !active && "opacity-40")}
        style={{ backgroundColor: dot }}
      />
      {text}
    </button>
  );
}
