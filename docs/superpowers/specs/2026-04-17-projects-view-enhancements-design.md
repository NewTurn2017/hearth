# Projects View Enhancements — Design

**Date**: 2026-04-17
**Status**: Approved for implementation planning
**Scope**: Cross-priority drag-and-drop, UI zoom, card-grid project view + detail modal, memo grouping + global #N badges, AI memo tools

## Summary

Three user-requested improvements to the Hearth projects experience, plus a scope expansion surfaced mid-brainstorming (memo grouping + AI addressability). All changes are UI/UX refinements on top of existing schema; no migrations required.

1. **Cross-priority drag-and-drop** — dragging a project card between P0..P4 groups updates its priority and drops at the precise drop position.
2. **UI zoom** — `Cmd+=` / `Cmd+-` / `Cmd+0` cycle through four scale steps using CSS `zoom`, persisted to the settings table.
3. **Card-grid project view + double-click detail modal** — replace the single-row list with a 2-column detailed card grid. Double-click opens a dialog combining the edit form with a memo CRUD panel scoped to that project.
4. **Memo grouping + global #N badges** — MemoBoard renders memos grouped by their linked project (with a trailing "기타" group), each card showing a `#N` badge derived from global `sort_order`. New AI memo tools use the `#N` reference for natural-language operations.

## Architecture

### Modules touched

```
src/components/ProjectList.tsx          grid 2-col + unified DndContext
src/components/ProjectCard.tsx          detailed card layout + dblclick handler
src/components/ProjectDetailDialog.tsx  NEW — edit form + memo CRUD panel
src/components/MemoBoard.tsx            grouped rendering + #N badges
src/components/MemoCard.tsx             sequenceNumber prop
src/hooks/useUiScale.ts                 NEW — zoom state + shortcut binding
src/App.css                             :root zoom target, existing scale utilities preserved

src-tauri/src/cmd_memos.rs              add update_memo_by_number / delete_memo_by_number; extend create_memo with project_name
src-tauri/src/ai_tools.rs               3 new tool specs
src-tauri/src/cmd_settings.rs           get_ui_scale / set_ui_scale
src/command/buildSystemPrompt.ts        memo-domain prose for #N semantics
```

### Data flow — cross-priority drag

```
User drops P0 card into P2 position 3
  → ProjectList.onDragEnd
    → api.updateProject(id, { priority: "P2" })
    → api.reorderProjects(newP2Ids)
    → api.reorderProjects(newP0Ids)
  → useProjects invalidate → refetch → UI reflects canonical server state
```

Three serial writes. On partial failure: toast a warning and let the next refetch reconcile with the server.

### Data flow — detail modal memo CRUD

```
Double-click card → setDetailProjectId(id)
  → Dialog mounts → reuses the parent useMemos() and filters locally
    (const scoped = memos.filter(m => m.project_id === detailProjectId))
  → user creates/edits/deletes inline → optimistic local update + API call
  → onClose resets detailProjectId
```

### Data flow — zoom

```
App mount → useUiScale → api.getUiScale → documentElement.style.zoom
User Cmd+= → bump(+1) → setScale + zoom + api.setUiScale (fire-and-forget)
```

## Components

### 1. Cross-priority drag-and-drop

Replace per-group `DndContext` with a single context wrapping all five groups. dnd-kit raises `onDragEnd` even when source and destination belong to different `SortableContext`s.

```ts
const handleDragEnd = (event: DragEndEvent) => {
  const { active, over } = event;
  if (!over) return;

  const source = projectById(active.id);
  const target = deriveTarget(over);                   // { priority, overId }
  if (!target) return;

  if (source.priority === target.priority) {
    if (active.id === over.id) return;
    const ids = reorderWithin(groups.get(target.priority), active.id, over.id);
    onReorder(target.priority, ids);
    return;
  }

  // Cross-group move: update priority + rewrite both groups' sort_order.
  const nextTargetIds = insertAt(groups.get(target.priority), active.id, target.overId);
  const nextSourceIds = remove(groups.get(source.priority), active.id);
  onUpdate(active.id, { priority: target.priority });
  onReorder(target.priority, nextTargetIds);
  onReorder(source.priority, nextSourceIds);
};
```

