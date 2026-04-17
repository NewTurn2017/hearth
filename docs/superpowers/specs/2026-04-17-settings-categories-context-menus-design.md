# Settings Modal + Category CRUD + Context Menus + New Memo Dialog

**Date:** 2026-04-17
**Status:** Accepted (design approved; plan to follow)

## Goal

Ship four tightly related UX upgrades in one coordinated drop:

1. **Custom right-click context menus** on project and memo cards, with the native WebKit menu (including "Inspect Element") suppressed globally.
2. **New Memo dialog** that lets the user type content, pick a project, and choose a color before the row lands in the database — replacing the current "create an empty yellow sticky and hope the user fills it in" flow.
3. **User-editable categories** (add, rename with cascade to all using projects, delete only when unused). Seeded with the current five presets so nothing regresses for existing users.
4. **Unified Settings modal** — tabbed `AI` / `백업` / `카테고리`. The AI-only dialog is retired; backup location becomes a persisted preference; category CRUD lives here.

## Non-Goals

- FK migration on `projects.category`. We keep the string column and UPDATE in a transaction on rename.
- Inline "+ 카테고리" button in the sidebar filter list. Settings modal is the single edit surface.
- Dev-mode "Inspect" toggle. The native menu is fully suppressed; devtools remains reachable via the normal keyboard shortcut / Tauri hotkey.
- Memo multi-select or bulk actions in the context menu.

## Architecture Overview

```
┌────────── Frontend ──────────┐        ┌────────── Backend (Rust) ──────────┐
│  SettingsDialog  ─ tabs      │        │  cmd_categories.rs  [NEW]          │
│    ├─ AI   (ported from      │        │    get_categories                  │
│    │       AiSettingsDialog) │        │    create_category                 │
│    ├─ 백업                    │───────▶│    update_category (rename CASCADE)│
│    │    GET/SET backup_dir   │        │    delete_category (reject if used)│
│    │    backup_db, list      │        │                                    │
│    └─ 카테고리                │        │  cmd_backup.rs  [MODIFY]           │
│         useCategories()      │        │    backup_dir() reads settings kv  │
│                              │        │    list_backups reads configured   │
│  NewMemoDialog  [NEW]        │        │    get_backup_dir / set_backup_dir │
│  ContextMenu primitive [NEW] │        │                                    │
│  useContextMenu hook  [NEW]  │        │  cmd_settings.rs  [MODIFY]         │
│  useCategories hook   [NEW]  │        │    K_BACKUP_DIR constant + helpers │
│                              │        │                                    │
│  ProjectCard  — onContextMenu│        │  db.rs  [MODIFY]                   │
│  MemoCard     — onContextMenu│        │    create categories table +       │
│  MemoBoard    — new dialog   │        │    seed Active/Side/Lab/Tools/     │
│  Layout       — global       │        │    Lecture on first run            │
│                 contextmenu  │        │                                    │
│                 blocker +    │        │                                    │
│                 SettingsDlg  │        │                                    │
└──────────────────────────────┘        └────────────────────────────────────┘
```

## Data Model

### New table: `categories`

```sql
CREATE TABLE categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    color      TEXT    NOT NULL DEFAULT '#6b7280',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    DEFAULT (datetime('now')),
    updated_at TEXT    DEFAULT (datetime('now'))
);
```

Seeded on first run (migration idempotent — only inserts when the table is empty):

| name    | color    | sort_order |
|---------|----------|------------|
| Active  | #22c55e  | 0          |
| Side    | #f97316  | 1          |
| Lab     | #a855f7  | 2          |
| Tools   | #6b7280  | 3          |
| Lecture | #3b82f6  | 4          |

`projects.category` stays `TEXT` (no FK). Rename is propagated via single UPDATE:

```sql
BEGIN;
UPDATE categories SET name = :new WHERE id = :id;
UPDATE projects   SET category = :new WHERE category = :old;
COMMIT;
```

Delete refuses with a Korean error when the count of projects using that name is > 0.

### New settings KV entry

`settings` table gains one row:

| key              | value (example)                                                    |
|------------------|--------------------------------------------------------------------|
| `backup.dir`     | `/Users/genie/Library/Application Support/com.newturn2017.hearth/backups` |

Missing row → fall back to `app_data_dir/backups` (current behavior).

## Backend Commands

### `cmd_categories.rs` (new)

