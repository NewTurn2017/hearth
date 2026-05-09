# Focus Memo Board + Memo Styling + Tags

**Date:** 2026-05-06
**Status:** Accepted direction; implementation plan to follow after review
**Branch/worktree:** `feat/memo-emphasis-categories` at `.worktrees/memo-emphasis-categories`

## Goal

Make Hearth memos feel closer to real notes placed on a monitor: visually obvious, movable, and easy to filter by work context. The first version adds a third memo view named **Focus**, lightweight memo emphasis controls, and memo-specific tags while reusing existing project categories for cross-cutting filtering.

The target experience is simple and tactile: a user can capture several notes, mark the important ones as large/bold/colorful, and arrange them on a monitor-like board without turning Hearth into a complex drawing canvas.

## User-Facing Outcomes

1. **Important memos stand out.** Users can set a memo to small, normal, or large text; toggle bold emphasis; and keep the existing color choices.
2. **A third memo view reduces empty-space awkwardness.** In addition to List and Matrix, **Focus** presents a monitor-like board with a left filter rail and a free-placement area.
3. **Memos can be organized beyond projects.** Existing project categories remain useful for filtering, and new memo tags support labels such as `검토`, `아이디어`, `대기`, `회의`, and `중요`.
4. **CLI and agent skill behavior stay aligned with the app.** Memo style and tags are available from the `hearth` CLI and reflected in the single `skills/hearth/SKILL.md` router.

## Non-Goals

- Freeform resizing with arbitrary width/height handles.
- Rotation, zooming, pan/viewport persistence, or infinite canvas behavior.
- Rich text editing, markdown formatting, checklists, image embeds, or nested blocks.
- A second independent “memo category” system that competes with existing project categories.
- Bulk tag editing or multi-select operations in the first version.
- Replacing List or Matrix view. Focus is an additional view.

## Core Decisions

| Decision | Value | Why |
|---|---|---|
| View name | `Focus` | Short, clear, and matches the “important notes on my monitor” concept. |
| Layout direction | Hybrid board | Left rail gives control; right board gives playful free placement. |
| MVP interaction | Position + style | Delivers the requested fun and emphasis without arbitrary canvas complexity. |
| Memo text size | `small`, `normal`, `large` | Explicit three-step model requested by the user; easy to expose in UI and CLI. |
| Emphasis | Boolean bold | Clear visual signal without introducing a full formatting model. |
| Existing category reuse | Project categories are filter context for memos | Avoids duplicating the current category model and keeps project/memo context connected. |
| New memo organization | Tags, not categories | Tags can be many-to-many and do not conflict with project category semantics. |
| Later canvas features | Defer rotation/resizing/zoom | These are fun but add accessibility, persistence, and interaction complexity. |

## Current Repository Anchors

- `src/components/MemoBoard.tsx` owns the current memo view switch and supports `list | matrix`.
- `src/components/MemoCard.tsx` renders draggable card memos and owns the memo context menu.
- `src/components/MemoRow.tsx` renders compact Matrix rows and shares color/project movement actions.
- `src/components/MemoMatrix.tsx` groups memos by project.
- `src/hooks/useMemos.ts` is the frontend memo store and listens to `memos:changed`.
- `src/hooks/useCategories.ts` already manages user-editable project categories.
- `src-tauri/core/src/memos.rs` owns memo CRUD and currently persists `content`, `color`, `project_id`, and `sort_order`.
- `src-tauri/core/src/db.rs` owns idempotent SQLite migrations.
- `src-tauri/cli/src/cmd/memo.rs` exposes memo list/get/create/update/delete.
- `src-tauri/cli/src/cmd/category.rs` already exposes project category management.
- `skills/hearth/SKILL.md` is the single agent skill router and must remain the only exposed Hearth skill.

## UX Design

### MemoBoard view switch

The memo header view switch becomes:

- `List`
- `Matrix`
- `Focus`