`deriveTarget` maps `over.id` to a priority: a card id resolves to the owning group; a synthetic drop-zone id (`priority-P3-empty`) resolves to that priority with `overId = null` (append).

Empty groups get a `useDroppable` zone so "move to an empty priority" is reachable. Drag overlay uses `DragOverlay` so the ghost follows the cursor across group borders.

### 2. ProjectCard — detailed 2-column layout

```
┌────────────────────────────────────────┐
│ ≡  PickAtSoul              [▶][📁][✕] │
│    [P0] [Active]                       │
│    메인 개발 진행 중 초기단계.           │
│    클라이언트 협의 단계에서…            │
│    ~/dev/active/pickatsoul              │
└────────────────────────────────────────┘
```

Grid: `grid grid-cols-1 md:grid-cols-2 gap-3`. Typography uses existing `text-body` / `text-small` utilities so the Tailwind-4 cascade stays consistent. Evaluation uses `line-clamp-3`.

Interaction map:
- single-click name/evaluation → inline edit (existing behavior preserved)
- single-click priority/category badge → existing `Popover`
- double-click empty card area → open detail dialog (event stopPropagation on interactive children)
- Ghostty / Finder / delete icons → always visible at 50% opacity, full opacity on hover

### 3. ProjectDetailDialog

```
┌─────────────────────────────────────────────────┐
│ PickAtSoul                              [✕]    │
├─────────────────────────────────────────────────┤
│ [name / priority / category / path / evaluation] │
│ [Save]  [Cancel]                                 │
├─────────────────────────────────────────────────┤
│ 📝 연결 메모 (3)                    [+ 새 메모] │
│  #7  스토어 UI 수정                [edit][del]  │
│  #12 결제 연동 테스트              [edit][del]  │
│  #18 OG 이미지 교체                [edit][del]  │
└─────────────────────────────────────────────────┘
```

Top half reuses `NewProjectDialog`'s field components in an edit variant (extract the repeated `<label>` + `<input>` blocks into a `ProjectFormFields` subcomponent so both dialogs share markup). Bottom half:
- Renders memos filtered to `project_id === project.id` from the existing `useMemos()` store, ordered by global `sort_order`.
- `+ 새 메모` inserts an empty memo with `project_id` preset and focuses the inline textarea.
- Edit uses the existing MemoCard inline editing flow (textarea on click).
- Delete prompts via `ask` from `@tauri-apps/plugin-dialog`.

### 4. MemoBoard — grouped rendering + #N badges

Ordering rule (derived on the client, no backend change):

```ts
const groups = [
  ...projects
    .sort((a, b) => PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority]
                    || a.sort_order - b.sort_order)
    .map(p => ({ kind: "project", project: p, memos: byProject.get(p.id) ?? [] })),
  { kind: "etc", memos: memos.filter(m => m.project_id === null) },
].filter(g => g.memos.length > 0);
```

Global badge map:

```ts
const seq = new Map(
  [...memos].sort((a, b) => a.sort_order - b.sort_order).map((m, i) => [m.id, i + 1])
);
```

MemoCard receives `sequenceNumber` as a prop and renders a corner badge (`absolute top-2 right-2 rounded-full bg-black/20 text-white px-1.5 text-[10px]`).

Within-group drag reuses the existing `reorder_memos` call. Dragging across group boundaries triggers `update_memo({ project_id: targetProjectId | null })` plus a reorder in both groups.

### 5. UI zoom hook

```ts
const STEPS = [0.85, 1.0, 1.15, 1.3] as const;
const DEFAULT = 1.0;

export function useUiScale() {
  const [scale, setScale] = useState<number>(DEFAULT);

  useEffect(() => {
    api.getUiScale().then(v => apply(v ?? DEFAULT));
  }, []);

  const apply = (s: number) => {
    document.documentElement.style.zoom = String(s);
    setScale(s);
    api.setUiScale(s);
  };

  const bump = (dir: 1 | -1) => {
    const i = STEPS.indexOf(scale as (typeof STEPS)[number]);
    const next = Math.max(0, Math.min(STEPS.length - 1, (i === -1 ? 1 : i) + dir));
    apply(STEPS[next]);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === "=" || e.key === "+") { e.preventDefault(); bump(1); }
      else if (e.key === "-") { e.preventDefault(); bump(-1); }
      else if (e.key === "0") { e.preventDefault(); apply(DEFAULT); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [scale]);

  return { scale };
}
```