```rust
pub struct Category {
    id: i64,
    name: String,
    color: String,
    sort_order: i64,
    usage_count: i64,   // derived: (SELECT COUNT(*) FROM projects WHERE category = name)
    created_at: String,
    updated_at: String,
}

get_categories() -> Vec<Category>             // ORDER BY sort_order, id; includes usage_count
create_category({ name, color? }) -> Category // rejects duplicate name
update_category(id, fields)                   // fields: name?, color?, sort_order?
                                              //   - if name changes: transaction with
                                              //     UPDATE projects cascade
                                              //   - new name must not collide
                                              //   - returns the updated Category
delete_category(id)                           // Err("카테고리 사용 중 (N개 프로젝트)") if
                                              // usage_count > 0
reorder_categories(ids)                       // persists sort_order = index for each id
```

Registered in `lib.rs` `generate_handler!` list.

### `cmd_backup.rs` (modify)

- New commands `get_backup_dir() -> String` and `set_backup_dir(path)` stored under `backup.dir`.
- Private helper `backup_dir(app)` now reads the settings KV first; falls back to `app_data_dir/backups`.
- `backup_db` and `list_backups` continue to call `backup_dir(app)`, so switching locations is transparent.
- `set_backup_dir` creates the directory if missing and stores the canonicalized path.

## Frontend

### New primitives

**`src/ui/ContextMenu.tsx`**

Props: `{ open, x, y, items, onClose }` — items: `{ id, label, icon?, danger?, disabled?, onSelect }[]`. Renders via `createPortal(document.body)` at `position: fixed`, with viewport-edge clamping (`x = min(x, innerWidth - panelW - 8)`), outside-click + Escape dismiss, and a short dismiss delay after `onSelect` so the click lands.

**`src/hooks/useContextMenu.ts`**

```ts
const { menu, open, close } = useContextMenu();
// ...
<div onContextMenu={open}>…</div>
<ContextMenu {...menu} items={…} onClose={close} />
```

`open(e)` calls `e.preventDefault()` + `e.stopPropagation()` and stores `{x: e.clientX, y: e.clientY, open: true}`.

**`src/hooks/useCategories.ts`**

Mirrors `useMemos` — loads from `api.getCategories()`, listens to a `categories:changed` event, exposes `{ categories, create, rename, recolor, remove, reorder, reload }`. All mutations dispatch the event after success so every other subscriber (Sidebar, ProjectCard popovers, ProjectFormFields) refetches.

### Global right-click blocker

`Layout.tsx` adds a one-line effect:

```ts
useEffect(() => {
  const block = (e: MouseEvent) => e.preventDefault();
  document.addEventListener("contextmenu", block);
  return () => document.removeEventListener("contextmenu", block);
}, []);
```

Cards that want their own menu call `e.stopPropagation()` in their own handler after setting menu state (the bubble up would otherwise hit the block). Devtools stays available via the keyboard shortcut.

### NewMemoDialog

`src/components/NewMemoDialog.tsx`. Dialog (reusing `ui/Dialog`) with:

- Title: "새 메모".
- `content` — textarea, autoFocus, min-height `120px`.
- `project_id` — native `<select>` whose options are built by iterating `PRIORITIES`, emitting one `<optgroup label="P0 — 긴급">` per priority, then `<option value="">프로젝트 없음 (기타)</option>` first. Uses `useProjects` + `useCategories` (actually only projects + a stable set of PRIORITIES).
- `color` — segmented row of 5 color swatches reusing `MEMO_COLORS`.
- Footer: `취소` / `추가`. `추가` disabled until `content.trim()` is non-empty.
- On submit → `api.createMemo({ content, color, project_id })` → `memos:changed` dispatch → close.
- Optional prop `defaultProjectId?: number | null` — pre-selects the dropdown when opened from a context we already know (e.g. future ProjectDetailDialog "연결 메모 추가" shortcut).

Integration:

- `MemoBoard.handleCreate` opens the dialog instead of creating an empty memo.
- `Layout` hosts a single `NewMemoDialog` instance, exposes `openNewMemo` to the command palette and (via a `window.dispatchEvent('memo:new-dialog')`) to MemoBoard so both entry points converge on the same modal.

### Context menus

**ProjectCard** — right-click opens menu with:
1. `프로젝트 설정` (cog icon) → `onOpenDetail(project)` (same as double-click today).
2. `Ghostty에서 열기` (play) — hidden if `!project.path`.
3. `Finder에서 열기` (folder) — hidden if `!project.path`.
4. `──` (separator).
5. `삭제` (trash, danger) → `onDelete(project.id)`.

**MemoCard** — right-click opens menu with:
1. `편집` (pencil) → `setEditing(true)`.
2. `색상 변경` (palette) → submenu with the five MEMO_COLORS swatches (implemented as a nested fly-out inside the same menu, not a real system submenu — simple Radix-style hover or click-through).
3. `프로젝트 이동` (folder-move) → submenu rendering the same priority-grouped project list as `NewMemoDialog` plus "기타 (연결 해제)". Selecting dispatches `onUpdate(id, { project_id: picked ?? null })` — `null` detaches via the existing `Option<Option<i64>>` shape in `UpdateMemoInput` (the `project_id = 0` idiom only exists on the AI tool path, not on this command).
4. `──`.
5. `삭제` (trash, danger) → `onDelete(id)`.