The selected view continues to be persisted in `localStorage` under the existing memo-board view preference key or a compatible successor. Unknown old values fall back to `list`.

### Focus layout

Focus uses a two-pane layout:

```text
┌─────────────────────────────────────────────────────────────┐
│ 메모보드                                      [List Matrix Focus] [+] │
├───────────────┬─────────────────────────────────────────────┤
│ Filter rail   │ Monitor board                               │
│               │                                             │
│ Quick filters │   ┌ sticky memo ┐      ┌ sticky memo ┐      │
│ - 전체         │   │ 큰 + 굵게    │      │ 작게         │      │
│ - 중요         │   └─────────────┘      └─────────────┘      │
│ - 미연결       │                                             │
│               │          ┌ sticky memo ┐                    │
│ Categories    │          │ 태그 포함     │                    │
│ - Active      │          └─────────────┘                    │
│ - Lab         │                                             │
│               │                                             │
│ Tags          │                                             │
│ - 검토         │                                             │
│ - 아이디어     │                                             │
└───────────────┴─────────────────────────────────────────────┘
```

The board is visually framed like a monitor or screen, but it remains plain HTML/CSS. SVG-like decoration may be used for the screen outline, subtle guide lines, or glow, but memo positioning should remain DOM-based for accessibility and drag support.

### Left filter rail

The first version supports:

- Quick filters:
  - `전체`
  - `중요` — memos with `is_bold = true`, `font_size = large`, or tag `중요`
  - `미연결` — memos with no project
- Project category filters:
  - Existing `categories` rows from `useCategories()`
  - A memo matches a project category when its linked project belongs to that category
- Memo tag filters:
  - New memo tags, ordered by usage count then name or explicit sort order

Filter behavior is additive within a group only when the UI explicitly supports it. For MVP, keep selection simple: one project category filter and one memo tag filter at a time, plus one quick filter.

### Board placement

Each memo has a Focus position:

- `focus_x`: normalized number from `0` to `1`
- `focus_y`: normalized number from `0` to `1`

Normalized coordinates make the board resilient to window resizing and UI scale changes. The frontend converts normalized values to absolute CSS positions inside the board.

If a memo has no stored position, Focus places it in a deterministic default cascade so new or migrated memos appear usable immediately:

```text
x = 0.08 + (index % 4) * 0.21
y = 0.10 + floor(index / 4) * 0.18
```

Values are clamped so cards stay inside the board.

### Drag behavior

- Dragging a note updates the local UI immediately.
- On drag end, the app persists `focus_x` and `focus_y` through the memo update command.
- If persistence fails, the UI reloads from the database and shows a toast error.
- Keyboard-only placement can be a later improvement; the context menu and List/Matrix views remain accessible fallback surfaces in MVP.

### Memo styling controls

Memo context menus in Card, Row, and Focus surfaces expose consistent controls:

- `글씨 크기`
  - `작게`
  - `일반`
  - `크게`
- `강조`
  - `굵게 표시` toggle
- `색상 변경`
  - Existing `MEMO_COLORS`
- `태그`
  - Add/remove memo tags through a small tag picker or inline menu section

The first implementation keeps tag editing simple: show existing tags, allow selecting from known tags, and allow creating one new tag by typing in a dialog/popover. It does not include a full tag-management settings screen in MVP.

### Visual style

Focus should preserve Hearth’s current dark, amber-accented visual system:

- Board background: dark monitor-like surface with subtle border/glow.
- Notes: existing pastel memo colors.
- Large/bold notes: visually obvious but not cartoonish.
- Left rail: compact, readable, and similar to Sidebar/Settings control density.

## Data Model

### Extend `memos`

```sql
ALTER TABLE memos ADD COLUMN font_size TEXT NOT NULL DEFAULT 'normal';
ALTER TABLE memos ADD COLUMN is_bold INTEGER NOT NULL DEFAULT 0;
ALTER TABLE memos ADD COLUMN focus_x REAL;
ALTER TABLE memos ADD COLUMN focus_y REAL;
```