Invoked once from `App.tsx`. Settings round-trip via two new Tauri commands hitting `settings` table key `ui_scale`.

### 6. AI memo tools

Three new `ToolSpec` entries in `ai_tools.rs`:

- `create_memo(content, project_name?)` — `Mutation`. When `project_name` is provided, resolve to a `project_id` via `LIKE '%name%'` match (first hit); when unresolved or null, memo remains unassigned ("기타"). The agent response must mention any fallback.
- `update_memo_by_number(number, content)` — `Mutation`. Resolves `number` to the memo id via `ORDER BY sort_order LIMIT 1 OFFSET number-1`.
- `delete_memo_by_number(number)` — `Mutation`. Same resolution as update.

`buildSystemPrompt.ts` gets a new `[메모]` section explaining the `#N` convention so the agent reliably maps natural language to the correct tool call.

## Error handling

| Failure | Response |
| --- | --- |
| `update_project` fails during drag | toast `우선순위 변경 실패`; next `get_projects` refetch restores server state |
| `update_memo_by_number` with N out of range | Rust returns `Err(format!("#{} 메모를 찾을 수 없음", n))` → agent relays the error |
| `create_memo(project_name=X)` with no match | Memo created unassigned; agent notes the fallback |
| `set_ui_scale` fails | Silently ignored — the scale still applies for the current session |
| Detail dialog open when underlying project is deleted | Dialog closes; toast `프로젝트가 삭제되었습니다` |

## Testing

### Frontend (Vitest — introduce with this change)

- `deriveTarget(over)` — card id, empty-zone id, null
- `globalSequence(memos)` — ensures #N matches sort order and is stable across updates
- `useUiScale.bump` — upper bound, lower bound, reset

### Backend (`src-tauri/tests/`)

Extend the existing integration-test harness.

- `update_memo_by_number`: happy path, OFFSET beyond range
- `create_memo(project_name)`: exact match, partial LIKE match, no match (null fallback)
- `set_ui_scale` → `get_ui_scale` round-trip

### Manual verification checklist

- [ ] Drag a P0 card into the middle of P2, confirm DB `priority` + `sort_order` via `sqlite3`
- [ ] `Cmd+=` × 3, `Cmd+-` × 2, `Cmd+0` — scale matches expected step
- [ ] Double-click card → edit in modal → save → card reflects change
- [ ] Modal: add three memos → confirm they appear in MemoBoard's matching project group
- [ ] `"WithGenieLMS 에 '심사 통과' 메모 추가"` through the ⌘K palette → confirm → row appears in correct group
- [ ] `"#5 메모 삭제"` through the palette → confirm → row removed and subsequent badges renumber

## Scope

**In**
- 2-col detailed card project view
- Cross-priority drag with precise insertion
- Double-click detail dialog with memo CRUD
- Memo grouping by project + "기타" group
- Global `#N` memo badges
- Cross-group memo drag
- Three AI memo tools + system prompt update
- `Cmd+=` / `Cmd+-` / `Cmd+0` with persistence

**Out (YAGNI)**
- Manual drag reordering of memo groups themselves (priority auto-ordering only)
- Arbitrary zoom steps (4 fixed steps)
- Archived project state
- Schedule linkage in the detail modal (`schedules` has no `project_id`)
- View toggle (list vs card) — card becomes the single view

## Risks

- **CSS `zoom` + react-big-calendar**: calendar pixel math may misalign at non-100% scale. Mitigation: verify manually, fall back to `transform: scale()` with layout container sizing if needed.
- **dnd-kit collision detection across multiple sortable lists**: empty-zone targeting can be flaky with `closestCenter`. Mitigation: try `pointerWithin` first, fall back to `closestCorners`.
- **Global `#N` stability under concurrent edits**: badges shift when memos are added/deleted. Mitigation: document that `#N` is the current-snapshot address; the agent should read memos fresh before emitting an update_by_number call (the loop already passes recent state in context).