### SettingsDialog

`src/components/SettingsDialog.tsx` — replaces `AiSettingsDialog`. Props `{ open, onClose }`. Uses a simple state-driven tab bar (no routing) with three tabs:

- **AI** — `<AiSettingsSection />` extracted verbatim from today's `AiSettingsDialog` body (provider select + OpenAI key + save). Save still dispatches `ai-settings:changed`.
- **백업** — backup location row (path + `변경…` opens `dialog.open({ directory: true })`), `지금 백업` button, and the five-item recent backups list with per-row `복원` (with an `ask` confirm, since restore overwrites the live DB).
- **카테고리** — drag-sortable list of category rows. Each row: color swatch (opens popover with 10 preset colors + custom HEX input), name inline-edit (blur commits rename), `✕` delete (disabled + tooltip when `usage_count > 0` — the field ships with `get_categories`). Footer: `+ 카테고리 추가` that appends an editable new row bound to `create_category`.

### Sidebar / ProjectFormFields / ProjectCard updates

Anywhere that currently imports `CATEGORIES` or `CATEGORY_COLORS` from `types.ts`:

- `Sidebar.tsx` category filter now reads from `useCategories()`.
- `ProjectFormFields.tsx` category `<select>` iterates `useCategories().categories`.
- `ProjectCard.tsx` category popover iterates `useCategories().categories`; color lookup replaces the static `CATEGORY_COLORS[category]` with `categories.find(c => c.name === project.category)?.color ?? defaultColor`.

The static `CATEGORIES` / `CATEGORY_COLORS` constants stay in `types.ts` as the seed list for backend migration and as a last-resort fallback in the color lookup above.

## Event Plumbing

| Event                   | Dispatched by                          | Consumed by                         |
|-------------------------|----------------------------------------|-------------------------------------|
| `categories:changed`    | category CRUD handlers                 | `useCategories`                     |
| `memos:changed`         | NewMemoDialog, MemoCard context menu   | `useMemos`                          |
| `projects:changed`      | ProjectCard context menu (delete)      | `useProjects`                       |
| `ai-settings:changed`   | Settings AI tab save                   | `useAiStatus`                       |
| `backup:changed`        | Settings 백업 tab save/backup/restore  | Future listeners (none today)       |

## Testing

Backend (Rust integration tests):

- `cmd_categories`:
  - `create_category` rejects duplicate name.
  - `update_category` rename cascades project rows and rolls back on duplicate-target-name.
  - `delete_category` refuses when a project references the name.
  - Seeding is idempotent (second run doesn't duplicate rows).
- `cmd_backup`:
  - `set_backup_dir` creates missing directories.
  - `backup_db` without `dest_path` writes into the configured directory.
  - `list_backups` reflects the configured directory.

Frontend (Vitest):

- `useContextMenu` — `open(e)` calls preventDefault and stores coords; close resets.
- `ContextMenu` clamps panel x/y inside the viewport (jsdom mocked `innerWidth`).
- `NewMemoDialog` — submit disabled on empty content, calls `api.createMemo` with selected project+color, clears on close.
- Categories helpers if any pure helper is extracted.

Manual:

- Right-click on project card → custom menu appears; 프로젝트 설정 opens dialog.
- Right-click on memo card → custom menu appears; 색상/프로젝트 이동 submenu works.
- Right-click elsewhere → nothing (no Inspect).
- Settings modal tab switching preserves unsaved input per tab.
- Rename "Active" → all existing Active-tagged projects show the new name in sidebar + cards.
- Delete a category in use → toast error "사용 중".
- Change backup location → backup file lands there; list shows entries from new dir only.
- New Memo dialog from MemoBoard + Cmd+K both open the same modal.

## Risks / Open Questions

- **Submenu UX** — nested menus inside a context menu are finicky. The plan's default: a flat color-swatch row shown inline inside the same menu (no nested flyout), and "프로젝트 이동" opens a small picker dialog rather than an in-menu flyout. Keeps interactions predictable and avoids hover-timing bugs.
- **Static fallback** — if a renamed category points to a name the UI's fallback doesn't know, the color falls back to neutral gray until the `useCategories` fetch lands. Acceptable for the few milliseconds between dispatch and fetch.
- **Devtools accessibility** — suppressing `contextmenu` globally removes the only discovery path for "Inspect Element". The keyboard shortcut still works (Cmd+Opt+I on macOS), but a doc note in the README would help.