Valid `font_size` values:

- `small`
- `normal`
- `large`

Validation belongs in Rust core update/create paths, not only in the frontend.

### New table: `memo_tags`

```sql
CREATE TABLE IF NOT EXISTS memo_tags (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    color      TEXT    NOT NULL DEFAULT '#6b7280',
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
);
```

### New table: `memo_tag_links`

```sql
CREATE TABLE IF NOT EXISTS memo_tag_links (
    memo_id INTEGER NOT NULL REFERENCES memos(id) ON DELETE CASCADE,
    tag_id  INTEGER NOT NULL REFERENCES memo_tags(id) ON DELETE CASCADE,
    PRIMARY KEY (memo_id, tag_id)
);
```

The many-to-many model lets one memo be both `검토` and `중요` without overloading the project category field.

### Seed tags

On first run only, seed a small Korean-first tag set:

| name | color | reason |
|---|---|---|
| 중요 | `#ef4444` | Common quick filter and visual priority. |
| 검토 | `#f59e0b` | Review/waiting-for-decision memos. |
| 아이디어 | `#a855f7` | Brainstorming notes. |
| 대기 | `#64748b` | Blocked or waiting items. |
| 회의 | `#0ea5e9` | Meeting-related memos. |

## TypeScript Types

`src/types.ts` adds:

```ts
export type MemoFontSize = "small" | "normal" | "large";

export interface MemoTag {
  id: number;
  name: string;
  color: string;
  sort_order: number;
  usage_count: number;
  created_at: string;
  updated_at: string;
}

export interface Memo {
  id: number;
  content: string;
  color: string;
  project_id: number | null;
  sort_order: number;
  font_size: MemoFontSize;
  is_bold: boolean;
  focus_x: number | null;
  focus_y: number | null;
  tags: MemoTag[];
  created_at: string;
  updated_at: string;
}
```

Return nested tags in `get_memos` for MVP. If this later becomes too large, a follow-up can split `get_memos` and `get_memo_tags_by_memo`; the first implementation should prioritize a simple frontend contract.

## Backend API

### Memo create/update inputs

`NewMemo` gains:

- `font_size: Option<&str>` with default `normal`
- `is_bold: Option<bool>` with default `false`
- `focus_x: Option<f64>`
- `focus_y: Option<f64>`
- `tag_names: Vec<&str>` or equivalent owned strings at command boundary

`UpdateMemo` gains:

- `font_size: Option<&str>`
- `is_bold: Option<bool>`
- `focus_x: Option<Option<f64>>`
- `focus_y: Option<Option<f64>>`
- `tag_names: Option<Vec<String>>` replacing the full tag set when present

For the frontend Tauri command, prefer an explicit replace model for tags:

```ts
updateMemo(id, { tag_names: ["검토", "중요"] })
```

This is easier to keep idempotent from UI state than separate add/remove calls.

### New memo tag commands

Add Rust/Tauri commands:

- `get_memo_tags() -> Vec<MemoTag>`
- `create_memo_tag(input: { name, color? }) -> MemoTag`
- `update_memo_tag(id, fields: { name?, color?, sort_order? }) -> MemoTag`
- `delete_memo_tag(id) -> void`
- `reorder_memo_tags(ids) -> void`

Deletion should remove only the tag and links, not the memo. If the tag is in use, deletion can still succeed because tag links are not primary content. A confirmation UI can be added later if needed.

## Frontend Components

### `MemoBoard.tsx`

- Extend local view state to `"list" | "matrix" | "focus"`.
- Add a `Focus` tab with a suitable icon.
- Compute memo tags and project category filter data once and pass to `FocusMemoBoard`.

### New `FocusMemoBoard.tsx`

Responsibilities:

- Render filter rail.
- Render monitor-like board.
- Render positioned memo notes.
- Own drag-end coordinate conversion.
- Call `onUpdate(memo.id, { focus_x, focus_y })` on drop.
- Apply active quick/category/tag filters.

### New `FocusMemoNote.tsx`

Responsibilities:

- Render a memo with color, font size, bold state, tags, and linked project label.
- Reuse the same context-menu item builders as `MemoCard` where possible.
- Support drag within the Focus board.

### Shared memo action helper

To avoid duplicating context-menu logic across `MemoCard`, `MemoRow`, and `FocusMemoNote`, introduce a small helper such as:

```ts
buildMemoMenuItems({ memo, projects, tags, onUpdate, onDelete, openProjectPicker, openTagPicker })
```

This should remain a helper, not a large abstraction layer. The purpose is consistency, not framework building.

### New hook: `useMemoTags`

Mirrors `useCategories`:

- Loads `api.getMemoTags()`.
- Listens to `memo-tags:changed`.
- Exposes create, rename, recolor, remove, reorder, reload.
- Mutations dispatch `memo-tags:changed` and, when links can change memo display, `memos:changed`.

## CLI Design

Extend `hearth memo` commands.

### Create

```bash
hearth memo create "기술 검토" \
  --color yellow \
  --project 40 \
  --size large \
  --bold \
  --tag 검토 \
  --tag 중요
```

### Update

```bash
hearth memo update 24 --size normal --bold false
hearth memo update 24 --tag 검토 --tag 대기
hearth memo update 24 --clear-tags
hearth memo update 24 --focus-x 0.42 --focus-y 0.18
```

Rules:

- `--size` accepts only `small|normal|large`.
- `--bold` accepts a bool for update; create can use a flag.
- Repeating `--tag` replaces the memo's full tag set unless paired with future `tag add/remove` subcommands.
- `--clear-tags` conflicts with `--tag`.
- Focus coordinates are clamped to `0.0..=1.0` in core.

### New tag subcommand

Add:

```bash
hearth memo-tag list
hearth memo-tag create 중요 --color "#ef4444"
hearth memo-tag update 1 --name 긴급검토 --color "#f97316"
hearth memo-tag delete 1
```

A separate `memo-tag` subcommand avoids overloading the existing project `category` commands.

## Agent Skill Update

`skills/hearth/SKILL.md` remains the only exposed skill. It gains routing rules for:

- Creating a styled memo:
  - “크게 표시해줘” → `--size large`
  - “진하게 표시해줘” → `--bold`
  - “중요 태그 달아줘” → `--tag 중요`
- Updating style:
  - `hearth memo update <id> --size <small|normal|large> --bold <true|false>`
- Updating tags:
  - Read phase: `hearth memo list` and `hearth memo-tag list`
  - Mutation recipe: `hearth memo update <id> --tag ...`

The existing propose → approve → apply gate remains mandatory for all mutating skill actions.

## Data Flow

```text
User action in Focus board
  ↓
FocusMemoBoard drag/style/tag handler
  ↓
api.updateMemo(...)
  ↓
Tauri command
  ↓
hearth_core::memos::update
  ↓
SQLite memos + memo_tag_links
  ↓
audit_log entry
  ↓
Frontend local state update + memos:changed bridge for external writers
```

External CLI writes continue to be picked up by the existing `PRAGMA data_version` bridge, which dispatches `memos:changed`. Tag-table writes should also dispatch or map to the same refresh event when they affect visible memo rendering.

## Migration and Backward Compatibility

- Existing memos receive `font_size = normal` and `is_bold = 0`.
- Existing memos have no Focus coordinates and are auto-laid out until moved.
- Existing memo colors remain unchanged.
- Existing project categories remain unchanged.
- Existing `hearth memo create` and `hearth memo update` commands continue to work without new flags.
- Existing exports/imports must include new memo fields and memo tag tables.
- Audit undo/redo should preserve new memo fields and tag link changes for CLI-originated updates.

## Testing Strategy

### Frontend tests

- `MemoBoard` persists and restores `focus` view.
- Focus filter rail filters by:
  - quick filter `중요`
  - linked project category
  - memo tag
- `FocusMemoNote` applies text size and bold classes.
- Drag-end coordinate conversion clamps values between `0` and `1`.
- Context menu updates style fields through `onUpdate`.

### Rust core tests

- Migration adds memo style/position columns idempotently.
- `memos::create` defaults `font_size` and `is_bold`.
- `memos::update` rejects invalid size values.
- Focus coordinates are clamped or rejected consistently.
- Memo tag create/list/update/delete works.
- Updating a memo's tag set creates missing tags by name, then replaces the full memo tag set with the requested names.
- Export/import round-trips memo tags and style fields.

### CLI tests

- `hearth memo create "x" --size large --bold --tag 중요` returns the expected JSON.
- `hearth memo update <id> --size small --bold false` updates style.
- `hearth memo update <id> --clear-tags` removes links.
- `hearth memo-tag list/create/update/delete` works against a temp DB.
- Existing smoke-skill recipes still pass.

### Manual smoke

- Open Hearth, create several memos, switch to Focus.
- Drag memos around, switch away, return to Focus, verify positions persist.
- Change size/bold/color from the context menu and verify List/Matrix remain readable.
- Use CLI to add a tag while the app is open and verify the UI refreshes.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Focus becomes a complex canvas project | Restrict MVP to normalized x/y, fixed size presets, no rotation/resizing/zoom. |
| Context-menu code duplicates across memo surfaces | Extract small shared menu-builder helpers only where duplication becomes real. |
| Tags and project categories confuse users | Label rail sections explicitly: `프로젝트 카테고리` and `메모 태그`. |
| Coordinates look bad after resize | Store normalized values and clamp rendered positions. |
| CLI flags drift from skill docs | Update CLI docs, `skills/hearth/SKILL.md`, and `scripts/smoke-skills.sh` together. |
| Export/import misses new tables | Include export/import in implementation plan and tests, not as a follow-up. |

## Implementation Phases

### Phase 1 — Data model and core contracts

- Extend memo model with style/position fields.
- Add memo tag tables and core CRUD.
- Update export/import and audit behavior.
- Add Rust tests.

### Phase 2 — CLI and skill surface

- Extend `hearth memo` flags.
- Add `hearth memo-tag` commands.
- Update `skills/hearth/SKILL.md` routing.
- Update skill smoke tests and CLI docs.

### Phase 3 — Frontend Focus view

- Add `Focus` view switch.
- Add `FocusMemoBoard`, `FocusMemoNote`, and `useMemoTags`.
- Add style/tag context-menu controls.
- Add targeted frontend tests.

### Phase 4 — Visual polish and verification

- Tune monitor-like board styling.
- Verify List/Matrix remain unchanged for existing users.
- Run full test/build checks.

## Acceptance Criteria

- Users can switch to `Focus` and see memos on a monitor-like board.
- Users can drag a memo in Focus, restart or refresh, and see its position preserved.
- Users can set memo text size to small, normal, or large.
- Users can toggle bold emphasis.
- Users can assign memo tags and filter by those tags in Focus.
- Users can filter Focus by existing project categories via linked projects.
- CLI can create/update memo style, tags, and Focus position.
- The single Hearth skill documents and uses the new CLI recipes through the existing approval gate.
- Existing List and Matrix views continue to display all memos correctly.
- Existing tests pass, and new tests cover the behavior above.

## Open Follow-Up Ideas

These are intentionally outside MVP but compatible with this design:

- Manual note resize handles.
- Slight rotation presets for a more physical sticky-note feel.
- Saved Focus board presets per project category.
- Keyboard nudging for selected Focus notes.
- Dedicated tag management tab in Settings.
- Mini-map or zoom controls if the board grows beyond one screen.
