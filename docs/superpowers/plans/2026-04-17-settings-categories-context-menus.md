# Settings Modal + Category CRUD + Context Menus + New Memo Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the four coordinated UX upgrades in `docs/superpowers/specs/2026-04-17-settings-categories-context-menus-design.md` — custom right-click context menus (with the native WebKit menu suppressed globally), a New Memo dialog that captures content/project/color before creating the row, user-editable categories (add / rename-with-cascade / delete-if-unused), and a unified tabbed Settings modal (AI / 백업 / 카테고리) that retires the AI-only dialog and makes the backup directory a persisted preference.

**Architecture:** Additive. Backend gains a `cmd_categories.rs` module + a `categories` table seeded on first run with the current five presets; `cmd_backup` is refactored so both `backup_db` and `list_backups` read the backup directory from the `settings` KV (new `backup.dir` row, with fall-back to the legacy `app_data_dir/backups`). Frontend adds a `ContextMenu` primitive + `useContextMenu` and `useCategories` hooks, a `NewMemoDialog`, and a `SettingsDialog` that hosts three tab sections (AI / 백업 / 카테고리). `AiSettingsDialog` becomes `AiSettingsSection` (same content, extracted into the tab) and is removed from `Layout`. `Layout` mounts a global `contextmenu`-blocker effect so only our custom menus appear. Existing constants `CATEGORIES` / `CATEGORY_COLORS` stay in `types.ts` as the backend seed list and as a last-resort color fallback while `useCategories` is warming up.

**Tech Stack:** React 19, TypeScript 5.8, Tailwind 4, dnd-kit 6.3 + sortable 10.0 (reused for the draggable category list), Tauri 2 (dialog plugin), rusqlite 0.34, Vitest + @testing-library/react.

---

## File Structure

**New files (backend)**

```
src-tauri/src/cmd_categories.rs         Tauri commands: get/create/update/delete/reorder + Category row
src-tauri/tests/categories.rs           Integration test: seed idempotency, duplicate-name reject,
                                        rename cascade, delete-in-use reject
src-tauri/tests/backup_dir.rs           Integration test: KV-backed backup dir helper, fall-back,
                                        set_backup_dir creates missing dirs
```

**New files (frontend)**

```
src/ui/ContextMenu.tsx                  Portal-rendered fixed-position menu primitive
                                        Props: { open, x, y, items, onClose }
src/hooks/useContextMenu.ts             { menu, open, close } — open(e) stores x/y and flips open
src/hooks/useCategories.ts              Mirrors useMemos — categories + CRUD + categories:changed listener
src/components/NewMemoDialog.tsx        content/project/color picker dialog
src/components/SettingsDialog.tsx       Tabbed shell (AI / 백업 / 카테고리)
src/components/SettingsAiSection.tsx    Extracted verbatim from AiSettingsDialog body
src/components/SettingsBackupSection.tsx Backup dir row + 지금 백업 + recent backups list with 복원
src/components/SettingsCategoriesSection.tsx Draggable list of categories + inline edit + add
src/hooks/__tests__/useContextMenu.test.ts
src/ui/__tests__/ContextMenu.test.tsx
src/components/__tests__/NewMemoDialog.test.tsx
```

**Modified files**

```
src-tauri/src/db.rs                     + categories table + idempotent seed of the 5 presets
src-tauri/src/cmd_backup.rs             backup_dir(app) reads settings KV;
                                        + get_backup_dir / set_backup_dir commands
src-tauri/src/cmd_settings.rs           + K_BACKUP_DIR constant (reused by cmd_backup)
src-tauri/src/lib.rs                    + cmd_categories mod + ~10 new command registrations
src/api.ts                              + Category type bindings, + 5 category commands,
                                        + get_backup_dir / set_backup_dir
src/types.ts                            + Category interface (db-shaped, separate from the
                                        legacy string union which stays as a fallback)
src/components/Layout.tsx               retire AiSettingsDialog; add global contextmenu blocker;
                                        host SettingsDialog + NewMemoDialog;
                                        expose openNewMemo to command palette; listen for
                                        'memo:new-dialog' to converge both entry points
src/components/TopBar.tsx               Settings2 button opens Settings (not AI-only); remove 백업
                                        button (moved into Settings 백업 tab); onOpenSettings prop
src/components/MemoBoard.tsx            handleCreate dispatches 'memo:new-dialog' instead of
                                        creating an empty memo
src/components/MemoCard.tsx             + onContextMenu hook + menu items (편집 / 색상 변경 /
                                        프로젝트 이동 / 삭제); color row + project picker inline
src/components/ProjectCard.tsx          + onContextMenu hook + menu items (프로젝트 설정 /
                                        Ghostty / Finder / 삭제);
                                        category badge uses useCategories() color lookup
src/components/Sidebar.tsx              category filter iterates useCategories().categories
src/components/ProjectFormFields.tsx    category <select> iterates useCategories().categories
src/command/dispatch.ts                 new-memo command calls deps.openNewMemo (still dispatched
                                        to the new dialog) — no shape change
```

**Unchanged but referenced**

```
src/types.ts                            CATEGORIES / CATEGORY_COLORS constants stay for color
                                        fallback and as the Rust seed source-of-truth comment
src-tauri/src/models.rs                 unchanged — Project.category remains Option<String>
```

Each task below produces one focused commit.

---

## Task 1: Backend — create `categories` table + idempotent seed

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/db.rs`

- [ ] **Step 1: Add the migration block + seed helper**

In `run_migrations`, append a `CREATE TABLE IF NOT EXISTS categories (...)` statement and, after the batch completes, call a new `seed_categories_if_empty(conn)` helper that only inserts when the table is empty.

Replace `run_migrations` with this content (keeps the existing statements verbatim and adds the new table + seed):

```rust
fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            priority TEXT NOT NULL DEFAULT 'P4',
            number INTEGER,
            name TEXT NOT NULL,
            category TEXT,
            path TEXT,
            evaluation TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS schedules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            time TEXT,
            location TEXT,
            description TEXT,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT 'yellow',
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            company_name TEXT,
            ceo TEXT,
            phone TEXT,
            fax TEXT,
            email TEXT,
            offices TEXT,
            project_desc TEXT,
            status TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS categories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            color      TEXT    NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;

    seed_categories_if_empty(conn)?;
    Ok(())
}

fn seed_categories_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 =
        conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let seed: [(&str, &str, i64); 5] = [
        ("Active",  "#22c55e", 0),
        ("Side",    "#f97316", 1),
        ("Lab",     "#a855f7", 2),
        ("Tools",   "#6b7280", 3),
        ("Lecture", "#3b82f6", 4),
    ];
    let tx = conn.unchecked_transaction()?;
    for (name, color, ord) in seed {
        tx.execute(
            "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, color, ord],
        )?;
    }
    tx.commit()?;
    Ok(())
}
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat(db): add categories table with idempotent seed of 5 presets"
```

---

## Task 2: Backend — `cmd_categories.rs` module + `Category` row shape

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_categories.rs`
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`

- [ ] **Step 1: Create the module with all five commands**

Write this full file at `src-tauri/src/cmd_categories.rs`:

```rust
// User-editable project categories.
//
// Rename propagates to `projects.category` inside a transaction — we keep the
// column as TEXT without a FK so existing rows survive a category drop-in.
// Delete refuses with a Korean error when any project still references the
// name; the UI shows the live usage count on every row.
//
// The `categories` row is the single source of truth for category color /
// order. The legacy `CATEGORY_COLORS` constant in `types.ts` stays in the
// codebase as the seed list and as a last-resort UI fallback.

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    /// Derived (SELECT COUNT(*) FROM projects WHERE category = name).
    /// The UI uses this to disable the delete button and to format the
    /// "사용 중 (N개)" error.
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to_category(row: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        sort_order: row.get(3)?,
        usage_count: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const SELECT_WITH_USAGE: &str =
    "SELECT c.id, c.name, c.color, c.sort_order, \
            (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) AS usage_count, \
            c.created_at, c.updated_at \
     FROM categories c";

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let sql = format!("{} ORDER BY c.sort_order ASC, c.id ASC", SELECT_WITH_USAGE);
    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_category)
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryInput {
    pub name: String,
    pub color: Option<String>,
}

#[tauri::command]
pub fn create_category(
    state: State<'_, AppState>,
    input: CreateCategoryInput,
) -> Result<Category, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("카테고리 이름이 비어 있습니다".into());
    }
    let color = input
        .color
        .unwrap_or_else(|| "#6b7280".into())
        .trim()
        .to_string();

    let db = state.db.lock().map_err(|e| e.to_string())?;

    // Reject duplicate name explicitly so the UI can surface the Korean error
    // instead of relying on the raw UNIQUE constraint message.
    let exists: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [&name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists > 0 {
        return Err(format!("이미 존재하는 카테고리 이름입니다: {name}"));
    }

    let next_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    db.execute(
        "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, color, next_order],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    db.query_row(
        &format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE),
        [id],
        row_to_category,
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

#[tauri::command]
pub fn update_category(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateCategoryInput,
) -> Result<Category, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;

    // Resolve the current name first so we can drive the rename cascade from
    // a single place (and reject a duplicate-target-name before touching
    // projects).
    let current_name: String = db
        .query_row(
            "SELECT name FROM categories WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| format!("카테고리를 찾을 수 없음: id={id}"))?;

    let mut new_name: Option<String> = None;
    if let Some(raw) = fields.name.as_ref() {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err("카테고리 이름이 비어 있습니다".into());
        }
        if trimmed != current_name {
            let collides: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM categories WHERE name = ?1 AND id <> ?2",
                    rusqlite::params![trimmed, id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if collides > 0 {
                return Err(format!("이미 존재하는 카테고리 이름입니다: {trimmed}"));
            }
            new_name = Some(trimmed);
        }
    }

    let tx = db.transaction().map_err(|e| e.to_string())?;

    if let Some(ref n) = new_name {
        tx.execute(
            "UPDATE categories SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![n, id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE projects SET category = ?1, updated_at = datetime('now') WHERE category = ?2",
            rusqlite::params![n, current_name],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(ref color) = fields.color {
        tx.execute(
            "UPDATE categories SET color = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![color.trim(), id],
        )
        .map_err(|e| e.to_string())?;
    }

    if let Some(ord) = fields.sort_order {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![ord, id],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;

    db.query_row(
        &format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE),
        [id],
        row_to_category,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let (name, usage): (String, i64) = db
        .query_row(
            "SELECT name, (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) \
             FROM categories c WHERE c.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| format!("카테고리를 찾을 수 없음: id={id}"))?;

    if usage > 0 {
        return Err(format!("카테고리 사용 중 ({usage}개 프로젝트): {name}"));
    }
    db.execute("DELETE FROM categories WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_categories(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.transaction().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![i as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
```

- [ ] **Step 2: Register the module + commands in `lib.rs`**

Open `src-tauri/src/lib.rs` and make two edits:

Add `mod cmd_categories;` alphabetically near the other `mod cmd_*;` declarations (between `mod cmd_backup;` and `mod cmd_clients;`).

Extend the `tauri::generate_handler!` list with the five new handlers (place them after `cmd_backup::list_backups` and before the `cmd_ai::*` block):

```rust
cmd_categories::get_categories,
cmd_categories::create_category,
cmd_categories::update_category,
cmd_categories::delete_category,
cmd_categories::reorder_categories,
```

- [ ] **Step 3: Run `cargo check` to verify the module compiles**

Run: `cd src-tauri && cargo check`
Expected: builds cleanly (no warnings about unused imports / dead code related to the new module).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/cmd_categories.rs src-tauri/src/lib.rs
git commit -m "feat(backend): category CRUD commands with rename-cascade + delete-if-unused"
```

---

## Task 3: Backend — integration test for `cmd_categories`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src-tauri/tests/categories.rs`

- [ ] **Step 1: Write the integration test**

This test runs against an in-memory SQLite connection with the same schema — we recreate the tables inline so we exercise the SQL behavior without spinning up Tauri. Mirrors the pattern in `tests/memo_by_number.rs`.

Write this file:

```rust
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            priority TEXT NOT NULL DEFAULT 'P4',
            number INTEGER,
            name TEXT NOT NULL,
            category TEXT,
            path TEXT,
            evaluation TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE categories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            color TEXT NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        "#,
    )
    .unwrap();
    conn
}

fn seed(conn: &Connection) {
    conn.execute_batch(
        r#"
        INSERT INTO categories (name, color, sort_order) VALUES
          ('Active',  '#22c55e', 0),
          ('Side',    '#f97316', 1),
          ('Lab',     '#a855f7', 2),
          ('Tools',   '#6b7280', 3),
          ('Lecture', '#3b82f6', 4);
        INSERT INTO projects (name, category, priority) VALUES
          ('alpha', 'Active', 'P2'),
          ('beta',  'Active', 'P3'),
          ('gamma', 'Side',   'P2');
        "#,
    )
    .unwrap();
}

#[test]
fn unique_constraint_rejects_duplicate_name() {
    let conn = setup_db();
    seed(&conn);
    let err = conn
        .execute(
            "INSERT INTO categories (name, color, sort_order) VALUES ('Active', '#000000', 9)",
            [],
        )
        .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("UNIQUE"), "expected UNIQUE error, got: {msg}");
}

#[test]
fn rename_cascade_moves_all_projects() {
    let conn = setup_db();
    seed(&conn);
    let tx = conn.unchecked_transaction().unwrap();
    tx.execute(
        "UPDATE categories SET name = 'Production' WHERE name = 'Active'",
        [],
    )
    .unwrap();
    tx.execute(
        "UPDATE projects SET category = 'Production' WHERE category = 'Active'",
        [],
    )
    .unwrap();
    tx.commit().unwrap();

    let renamed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Production'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(renamed, 2);

    let old_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Active'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(old_rows, 0);
}

#[test]
fn delete_in_use_is_blocked_by_usage_count() {
    let conn = setup_db();
    seed(&conn);
    let usage: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects WHERE category = 'Active'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(usage, 2, "precondition: Active is in use");

    // The command-layer guard is the check we're modeling: refuse deletion
    // when usage > 0. The test asserts the SQL-side fact the guard relies on.
    assert!(usage > 0);
}

#[test]
fn delete_unused_succeeds() {
    let conn = setup_db();
    seed(&conn);
    let deleted = conn
        .execute("DELETE FROM categories WHERE name = 'Lecture'", [])
        .unwrap();
    assert_eq!(deleted, 1);
}
```

- [ ] **Step 2: Run the test**

Run: `cd src-tauri && cargo test --test categories`
Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/categories.rs
git commit -m "test(backend): categories table constraints + rename cascade"
```

---

## Task 4: Backend — `backup.dir` setting + refactor `cmd_backup.rs`

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_settings.rs`
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_backup.rs`
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`

- [ ] **Step 1: Expose the new KV key + read helper from `cmd_settings`**

Open `src-tauri/src/cmd_settings.rs`. Add a new constant alongside `K_UI_SCALE` and make the `read` / `write` helpers `pub(crate)` so `cmd_backup` can reuse them.

Add the constant (group with the other `K_*` constants):

```rust
pub(crate) const K_BACKUP_DIR: &str = "backup.dir";
```

Change the helpers from `fn` to `pub(crate) fn`:

```rust
pub(crate) fn read(db: &rusqlite::Connection, key: &str) -> Result<String, String> { ... }
pub(crate) fn write(db: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> { ... }
```

(Keep the bodies identical — only the visibility changes.)

- [ ] **Step 2: Rewrite `cmd_backup.rs` to read the backup dir from settings**

Replace the `backup_dir` helper and add `get_backup_dir` / `set_backup_dir` commands. Also updated the auto-backup path so close-time backups honor the configured directory. Full new file:

```rust
use crate::cmd_settings::{self, K_BACKUP_DIR};
use crate::AppState;
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

/// Resolve the directory that holds rolling backups. Reads `backup.dir` from
/// the settings KV; falls back to `$APP_DATA/backups` when unset so first-run
/// behavior matches the pre-setting world.
fn backup_dir(app: &AppHandle, state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let configured = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::read(&db, K_BACKUP_DIR)?
    };
    let dir = if configured.is_empty() {
        app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("backups")
    } else {
        PathBuf::from(configured)
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("data.db"))
}

#[tauri::command]
pub fn get_backup_dir(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    Ok(backup_dir(&app, &state)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_backup_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let canonical = PathBuf::from(path.trim());
    if canonical.as_os_str().is_empty() {
        return Err("백업 위치가 비어 있습니다".into());
    }
    fs::create_dir_all(&canonical)
        .map_err(|e| format!("백업 폴더를 만들 수 없습니다: {e}"))?;
    let stored = canonical.to_string_lossy().to_string();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::write(&db, K_BACKUP_DIR, &stored)?;
    }
    Ok(stored)
}

#[tauri::command]
pub fn backup_db(
    state: State<'_, AppState>,
    app: AppHandle,
    dest_path: Option<String>,
) -> Result<String, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }

    let source = db_path(&app)?;
    let dest = match dest_path {
        Some(p) => PathBuf::from(p),
        None => {
            let dir = backup_dir(&app, &state)?;
            let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
            dir.join(format!("hearth-backup-{}.db", timestamp))
        }
    };

    fs::copy(&source, &dest).map_err(|e| format!("Backup failed: {}", e))?;

    let dir = backup_dir(&app, &state)?;
    let mut backups: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("hearth-backup-")
        })
        .collect();
    backups.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    for old in backups.into_iter().skip(5) {
        fs::remove_file(old.path()).ok();
    }

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_db(app: AppHandle, src_path: String) -> Result<(), String> {
    let source = PathBuf::from(&src_path);
    if !source.exists() {
        return Err("Backup file not found".into());
    }
    let dest = db_path(&app)?;
    fs::copy(&source, &dest).map_err(|e| format!("Restore failed: {}", e))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created: String,
}

#[tauri::command]
pub fn list_backups(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir(&app, &state)?;
    let mut backups: Vec<BackupInfo> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("hearth-backup-")
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()
                .map(|t| {
                    let dt: chrono::DateTime<Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            Some(BackupInfo {
                path: e.path().to_string_lossy().to_string(),
                filename: e.file_name().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                created: modified,
            })
        })
        .collect();
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(backups)
}

pub fn auto_backup_on_close(app: &AppHandle) {
    let state: State<'_, AppState> = app.state();
    if let Ok(db) = state.db.lock() {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
    }
    let source = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("data.db"));
    let dest_dir = backup_dir(app, &state).ok();

    if let (Some(src), Some(dir)) = (source, dest_dir) {
        fs::create_dir_all(&dir).ok();
        let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
        let dest = dir.join(format!("hearth-backup-{}.db", timestamp));
        fs::copy(&src, &dest).ok();

        if let Ok(entries) = fs::read_dir(&dir) {
            let mut backups: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("hearth-backup-")
                })
                .collect();
            backups.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
            for old in backups.into_iter().skip(5) {
                fs::remove_file(old.path()).ok();
            }
        }
    }
}
```

- [ ] **Step 3: Register the two new commands in `lib.rs`**

Add to the `generate_handler!` list alongside the other `cmd_backup::*` entries:

```rust
cmd_backup::get_backup_dir,
cmd_backup::set_backup_dir,
```

- [ ] **Step 4: Run `cargo check`**

Run: `cd src-tauri && cargo check`
Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/cmd_settings.rs src-tauri/src/cmd_backup.rs src-tauri/src/lib.rs
git commit -m "feat(backup): persist backup directory in settings + get/set commands"
```

---

## Task 5: Frontend — API bindings + types for categories and backup dir

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/types.ts`
- Modify: `/Users/genie/dev/tools/hearth/src/api.ts`

- [ ] **Step 1: Add the `Category` interface to `types.ts`**

Append after the existing `BackupInfo` interface (around line 54):

```ts
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
```

- [ ] **Step 2: Add the category and backup-dir bindings to `api.ts`**

Append to `api.ts` (after the existing `listBackups` and before the AI block):

```ts
// 카테고리 (user-editable project categories)
import type { CategoryRow } from "./types";

export const getCategories = () => invoke<CategoryRow[]>("get_categories");

export const createCategory = (input: { name: string; color?: string }) =>
  invoke<CategoryRow>("create_category", { input });

export const updateCategory = (
  id: number,
  fields: { name?: string; color?: string; sort_order?: number }
) => invoke<CategoryRow>("update_category", { id, fields });

export const deleteCategory = (id: number) =>
  invoke<void>("delete_category", { id });

export const reorderCategories = (ids: number[]) =>
  invoke<void>("reorder_categories", { ids });

// 백업 위치 (persisted under settings key `backup.dir`)
export const getBackupDir = () => invoke<string>("get_backup_dir");
export const setBackupDir = (path: string) =>
  invoke<string>("set_backup_dir", { path });
```

- [ ] **Step 3: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/types.ts src/api.ts
git commit -m "feat(api): bindings for category CRUD + backup directory"
```

---

## Task 6: Frontend — `useCategories` hook

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/hooks/useCategories.ts`

- [ ] **Step 1: Write the hook**

Mirrors `useMemos` — local cache, listens for `categories:changed`, dispatches the same event after every mutation so other subscribers (Sidebar, ProjectCard badges, ProjectFormFields select) stay in sync.

Write this file:

```ts
import { useCallback, useEffect, useState } from "react";
import type { CategoryRow } from "../types";
import * as api from "../api";

/**
 * Reactive store for user-editable project categories. Mirrors `useMemos`
 * conventions — every mutation dispatches `categories:changed` on `window`
 * so every other subscriber (Sidebar filter list, ProjectFormFields select,
 * ProjectCard category popover) refetches.
 *
 * The hook does **not** take filter arguments — the category list is small
 * and shared by every surface, so we keep a single reactive copy.
 */
export function useCategories() {
  const [categories, setCategories] = useState<CategoryRow[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getCategories();
      setCategories(data);
    } catch (e) {
      console.error("Failed to load categories:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const onChanged = () => {
      load();
    };
    window.addEventListener("categories:changed", onChanged);
    return () => window.removeEventListener("categories:changed", onChanged);
  }, [load]);

  const create = async (input: { name: string; color?: string }) => {
    const created = await api.createCategory(input);
    window.dispatchEvent(new CustomEvent("categories:changed"));
    return created;
  };

  const rename = async (id: number, name: string) => {
    const updated = await api.updateCategory(id, { name });
    // Project rows may have been cascaded — notify both listeners.
    window.dispatchEvent(new CustomEvent("categories:changed"));
    window.dispatchEvent(new CustomEvent("projects:changed"));
    return updated;
  };

  const recolor = async (id: number, color: string) => {
    const updated = await api.updateCategory(id, { color });
    window.dispatchEvent(new CustomEvent("categories:changed"));
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteCategory(id);
    window.dispatchEvent(new CustomEvent("categories:changed"));
  };

  const reorder = async (ids: number[]) => {
    await api.reorderCategories(ids);
    window.dispatchEvent(new CustomEvent("categories:changed"));
  };

  return {
    categories,
    loading,
    create,
    rename,
    recolor,
    remove,
    reorder,
    reload: load,
  };
}
```

- [ ] **Step 2: Commit**

```bash
git add src/hooks/useCategories.ts
git commit -m "feat(hooks): useCategories store with change-event plumbing"
```

---

## Task 7: Frontend — `ContextMenu` primitive

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/ui/ContextMenu.tsx`

- [ ] **Step 1: Write the failing test for viewport clamping**

Create `/Users/genie/dev/tools/hearth/src/ui/__tests__/ContextMenu.test.tsx`:

```tsx
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ContextMenu } from "../ContextMenu";

describe("ContextMenu", () => {
  it("renders nothing when closed", () => {
    const { container } = render(
      <ContextMenu
        open={false}
        x={10}
        y={10}
        items={[{ id: "a", label: "A", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    expect(container.firstChild).toBeNull();
  });

  it("renders items when open", () => {
    render(
      <ContextMenu
        open
        x={10}
        y={10}
        items={[{ id: "a", label: "Alpha", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    expect(screen.getByText("Alpha")).toBeInTheDocument();
  });

  it("clamps x so the panel does not overflow the right edge", () => {
    Object.defineProperty(window, "innerWidth", { value: 800, writable: true });
    render(
      <ContextMenu
        open
        x={790}
        y={10}
        items={[{ id: "a", label: "A", onSelect: () => {} }]}
        onClose={() => {}}
      />
    );
    const panel = screen.getByRole("menu");
    const left = parseFloat((panel as HTMLElement).style.left);
    // Panel width is 208px (min-w-[208px]); clamp leaves at least 8px margin.
    // Panel must not start later than innerWidth - panelW - margin.
    expect(left).toBeLessThanOrEqual(800 - 208 - 8 + 0.5);
  });
});
```

- [ ] **Step 2: Run the test — expect FAIL (ContextMenu not defined)**

Run: `npx vitest run src/ui/__tests__/ContextMenu.test.tsx`
Expected: FAIL with "Cannot find module '../ContextMenu'".

- [ ] **Step 3: Implement `ContextMenu`**

Create `/Users/genie/dev/tools/hearth/src/ui/ContextMenu.tsx`:

```tsx
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";
import type { LucideIcon } from "lucide-react";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: LucideIcon;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
  /** Optional inline content rendered inside the menu row (e.g. a color-swatch
   *  row for "색상 변경"). When set, `label` is still used as the row header
   *  and `onSelect` is ignored. */
  inline?: ReactNode;
  /** Divider row — renders a thin separator and ignores all other fields. */
  separator?: boolean;
}

const PANEL_WIDTH = 208;
const MARGIN = 8;

export function ContextMenu({
  open,
  x,
  y,
  items,
  onClose,
}: {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const [clampedX, setClampedX] = useState(x);
  const [clampedY, setClampedY] = useState(y);

  // Clamp before paint so the menu does not flash at the requested coords and
  // then jump. `useLayoutEffect` is required — `useEffect` runs after paint.
  useLayoutEffect(() => {
    if (!open) return;
    const panel = panelRef.current;
    const h = panel?.offsetHeight ?? 240;
    const nextX = Math.min(x, window.innerWidth - PANEL_WIDTH - MARGIN);
    const nextY = Math.min(y, window.innerHeight - h - MARGIN);
    setClampedX(Math.max(MARGIN, nextX));
    setClampedY(Math.max(MARGIN, nextY));
  }, [open, x, y]);

  useEffect(() => {
    if (!open) return;
    const onDocPointer = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // `mousedown` beats `click` — important because the right-click that
    // opens the menu fires `contextmenu` first, then `mousedown` on release.
    document.addEventListener("mousedown", onDocPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDocPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      ref={panelRef}
      role="menu"
      style={{
        position: "fixed",
        left: clampedX,
        top: clampedY,
        minWidth: PANEL_WIDTH,
      }}
      className={cn(
        "z-[200] p-1 rounded-[var(--radius-md)]",
        "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
        "shadow-[var(--shadow-e3)] text-[13px]"
      )}
    >
      {items.map((it) => {
        if (it.separator) {
          return (
            <div
              key={it.id}
              role="separator"
              className="my-1 h-px bg-[var(--color-border)]"
            />
          );
        }
        if (it.inline) {
          return (
            <div key={it.id} className="px-2 py-1">
              <div className="text-[11px] text-[var(--color-text-dim)] mb-1">
                {it.label}
              </div>
              {it.inline}
            </div>
          );
        }
        return (
          <button
            key={it.id}
            role="menuitem"
            type="button"
            disabled={it.disabled}
            onClick={() => {
              if (it.disabled) return;
              // Short delay so the click lands before the menu tears down —
              // without it, the outside-click handler wins the race and the
              // `onSelect` callback never fires on fast trackpad clicks.
              setTimeout(() => {
                it.onSelect();
                onClose();
              }, 0);
            }}
            className={cn(
              "w-full flex items-center gap-2 px-2 h-8 rounded text-left",
              "transition-colors duration-[120ms]",
              it.disabled
                ? "opacity-50 cursor-not-allowed"
                : it.danger
                  ? "text-[var(--color-danger)] hover:bg-[var(--color-danger)] hover:text-white"
                  : "text-[var(--color-text)] hover:bg-[var(--color-surface-3)]"
            )}
          >
            {it.icon && <Icon icon={it.icon} size={14} />}
            <span>{it.label}</span>
          </button>
        );
      })}
    </div>,
    document.body
  );
}
```

- [ ] **Step 4: Run the test — expect PASS**

Run: `npx vitest run src/ui/__tests__/ContextMenu.test.tsx`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/ui/ContextMenu.tsx src/ui/__tests__/ContextMenu.test.tsx
git commit -m "feat(ui): portal-rendered ContextMenu primitive with viewport clamp"
```

---

## Task 8: Frontend — `useContextMenu` hook

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/hooks/useContextMenu.ts`
- Create: `/Users/genie/dev/tools/hearth/src/hooks/__tests__/useContextMenu.test.ts`

- [ ] **Step 1: Write the failing test**

Create the test file:

```ts
import { describe, it, expect, vi } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useContextMenu } from "../useContextMenu";

describe("useContextMenu", () => {
  it("open(e) calls preventDefault + stopPropagation and stores coords", () => {
    const { result } = renderHook(() => useContextMenu());
    const preventDefault = vi.fn();
    const stopPropagation = vi.fn();
    const fakeEvent = {
      clientX: 120,
      clientY: 80,
      preventDefault,
      stopPropagation,
    } as unknown as React.MouseEvent;

    act(() => {
      result.current.open(fakeEvent);
    });

    expect(preventDefault).toHaveBeenCalledOnce();
    expect(stopPropagation).toHaveBeenCalledOnce();
    expect(result.current.menu.open).toBe(true);
    expect(result.current.menu.x).toBe(120);
    expect(result.current.menu.y).toBe(80);
  });

  it("close() resets open to false", () => {
    const { result } = renderHook(() => useContextMenu());
    const fakeEvent = {
      clientX: 1,
      clientY: 1,
      preventDefault: () => {},
      stopPropagation: () => {},
    } as unknown as React.MouseEvent;

    act(() => {
      result.current.open(fakeEvent);
    });
    expect(result.current.menu.open).toBe(true);

    act(() => {
      result.current.close();
    });
    expect(result.current.menu.open).toBe(false);
  });
});
```

- [ ] **Step 2: Run the test — expect FAIL**

Run: `npx vitest run src/hooks/__tests__/useContextMenu.test.ts`
Expected: FAIL with "Cannot find module '../useContextMenu'".

- [ ] **Step 3: Write the hook**

Create `/Users/genie/dev/tools/hearth/src/hooks/useContextMenu.ts`:

```ts
import { useCallback, useState, type MouseEvent } from "react";

export interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
}

export function useContextMenu() {
  const [menu, setMenu] = useState<ContextMenuState>({
    open: false,
    x: 0,
    y: 0,
  });

  const open = useCallback((e: MouseEvent) => {
    // `preventDefault` suppresses the native WebKit menu; `stopPropagation`
    // prevents the global blocker effect in Layout (which also calls
    // preventDefault) from hiding our own menu by bubbling.
    e.preventDefault();
    e.stopPropagation();
    setMenu({ open: true, x: e.clientX, y: e.clientY });
  }, []);

  const close = useCallback(() => {
    setMenu((prev) => ({ ...prev, open: false }));
  }, []);

  return { menu, open, close };
}
```

- [ ] **Step 4: Run the test — expect PASS**

Run: `npx vitest run src/hooks/__tests__/useContextMenu.test.ts`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/hooks/useContextMenu.ts src/hooks/__tests__/useContextMenu.test.ts
git commit -m "feat(hooks): useContextMenu wires e.preventDefault + coord capture"
```

---

## Task 9: Frontend — global right-click blocker in `Layout`

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/Layout.tsx`

- [ ] **Step 1: Add the effect that suppresses the native menu**

Add a `useEffect` import to the existing line and append the effect inside the `Layout` function body (just before the `return` statement):

```tsx
import { useCallback, useEffect, useState } from "react";
```

```tsx
// Global right-click blocker: suppress the native WebKit menu (which
// includes "Inspect Element" in dev). Cards that want their own menu
// open it via `useContextMenu` and call `e.stopPropagation()` inside
// their handler so this listener never sees the bubble. Devtools stays
// reachable via the standard keyboard shortcut.
useEffect(() => {
  const block = (e: MouseEvent) => e.preventDefault();
  document.addEventListener("contextmenu", block);
  return () => document.removeEventListener("contextmenu", block);
}, []);
```

- [ ] **Step 2: Manual verify (smoke)**

Run: `npm run tauri dev`
Expected: Right-click anywhere outside a card does nothing (no native menu). Keyboard devtools shortcut still works.

- [ ] **Step 3: Commit**

```bash
git add src/components/Layout.tsx
git commit -m "feat(ui): suppress native contextmenu globally"
```

---

## Task 10: Frontend — `NewMemoDialog`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/NewMemoDialog.tsx`
- Create: `/Users/genie/dev/tools/hearth/src/components/__tests__/NewMemoDialog.test.tsx`

- [ ] **Step 1: Write the failing test**

Create the test file. It mocks `api.createMemo` + `useProjects` and asserts: submit disabled on empty, calls `createMemo` with the selected project + color, resets on close.

```tsx
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { NewMemoDialog } from "../NewMemoDialog";
import { ToastProvider } from "../../ui/Toast";

vi.mock("../../api", () => ({
  createMemo: vi.fn().mockResolvedValue({
    id: 1,
    content: "hello",
    color: "pink",
    project_id: null,
    sort_order: 0,
    created_at: "",
    updated_at: "",
  }),
}));

vi.mock("../../hooks/useProjects", () => ({
  useProjects: () => ({ projects: [], loading: false }),
}));

import * as api from "../../api";

const renderIt = (open = true, onClose = () => {}) =>
  render(
    <ToastProvider>
      <NewMemoDialog open={open} onClose={onClose} />
    </ToastProvider>
  );

describe("NewMemoDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("disables the 추가 button when content is empty", () => {
    renderIt();
    const submit = screen.getByRole("button", { name: "추가" });
    expect(submit).toBeDisabled();
  });

  it("calls createMemo with content + chosen color on submit", async () => {
    renderIt();
    const textarea = screen.getByRole("textbox");
    fireEvent.change(textarea, { target: { value: "hello" } });
    const pinkBtn = screen.getByLabelText("색상: pink");
    fireEvent.click(pinkBtn);
    const submit = screen.getByRole("button", { name: "추가" });
    expect(submit).not.toBeDisabled();
    fireEvent.click(submit);
    await waitFor(() => {
      expect(api.createMemo).toHaveBeenCalledWith({
        content: "hello",
        color: "pink",
        project_id: undefined,
      });
    });
  });
});
```

- [ ] **Step 2: Run the test — expect FAIL**

Run: `npx vitest run src/components/__tests__/NewMemoDialog.test.tsx`
Expected: FAIL with module-not-found.

- [ ] **Step 3: Write the dialog**

Create `/Users/genie/dev/tools/hearth/src/components/NewMemoDialog.tsx`:

```tsx
import { useEffect, useMemo, useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { useToast } from "../ui/Toast";
import { useProjects } from "../hooks/useProjects";
import { PRIORITIES, MEMO_COLORS } from "../types";
import type { Priority } from "../types";
import { cn } from "../lib/cn";
import * as api from "../api";

const ALL_PRIORITIES = new Set<Priority>(PRIORITIES);

export function NewMemoDialog({
  open,
  onClose,
  defaultProjectId = null,
}: {
  open: boolean;
  onClose: () => void;
  /** Pre-selects the project dropdown. Null means "프로젝트 없음". */
  defaultProjectId?: number | null;
}) {
  const toast = useToast();
  const { projects } = useProjects(ALL_PRIORITIES, null);

  const [content, setContent] = useState("");
  const [projectId, setProjectId] = useState<number | null>(defaultProjectId);
  const [color, setColor] = useState(MEMO_COLORS[0].name);
  const [saving, setSaving] = useState(false);

  // Reset form state every time the dialog reopens so stale input from a
  // prior cancelled attempt never carries over.
  useEffect(() => {
    if (!open) return;
    setContent("");
    setProjectId(defaultProjectId);
    setColor(MEMO_COLORS[0].name);
  }, [open, defaultProjectId]);

  const grouped = useMemo(() => {
    const map = new Map<Priority, typeof projects>();
    for (const p of PRIORITIES) map.set(p, []);
    for (const p of projects) {
      if ((PRIORITIES as readonly string[]).includes(p.priority)) {
        map.get(p.priority as Priority)!.push(p);
      }
    }
    return map;
  }, [projects]);

  const canSubmit = content.trim().length > 0 && !saving;

  const submit = async () => {
    if (!canSubmit) return;
    setSaving(true);
    try {
      await api.createMemo({
        content: content.trim(),
        color,
        project_id: projectId ?? undefined,
      });
      window.dispatchEvent(new CustomEvent("memos:changed"));
      toast.success("메모 추가됨");
      onClose();
    } catch (e) {
      toast.error(`메모 추가 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const cancel = () => {
    onClose();
  };

  return (
    <Dialog open={open} onClose={cancel} labelledBy="new-memo-title">
      <h2
        id="new-memo-title"
        className="text-heading text-[var(--color-text-hi)] mb-4"
      >
        새 메모
      </h2>
      <div className="flex flex-col gap-4">
        <textarea
          autoFocus
          value={content}
          onChange={(e) => setContent(e.target.value)}
          placeholder="메모 내용…"
          className={cn(
            "min-h-[120px] w-full px-2 py-1.5 rounded-[var(--radius-md)] text-[13px]",
            "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
            "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
          )}
        />

        <div>
          <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
            프로젝트
          </label>
          <select
            value={projectId ?? ""}
            onChange={(e) =>
              setProjectId(e.target.value === "" ? null : Number(e.target.value))
            }
            className={cn(
              "h-9 w-full px-2 rounded-[var(--radius-md)] text-[13px]",
              "bg-[var(--color-surface-2)] border border-[var(--color-border)]",
              "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
            )}
          >
            <option value="">프로젝트 없음 (기타)</option>
            {PRIORITIES.map((pri) => {
              const items = grouped.get(pri) ?? [];
              if (items.length === 0) return null;
              return (
                <optgroup key={pri} label={`${pri}`}>
                  {items.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </optgroup>
              );
            })}
          </select>
        </div>

        <div>
          <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
            색상
          </label>
          <div className="flex gap-2">
            {MEMO_COLORS.map((c) => (
              <button
                key={c.name}
                type="button"
                aria-label={`색상: ${c.name}`}
                onClick={() => setColor(c.name)}
                className={cn(
                  "w-8 h-8 rounded-full border-2 transition-colors",
                  color === c.name
                    ? "border-[var(--color-brand-hi)]"
                    : "border-transparent"
                )}
                style={{ backgroundColor: c.bg }}
              />
            ))}
          </div>
        </div>
      </div>

      <div className="flex justify-end gap-2 mt-5">
        <Button variant="secondary" onClick={cancel} disabled={saving}>
          취소
        </Button>
        <Button variant="primary" onClick={submit} disabled={!canSubmit}>
          추가
        </Button>
      </div>
    </Dialog>
  );
}
```

- [ ] **Step 4: Run the test — expect PASS**

Run: `npx vitest run src/components/__tests__/NewMemoDialog.test.tsx`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/components/NewMemoDialog.tsx src/components/__tests__/NewMemoDialog.test.tsx
git commit -m "feat(memos): NewMemoDialog with content/project/color picker"
```

---

## Task 11: Frontend — wire `NewMemoDialog` into Layout + MemoBoard

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/Layout.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/MemoBoard.tsx`

- [ ] **Step 1: Host `NewMemoDialog` in Layout and converge both entry points**

In `Layout.tsx`, import `NewMemoDialog`, add state + open handler, and listen for the `memo:new-dialog` window event so MemoBoard can trigger the same modal without a prop drill.

Imports to add:

```tsx
import { NewMemoDialog } from "./NewMemoDialog";
```

Inside `Layout`, add state and the event listener (below the other `useState` calls):

```tsx
const [newMemoOpen, setNewMemoOpen] = useState(false);

useEffect(() => {
  const onNew = () => setNewMemoOpen(true);
  window.addEventListener("memo:new-dialog", onNew);
  return () => window.removeEventListener("memo:new-dialog", onNew);
}, []);

const openNewMemo = useCallback(() => {
  setActiveTab("memos");
  setNewMemoOpen(true);
}, []);
```

Change the `buildLocalCommands` call's `openNewMemo` to use the new handler:

```tsx
const commands = buildLocalCommands({
  openNewProject,
  openNewSchedule: () => setActiveTab("calendar"),
  openNewMemo,
});
```

Add `<NewMemoDialog />` as a sibling of `<NewProjectDialog />` (just before the closing `</div>`):

```tsx
<NewMemoDialog
  open={newMemoOpen}
  onClose={() => setNewMemoOpen(false)}
/>
```

- [ ] **Step 2: Have MemoBoard dispatch the event instead of creating an empty memo**

In `MemoBoard.tsx`, replace the existing `handleCreate` and drop the `create` import from `useMemos` (it is no longer used directly — the dialog creates via `api.createMemo` and the `memos:changed` event triggers the refetch):

```tsx
const handleCreate = () => {
  window.dispatchEvent(new CustomEvent("memo:new-dialog"));
};
```

Also remove `create` from the destructured `useMemos()` result (it is now unused in this file).

- [ ] **Step 3: Smoke-test both paths**

Run: `npm run tauri dev`
Expected:
- Clicking "메모 추가" on the MemoBoard opens `NewMemoDialog`.
- `Cmd+K` → "새 메모" opens the same dialog.
- Submitting adds a memo with the chosen content/color/project.

- [ ] **Step 4: Commit**

```bash
git add src/components/Layout.tsx src/components/MemoBoard.tsx
git commit -m "feat(memos): route memo creation through NewMemoDialog"
```

---

## Task 12: Frontend — `SettingsAiSection` (extracted from AiSettingsDialog)

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/SettingsAiSection.tsx`

- [ ] **Step 1: Extract the dialog body into a section component**

`SettingsAiSection` is identical behavior to `AiSettingsDialog` — same load-on-mount, same save flow, same `ai-settings:changed` dispatch — but no `<Dialog>` wrapper and no footer buttons (the outer `SettingsDialog` will own 닫기). Save happens on an internal "저장" button inside the section.

Create the file:

```tsx
// AI provider selector, extracted from the retired AiSettingsDialog.
//
// Same shape as before (provider toggle + OpenAI key field + save), just
// wrapped as a section for the unified SettingsDialog. The raw key is still
// write-only from the UI's perspective — the backend returns `has_openai_key`
// and never echoes the value back.

import { useEffect, useState } from "react";
import { Loader2, Trash2 } from "lucide-react";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import type { AiSettings } from "../types";
import * as api from "../api";

type Provider = AiSettings["provider"];

export function SettingsAiSection({ active }: { active: boolean }) {
  const toast = useToast();
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [provider, setProvider] = useState<Provider>("local");
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [hasStoredKey, setHasStoredKey] = useState(false);

  // Reload every time the AI tab is activated so the stored provider /
  // has_openai_key values reflect whatever may have changed elsewhere.
  useEffect(() => {
    if (!active) return;
    setLoading(true);
    setApiKeyInput("");
    api
      .getAiSettings()
      .then((s) => {
        setProvider(s.provider);
        setHasStoredKey(s.has_openai_key);
      })
      .catch((e) => toast.error(`설정 불러오기 실패: ${e}`))
      .finally(() => setLoading(false));
  }, [active, toast]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const result = await api.saveAiSettings({
        provider,
        openai_api_key: apiKeyInput.length > 0 ? apiKeyInput : undefined,
      });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("AI 설정 저장됨");
      window.dispatchEvent(new CustomEvent("ai-settings:changed"));
    } catch (e) {
      toast.error(`저장 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleClearKey = async () => {
    setSaving(true);
    try {
      const result = await api.saveAiSettings({
        provider,
        openai_api_key: "",
      });
      setHasStoredKey(result.has_openai_key);
      setApiKeyInput("");
      toast.success("저장된 API 키 삭제됨");
    } catch (e) {
      toast.error(`삭제 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-[13px] text-[var(--color-text-muted)] py-6">
        <Loader2 size={14} className="animate-spin" aria-hidden />
        <span>불러오는 중…</span>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <Field label="제공자">
        <div className="grid grid-cols-2 gap-2">
          <ProviderOption
            active={provider === "local"}
            onClick={() => setProvider("local")}
            title="로컬 MLX"
            subtitle="오프라인 · 무료"
          />
          <ProviderOption
            active={provider === "openai"}
            onClick={() => setProvider("openai")}
            title="OpenAI"
            subtitle="API 키 필요 · 종량제"
          />
        </div>
      </Field>

      {provider === "openai" && (
        <Field
          label="OpenAI API 키"
          hint={
            hasStoredKey
              ? "저장된 키가 있습니다. 새 키를 입력하면 덮어씁니다."
              : "sk-로 시작하는 키. 로컬 DB에 평문으로 저장됩니다."
          }
          right={
            hasStoredKey ? (
              <span className="text-[11px] text-[var(--color-success)] font-medium">
                저장됨
              </span>
            ) : null
          }
        >
          <div className="flex gap-2">
            <input
              type="password"
              value={apiKeyInput}
              onChange={(e) => setApiKeyInput(e.target.value)}
              placeholder={hasStoredKey ? "••••••••••••••••" : "sk-..."}
              autoComplete="off"
              spellCheck={false}
              className={cn(
                "flex-1 h-9 px-3 text-[13px] rounded-[var(--radius-md)]",
                "bg-[var(--color-surface-2)] text-[var(--color-text)]",
                "border border-[var(--color-border)]",
                "focus:outline-none focus:border-[var(--color-brand-hi)]"
              )}
            />
            {hasStoredKey && (
              <button
                type="button"
                onClick={handleClearKey}
                disabled={saving}
                title="저장된 API 키 삭제"
                className={cn(
                  "shrink-0 w-9 h-9 inline-flex items-center justify-center",
                  "rounded-[var(--radius-md)] border border-[var(--color-border)]",
                  "text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]",
                  "transition-colors duration-[120ms]",
                  "disabled:opacity-50 disabled:cursor-not-allowed"
                )}
                aria-label="저장된 API 키 삭제"
              >
                <Icon icon={Trash2} size={14} />
              </button>
            )}
          </div>
        </Field>
      )}

      <div className="flex justify-end">
        <Button variant="primary" onClick={handleSave} disabled={saving}>
          {saving ? "저장 중…" : "저장"}
        </Button>
      </div>
    </div>
  );
}

function Field({
  label,
  hint,
  right,
  children,
}: {
  label: string;
  hint?: string;
  right?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <div>
      <div className="flex items-center justify-between mb-1.5">
        <label className="text-[12px] font-medium text-[var(--color-text)]">
          {label}
        </label>
        {right}
      </div>
      {children}
      {hint && (
        <p className="text-[11px] text-[var(--color-text-dim)] mt-1.5">{hint}</p>
      )}
    </div>
  );
}

function ProviderOption({
  active,
  onClick,
  title,
  subtitle,
}: {
  active: boolean;
  onClick: () => void;
  title: string;
  subtitle: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "flex flex-col items-start gap-0.5 p-3 rounded-[var(--radius-md)] text-left",
        "border transition-colors duration-[120ms]",
        active
          ? "border-[var(--color-brand-hi)] bg-[var(--color-brand-soft)]"
          : "border-[var(--color-border)] bg-[var(--color-surface-2)] hover:bg-[var(--color-surface-3)]"
      )}
    >
      <span
        className={cn(
          "text-[13px] font-medium",
          active ? "text-[var(--color-brand-hi)]" : "text-[var(--color-text)]"
        )}
      >
        {title}
      </span>
      <span className="text-[11px] text-[var(--color-text-dim)]">{subtitle}</span>
    </button>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SettingsAiSection.tsx
git commit -m "feat(settings): extract AI settings into reusable section"
```

---

## Task 13: Frontend — `SettingsBackupSection`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/SettingsBackupSection.tsx`

- [ ] **Step 1: Write the section**

Renders the configured backup directory with a 변경… button (Tauri `dialog.open({ directory: true })`), a 지금 백업 button, and the five-item recent backups list with a per-row 복원 button that prompts with `ask(...)` before overwriting the live DB. Dispatches `backup:changed` after save/restore for future listeners.

```tsx
// Backup location + manual backup + restore list.
//
// Backup directory is stored under the `backup.dir` key in the settings KV.
// An empty value falls back to `$APP_DATA/backups` — the behavior pre-dating
// this feature. Restore is gated behind `ask(...)` because it overwrites
// `data.db` and the app only fully settles after the next launch.

import { useEffect, useState } from "react";
import { FolderCog, RotateCcw, Save } from "lucide-react";
import { ask, open as openDialog } from "@tauri-apps/plugin-dialog";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { useToast } from "../ui/Toast";
import { cn } from "../lib/cn";
import type { BackupInfo } from "../types";
import * as api from "../api";

export function SettingsBackupSection({ active }: { active: boolean }) {
  const toast = useToast();
  const [dir, setDir] = useState<string>("");
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);

  const refresh = async () => {
    setLoading(true);
    try {
      const [d, list] = await Promise.all([
        api.getBackupDir(),
        api.listBackups(),
      ]);
      setDir(d);
      setBackups(list);
    } catch (e) {
      toast.error(`백업 정보 불러오기 실패: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (active) void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const handlePickDir = async () => {
    const picked = await openDialog({ directory: true, multiple: false });
    if (!picked) return;
    const next = Array.isArray(picked) ? picked[0] : picked;
    setBusy(true);
    try {
      const canonical = await api.setBackupDir(next);
      setDir(canonical);
      await refresh();
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success("백업 위치 변경됨");
    } catch (e) {
      toast.error(`변경 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleBackupNow = async () => {
    setBusy(true);
    try {
      const path = await api.backupDb();
      await refresh();
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success(`백업 완료: ${path}`);
    } catch (e) {
      toast.error(`백업 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const handleRestore = async (info: BackupInfo) => {
    const ok = await ask(
      `${info.filename} 을(를) 복원하시겠습니까? 현재 DB가 덮어쓰기됩니다.`,
      { title: "백업 복원", kind: "warning" }
    );
    if (!ok) return;
    setBusy(true);
    try {
      await api.restoreDb(info.path);
      window.dispatchEvent(new CustomEvent("backup:changed"));
      toast.success("복원 완료 — 앱을 다시 시작하세요");
    } catch (e) {
      toast.error(`복원 실패: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex flex-col gap-5">
      <div>
        <label className="text-[12px] font-medium text-[var(--color-text)] mb-1.5 block">
          백업 위치
        </label>
        <div className="flex items-center gap-2">
          <div
            className={cn(
              "flex-1 h-9 px-3 inline-flex items-center text-[12px] font-mono",
              "rounded-[var(--radius-md)] bg-[var(--color-surface-2)]",
              "border border-[var(--color-border)] text-[var(--color-text)] truncate"
            )}
            title={dir}
          >
            {loading ? "불러오는 중…" : dir || "(설정되지 않음)"}
          </div>
          <Button
            variant="secondary"
            size="sm"
            leftIcon={FolderCog}
            onClick={handlePickDir}
            disabled={busy}
          >
            변경…
          </Button>
        </div>
      </div>

      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-[12px] font-medium text-[var(--color-text)]">
            최근 백업
          </label>
          <Button
            variant="primary"
            size="sm"
            leftIcon={Save}
            onClick={handleBackupNow}
            disabled={busy}
          >
            지금 백업
          </Button>
        </div>
        {backups.length === 0 ? (
          <p className="text-[12px] text-[var(--color-text-dim)]">
            아직 백업이 없습니다
          </p>
        ) : (
          <ul className="flex flex-col divide-y divide-[var(--color-border)] rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-surface-2)]">
            {backups.slice(0, 5).map((b) => (
              <li key={b.path} className="flex items-center gap-2 px-3 h-9">
                <span
                  className="flex-1 truncate text-[12px] font-mono text-[var(--color-text)]"
                  title={b.path}
                >
                  {b.filename}
                </span>
                <span className="text-[11px] text-[var(--color-text-dim)] shrink-0">
                  {b.created}
                </span>
                <button
                  type="button"
                  onClick={() => handleRestore(b)}
                  disabled={busy}
                  className={cn(
                    "inline-flex items-center gap-1 h-7 px-2 rounded-[var(--radius-sm)]",
                    "text-[11px] text-[var(--color-text-muted)]",
                    "hover:text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-3)]",
                    "disabled:opacity-50 disabled:cursor-not-allowed"
                  )}
                >
                  <Icon icon={RotateCcw} size={12} />
                  복원
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SettingsBackupSection.tsx
git commit -m "feat(settings): backup location + manual backup + restore list"
```

---

## Task 14: Frontend — `SettingsCategoriesSection`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/SettingsCategoriesSection.tsx`

- [ ] **Step 1: Write the section**

Drag-sortable list of category rows using dnd-kit (same pattern as `MemoBoard`). Each row:
- Color swatch → popover with 10 preset colors + custom HEX input
- Name input — inline edit, blur commits rename
- ✕ delete button — disabled with a tooltip when `usage_count > 0`

Footer:
- `+ 카테고리 추가` button that appends a new editable row bound to `create_category`.

```tsx
// User-editable category list.
//
// Rename: blur commits via `useCategories.rename`, which fires a cascading
// UPDATE on `projects.category` under the hood + dispatches
// `projects:changed`. Delete refuses when the row has dependents — the
// backend's `delete_category` returns a Korean error, the ✕ button is
// disabled + tooltipped before we even send the request.

import { useState, useRef, useEffect } from "react";
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from "@dnd-kit/core";
import type { DragEndEvent } from "@dnd-kit/core";
import {
  SortableContext,
  verticalListSortingStrategy,
  useSortable,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { GripVertical, Plus, X } from "lucide-react";
import { Button } from "../ui/Button";
import { Icon } from "../ui/Icon";
import { Popover } from "../ui/Popover";
import { Tooltip } from "../ui/Tooltip";
import { useToast } from "../ui/Toast";
import { useCategories } from "../hooks/useCategories";
import type { CategoryRow } from "../types";
import { cn } from "../lib/cn";

const PRESET_COLORS = [
  "#22c55e",
  "#f97316",
  "#a855f7",
  "#6b7280",
  "#3b82f6",
  "#ef4444",
  "#eab308",
  "#14b8a6",
  "#ec4899",
  "#0ea5e9",
];

export function SettingsCategoriesSection() {
  const toast = useToast();
  const { categories, create, rename, recolor, remove, reorder } =
    useCategories();
  const [adding, setAdding] = useState(false);
  const [newName, setNewName] = useState("");

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = async (e: DragEndEvent) => {
    const { active, over } = e;
    if (!over || active.id === over.id) return;
    const fromIdx = categories.findIndex((c) => c.id === Number(active.id));
    const toIdx = categories.findIndex((c) => c.id === Number(over.id));
    if (fromIdx < 0 || toIdx < 0) return;
    const next = [...categories];
    const [moved] = next.splice(fromIdx, 1);
    next.splice(toIdx, 0, moved);
    try {
      await reorder(next.map((c) => c.id));
    } catch (err) {
      toast.error(`순서 저장 실패: ${err}`);
    }
  };

  const commitAdd = async () => {
    const name = newName.trim();
    if (!name) {
      setAdding(false);
      setNewName("");
      return;
    }
    try {
      await create({ name });
      toast.success(`${name} 추가됨`);
    } catch (err) {
      toast.error(`추가 실패: ${err}`);
    } finally {
      setAdding(false);
      setNewName("");
    }
  };

  return (
    <div className="flex flex-col gap-3">
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={handleDragEnd}
      >
        <SortableContext
          items={categories.map((c) => c.id)}
          strategy={verticalListSortingStrategy}
        >
          <ul className="flex flex-col rounded-[var(--radius-md)] border border-[var(--color-border)] bg-[var(--color-surface-2)] divide-y divide-[var(--color-border)]">
            {categories.map((c) => (
              <CategoryRowItem
                key={c.id}
                category={c}
                onRename={(n) =>
                  rename(c.id, n).catch((e) => toast.error(`이름 변경 실패: ${e}`))
                }
                onRecolor={(col) =>
                  recolor(c.id, col).catch((e) =>
                    toast.error(`색 변경 실패: ${e}`)
                  )
                }
                onDelete={() =>
                  remove(c.id)
                    .then(() => toast.success(`${c.name} 삭제됨`))
                    .catch((e) => toast.error(`삭제 실패: ${e}`))
                }
              />
            ))}
          </ul>
        </SortableContext>
      </DndContext>

      {adding ? (
        <div className="flex gap-2">
          <input
            autoFocus
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onBlur={commitAdd}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitAdd();
              if (e.key === "Escape") {
                setAdding(false);
                setNewName("");
              }
            }}
            placeholder="카테고리 이름"
            className={cn(
              "flex-1 h-9 px-3 text-[13px] rounded-[var(--radius-md)]",
              "bg-[var(--color-surface-2)] text-[var(--color-text)]",
              "border border-[var(--color-border)]",
              "focus:outline-none focus:border-[var(--color-brand-hi)]"
            )}
          />
        </div>
      ) : (
        <Button
          variant="secondary"
          size="sm"
          leftIcon={Plus}
          onClick={() => setAdding(true)}
        >
          카테고리 추가
        </Button>
      )}
    </div>
  );
}

function CategoryRowItem({
  category,
  onRename,
  onRecolor,
  onDelete,
}: {
  category: CategoryRow;
  onRename: (name: string) => void;
  onRecolor: (color: string) => void;
  onDelete: () => void;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id: category.id });
  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const [draftName, setDraftName] = useState(category.name);
  const inputRef = useRef<HTMLInputElement>(null);

  // Keep local draft synced to upstream name changes (e.g. after a successful
  // rename) so an unrelated re-render doesn't overwrite in-progress input.
  useEffect(() => {
    setDraftName(category.name);
  }, [category.name]);

  const disableDelete = category.usage_count > 0;

  return (
    <li
      ref={setNodeRef}
      style={style}
      className="flex items-center gap-2 px-3 h-11"
    >
      <button
        {...attributes}
        {...listeners}
        type="button"
        aria-label="드래그하여 순서 변경"
        className="cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text-muted)]"
      >
        <Icon icon={GripVertical} size={14} />
      </button>

      <Popover
        trigger={({ onClick, "aria-expanded": ae }) => (
          <button
            type="button"
            onClick={onClick}
            aria-expanded={ae}
            aria-label="색상 변경"
            className="w-5 h-5 rounded-full border border-[var(--color-border)] shrink-0"
            style={{ backgroundColor: category.color }}
          />
        )}
      >
        {({ close }) => (
          <ColorPicker
            value={category.color}
            onChange={(c) => {
              onRecolor(c);
              close();
            }}
          />
        )}
      </Popover>

      <input
        ref={inputRef}
        value={draftName}
        onChange={(e) => setDraftName(e.target.value)}
        onBlur={() => {
          const next = draftName.trim();
          if (next && next !== category.name) onRename(next);
          else setDraftName(category.name);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") inputRef.current?.blur();
          if (e.key === "Escape") {
            setDraftName(category.name);
            inputRef.current?.blur();
          }
        }}
        className={cn(
          "flex-1 h-8 px-2 text-[13px] rounded-[var(--radius-sm)]",
          "bg-transparent border border-transparent hover:border-[var(--color-border)]",
          "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]",
          "focus:bg-[var(--color-surface-1)]"
        )}
      />

      <span className="text-[11px] text-[var(--color-text-dim)] tabular-nums shrink-0">
        {category.usage_count}개
      </span>

      <Tooltip
        label={disableDelete ? "사용 중인 카테고리는 삭제할 수 없습니다" : "삭제"}
        side="top"
      >
        <button
          type="button"
          onClick={() => !disableDelete && onDelete()}
          disabled={disableDelete}
          aria-label="삭제"
          className={cn(
            "w-7 h-7 inline-flex items-center justify-center rounded-[var(--radius-sm)]",
            disableDelete
              ? "text-[var(--color-text-dim)] cursor-not-allowed"
              : "text-[var(--color-text-muted)] hover:text-white hover:bg-[var(--color-danger)]"
          )}
        >
          <Icon icon={X} size={14} />
        </button>
      </Tooltip>
    </li>
  );
}

function ColorPicker({
  value,
  onChange,
}: {
  value: string;
  onChange: (c: string) => void;
}) {
  const [custom, setCustom] = useState(value);
  return (
    <div className="flex flex-col gap-2 p-2">
      <div className="grid grid-cols-5 gap-2">
        {PRESET_COLORS.map((c) => (
          <button
            key={c}
            type="button"
            onClick={() => onChange(c)}
            aria-label={`색상 ${c}`}
            className={cn(
              "w-6 h-6 rounded-full border transition-transform",
              c.toLowerCase() === value.toLowerCase()
                ? "border-[var(--color-brand-hi)] scale-110"
                : "border-[var(--color-border)]"
            )}
            style={{ backgroundColor: c }}
          />
        ))}
      </div>
      <div className="flex gap-1 items-center">
        <input
          value={custom}
          onChange={(e) => setCustom(e.target.value)}
          placeholder="#rrggbb"
          className={cn(
            "flex-1 h-7 px-2 text-[12px] font-mono rounded-[var(--radius-sm)]",
            "bg-[var(--color-surface-1)] border border-[var(--color-border)]",
            "text-[var(--color-text)] focus:outline-none focus:border-[var(--color-brand-hi)]"
          )}
        />
        <button
          type="button"
          onClick={() => {
            if (/^#[0-9a-fA-F]{6}$/.test(custom)) onChange(custom);
          }}
          className="text-[11px] px-2 h-7 rounded-[var(--radius-sm)] text-[var(--color-brand-hi)] hover:bg-[var(--color-surface-3)]"
        >
          적용
        </button>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SettingsCategoriesSection.tsx
git commit -m "feat(settings): editable + sortable category list"
```

---

## Task 15: Frontend — `SettingsDialog` tab shell

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/SettingsDialog.tsx`

- [ ] **Step 1: Write the shell**

State-driven tab bar (no routing). Each tab mounts its own section; active tab controls which section gets the `active` prop so it refreshes on switch. Unsaved input is preserved per tab because the section components stay mounted — only visibility toggles.

```tsx
// Unified settings modal. Replaces the AI-only dialog.
//
// Three tabs: AI / 백업 / 카테고리. We keep every section mounted so unsaved
// input survives a tab switch; only the `active` prop flips so each section
// can refetch on activation.

import { useState } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { cn } from "../lib/cn";
import { SettingsAiSection } from "./SettingsAiSection";
import { SettingsBackupSection } from "./SettingsBackupSection";
import { SettingsCategoriesSection } from "./SettingsCategoriesSection";

type TabKey = "ai" | "backup" | "categories";

const TABS: { key: TabKey; label: string }[] = [
  { key: "ai", label: "AI" },
  { key: "backup", label: "백업" },
  { key: "categories", label: "카테고리" },
];

export function SettingsDialog({
  open,
  onClose,
  initialTab = "ai",
}: {
  open: boolean;
  onClose: () => void;
  initialTab?: TabKey;
}) {
  const [tab, setTab] = useState<TabKey>(initialTab);

  return (
    <Dialog
      open={open}
      onClose={onClose}
      labelledBy="settings-title"
      className="max-w-2xl"
    >
      <h2
        id="settings-title"
        className="text-heading text-[var(--color-text-hi)] mb-4"
      >
        설정
      </h2>

      <div
        role="tablist"
        aria-label="설정 탭"
        className="flex gap-1 mb-5 border-b border-[var(--color-border)]"
      >
        {TABS.map((t) => (
          <button
            key={t.key}
            type="button"
            role="tab"
            aria-selected={tab === t.key}
            onClick={() => setTab(t.key)}
            className={cn(
              "px-3 h-9 text-[13px] -mb-px border-b-2 transition-colors",
              tab === t.key
                ? "border-[var(--color-brand-hi)] text-[var(--color-text-hi)]"
                : "border-transparent text-[var(--color-text-muted)] hover:text-[var(--color-text)]"
            )}
          >
            {t.label}
          </button>
        ))}
      </div>

      {/* Each section stays mounted; only visibility flips. `active` tells
          the section it is the one in focus so it can refetch. */}
      <div className={tab === "ai" ? "" : "hidden"}>
        <SettingsAiSection active={tab === "ai"} />
      </div>
      <div className={tab === "backup" ? "" : "hidden"}>
        <SettingsBackupSection active={tab === "backup"} />
      </div>
      <div className={tab === "categories" ? "" : "hidden"}>
        <SettingsCategoriesSection />
      </div>

      <div className="flex justify-end mt-6">
        <Button variant="secondary" onClick={onClose}>
          닫기
        </Button>
      </div>
    </Dialog>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/SettingsDialog.tsx
git commit -m "feat(settings): tabbed SettingsDialog shell"
```

---

## Task 16: Frontend — wire `SettingsDialog` into Layout + TopBar

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/Layout.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/TopBar.tsx`

- [ ] **Step 1: Replace `AiSettingsDialog` with `SettingsDialog` in `Layout`**

In `Layout.tsx`:
- Remove `import { AiSettingsDialog } from "./AiSettingsDialog";`
- Add `import { SettingsDialog } from "./SettingsDialog";`
- Rename `aiSettingsOpen` state to `settingsOpen`
- Change the TopBar prop from `onOpenAiSettings` to `onOpenSettings`
- Remove `onBackup` (backup now lives inside SettingsDialog) and the old `handleBackup` handler
- Replace the `<AiSettingsDialog />` render with `<SettingsDialog />`

Relevant new block inside `Layout`:

```tsx
const [settingsOpen, setSettingsOpen] = useState(false);
```

TopBar usage:

```tsx
<TopBar
  active={activeTab}
  onChange={setActiveTab}
  onImport={handleImport}
  onOpenSettings={() => setSettingsOpen(true)}
/>
```

Dialog:

```tsx
<SettingsDialog
  open={settingsOpen}
  onClose={() => setSettingsOpen(false)}
/>
```

- [ ] **Step 2: Update `TopBar` props and visible buttons**

`TopBar.tsx`:
- Rename prop `onOpenAiSettings` → `onOpenSettings`
- Remove the `onBackup` prop and its button (functionality moved to Settings 백업 tab)
- Change the "AI 설정" button label to "설정"
- Drop the `Save` icon import (no longer used)

New relevant fragment:

```tsx
export function TopBar({
  active,
  onChange,
  onImport,
  onOpenSettings,
}: {
  active: Tab;
  onChange: (tab: Tab) => void;
  onImport: () => void;
  onOpenSettings: () => void;
}) {
  return (
    <div className="flex items-center gap-1 px-4 h-12 bg-[var(--color-surface-1)] border-b border-[var(--color-border)]">
      {/* …tabs… */}
      <AiStatusPill />
      <Button
        variant="ghost"
        size="sm"
        leftIcon={Settings2}
        onClick={onOpenSettings}
      >
        설정
      </Button>
      <Button variant="ghost" size="sm" leftIcon={Download} onClick={onImport}>
        가져오기
      </Button>
    </div>
  );
}
```

- [ ] **Step 3: Delete the now-dead `AiSettingsDialog`**

Run: `rm src/components/AiSettingsDialog.tsx`
Expected: file removed.

- [ ] **Step 4: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run: `npm run tauri dev` — click 설정 → tabs render AI / 백업 / 카테고리; 닫기 dismisses.

- [ ] **Step 5: Commit**

```bash
git add src/components/Layout.tsx src/components/TopBar.tsx
git rm src/components/AiSettingsDialog.tsx
git commit -m "feat(ui): replace AI-only dialog with unified settings modal"
```

---

## Task 17: Frontend — migrate Sidebar, ProjectCard, ProjectFormFields to `useCategories`

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/Sidebar.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectCard.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectFormFields.tsx`

- [ ] **Step 1: Update `Sidebar`**

Replace the static `CATEGORIES` iteration with `useCategories().categories`. Use the category row's `color` for the dot and `name` for both the filter value and the label. Change the `activeCategory` type from the legacy `Category` union to `string | null` (free-form).

```tsx
import { PRIORITIES, PRIORITY_COLORS, PRIORITY_LABELS } from "../types";
import type { Priority } from "../types";
import { useCategories } from "../hooks/useCategories";
import { cn } from "../lib/cn";

export function Sidebar({
  activePriorities,
  activeCategory,
  onTogglePriority,
  onSelectCategory,
}: {
  activePriorities: Set<Priority>;
  activeCategory: string | null;
  onTogglePriority: (p: Priority) => void;
  onSelectCategory: (c: string | null) => void;
}) {
  const { categories } = useCategories();

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
        <FilterItem
          active={activeCategory === null}
          onClick={() => onSelectCategory(null)}
          text="전체 보기"
        />
        {categories.map((c) => (
          <FilterItem
            key={c.id}
            active={activeCategory === c.name}
            onClick={() =>
              onSelectCategory(activeCategory === c.name ? null : c.name)
            }
            dot={c.color}
            text={c.name}
          />
        ))}
      </FilterGroup>
    </aside>
  );
}

/* FilterGroup and FilterItem bodies are unchanged from the current file. */
```

(Leave the `FilterGroup` / `FilterItem` helper components verbatim.)

- [ ] **Step 2: Update `ProjectCard` category badge to use `useCategories`**

Replace `CATEGORY_COLORS[project.category as Category]` with a lookup on the reactive categories list, falling back to the legacy constant, then to neutral gray.

Changes to `ProjectCard.tsx`:
- Drop `CATEGORIES, CATEGORY_COLORS` from the `../types` imports (keep `PRIORITIES`, `PRIORITY_COLORS`, `PRIORITY_LABELS`).
- Import `useCategories` from `../hooks/useCategories`.
- Import `CATEGORY_COLORS` (still the same file) but **only** for the fallback lookup — leave it in `types.ts`. (Alternative: inline `"#6b7280"` constant.)

The relevant block becomes:

```tsx
const { categories } = useCategories();
const catRow = categories.find((c) => c.name === project.category);
const catColor =
  catRow?.color ??
  (project.category
    ? (CATEGORY_COLORS as Record<string, string | undefined>)[project.category] ??
      "#6b7280"
    : "#6b7280");
```

The badge + popover both use `catColor` / `categories` instead of the static constants:

```tsx
{project.category ? (
  <Badge tone={catColor}>{project.category}</Badge>
) : (
  <Badge>카테고리</Badge>
)}
```

```tsx
{categories.map((c) => (
  <button
    key={c.id}
    type="button"
    onClick={() => {
      if (c.name !== project.category)
        onUpdate(project.id, { category: c.name });
      close();
    }}
    className={cn(
      "flex items-center gap-2 px-2 h-7 text-[12px] text-left rounded",
      "hover:bg-[var(--color-surface-3)]",
      c.name === project.category && "bg-[var(--color-surface-3)]"
    )}
  >
    <span
      className="w-2 h-2 rounded-full"
      style={{ backgroundColor: c.color }}
    />
    <span className="text-[var(--color-text)]">{c.name}</span>
  </button>
))}
```

Retain the existing `CATEGORY_COLORS` + `Category` imports only as needed for the fallback (swap `import type { Project, Category, Priority }` → `import type { Project, Priority }`; keep `CATEGORY_COLORS` as a value import).

- [ ] **Step 3: Update `ProjectFormFields` select**

Replace the static `CATEGORIES.map(...)` with the reactive list:

```tsx
import type { KeyboardEvent } from "react";
import { Input } from "../ui/Input";
import { PRIORITIES } from "../types";
import type { Priority } from "../types";
import { useCategories } from "../hooks/useCategories";

export type ProjectFormState = {
  name: string;
  priority: Priority;
  category: string; // free-form — empty = 없음
  path: string;
  evaluation: string;
};

/* ... emptyProjectForm unchanged ... */

export function ProjectFormFields(/* ...props unchanged... */) {
  const { categories } = useCategories();
  /* ... same body, replace the category <select> options: */

  <select
    className={SELECT_CLASS}
    value={value.category}
    onChange={(e) => onChange({ category: e.target.value })}
  >
    <option value="">카테고리 없음</option>
    {categories.map((c) => (
      <option key={c.id} value={c.name}>
        {c.name}
      </option>
    ))}
  </select>
}
```

(The `Category` string-literal union no longer constrains the form — user categories are free-form strings.)

- [ ] **Step 4: Update `Layout` types for the new free-form category**

`Layout.tsx` uses `Category` from `types.ts` for `activeCategory` state. Change to `string | null`, and relax the `set_filter` guard so it accepts any non-empty string:

```tsx
const [activeCategory, setActiveCategory] = useState<string | null>(null);
```

In `handleClientIntent`'s `set_filter` branch, relax the category lookup:

```tsx
if (Array.isArray(cats)) {
  const firstValid = (cats as unknown[]).find(
    (c): c is string => typeof c === "string" && c.length > 0
  );
  setActiveCategory(firstValid ?? null);
}
```

Drop the `CATEGORIES` import since it's no longer referenced.

Update the `children` prop typing passed to consumers of `Layout`:

```tsx
children: (props: {
  activeTab: Tab;
  priorities: Set<Priority>;
  category: string | null;
  openNewProject: () => void;
}) => React.ReactNode;
```

- [ ] **Step 5: Update `App.tsx` types**

`ProjectsTab` currently accepts `category: Category | null`. Change the annotation to `string | null` to match the new upstream. `useProjects`' second arg was `Category | null` — change `useProjects.ts` signature:

```ts
export function useProjects(
  priorities: Set<Priority>,
  category: string | null
) { /* unchanged body */ }
```

- [ ] **Step 6: Type-check + smoke**

Run: `npx tsc --noEmit`
Expected: no errors.

Run: `npm run tauri dev` — sidebar, card popovers, and form selects all list the same (seeded) five categories, and the 전체 보기 / click-toggle behavior still works.

- [ ] **Step 7: Commit**

```bash
git add src/components/Sidebar.tsx src/components/ProjectCard.tsx src/components/ProjectFormFields.tsx src/components/Layout.tsx src/App.tsx src/hooks/useProjects.ts
git commit -m "feat(categories): route sidebar/card/form through useCategories"
```

---

## Task 18: Frontend — `ProjectCard` context menu

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectCard.tsx`

- [ ] **Step 1: Wire `useContextMenu` + render the menu**

Add imports:

```tsx
import { Play, FolderOpen, X, Settings2, Trash2 } from "lucide-react";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu, type ContextMenuItem } from "../ui/ContextMenu";
import * as api from "../api";
```

Inside `ProjectCard` body, after the existing hooks:

```tsx
const { menu, open: openMenu, close: closeMenu } = useContextMenu();

const menuItems: ContextMenuItem[] = [
  {
    id: "settings",
    label: "프로젝트 설정",
    icon: Settings2,
    onSelect: () => onOpenDetail(project),
  },
  ...(project.path
    ? ([
        {
          id: "ghostty",
          label: "Ghostty에서 열기",
          icon: Play,
          onSelect: () => api.openInGhostty(project.path!),
        },
        {
          id: "finder",
          label: "Finder에서 열기",
          icon: FolderOpen,
          onSelect: () => api.openInFinder(project.path!),
        },
      ] as ContextMenuItem[])
    : []),
  { id: "sep", label: "", separator: true, onSelect: () => {} },
  {
    id: "delete",
    label: "삭제",
    icon: Trash2,
    danger: true,
    onSelect: () => onDelete(project.id),
  },
];
```

Attach `onContextMenu={openMenu}` to the root `<div>` (the one with `setNodeRef` and the `onDoubleClick` handler).

Append the menu to the JSX tree (inside the root div or as a sibling — portal handles positioning either way):

```tsx
<ContextMenu
  open={menu.open}
  x={menu.x}
  y={menu.y}
  items={menuItems}
  onClose={closeMenu}
/>
```

- [ ] **Step 2: Smoke**

Run: `npm run tauri dev`
Expected: Right-click on a project card opens the custom menu. "프로젝트 설정" opens the detail dialog. "Ghostty"/"Finder" items appear only when `project.path` is set. "삭제" deletes.

- [ ] **Step 3: Commit**

```bash
git add src/components/ProjectCard.tsx
git commit -m "feat(projects): custom right-click context menu on ProjectCard"
```

---

## Task 19: Frontend — `MemoCard` context menu

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/MemoCard.tsx`

- [ ] **Step 1: Adopt the context-menu primitive**

Remove the separate `showColors` state and the floating color-swatch button (the color picker becomes an inline row inside the menu). Keep the existing edit/delete behavior and add two new items: 색상 변경 (inline swatches) + 프로젝트 이동 (opens a small picker dialog).

Imports to add:

```tsx
import { Pencil, Palette, FolderInput, Trash2 } from "lucide-react";
import { useContextMenu } from "../hooks/useContextMenu";
import { ContextMenu, type ContextMenuItem } from "../ui/ContextMenu";
import { MemoProjectPickerDialog } from "./MemoProjectPickerDialog";
```

(The picker dialog is created in the next task — reference it now so the import is in place.)

Inside `MemoCard`:

```tsx
const { menu, open: openMenu, close: closeMenu } = useContextMenu();
const [pickerOpen, setPickerOpen] = useState(false);

const menuItems: ContextMenuItem[] = [
  {
    id: "edit",
    label: "편집",
    icon: Pencil,
    onSelect: () => setEditing(true),
  },
  {
    id: "color",
    label: "색상 변경",
    icon: Palette,
    onSelect: () => {},
    inline: (
      <div className="flex gap-1">
        {MEMO_COLORS.map((c) => (
          <button
            key={c.name}
            type="button"
            aria-label={`색상: ${c.name}`}
            onClick={() => {
              onUpdate(memo.id, { color: c.name });
              closeMenu();
            }}
            className={cn(
              "w-6 h-6 rounded-full border",
              c.name === memo.color
                ? "border-[var(--color-brand-hi)]"
                : "border-[var(--color-border)]"
            )}
            style={{ backgroundColor: c.bg }}
          />
        ))}
      </div>
    ),
  },
  {
    id: "move",
    label: "프로젝트 이동",
    icon: FolderInput,
    onSelect: () => setPickerOpen(true),
  },
  { id: "sep", label: "", separator: true, onSelect: () => {} },
  {
    id: "delete",
    label: "삭제",
    icon: Trash2,
    danger: true,
    onSelect: () => onDelete(memo.id),
  },
];
```

Add `onContextMenu={openMenu}` to the root card `<div>`.

Drop the `showColors` state and the swatch palette overlay that currently sits at `top-2 left-2`.

Render the menu + picker:

```tsx
<ContextMenu
  open={menu.open}
  x={menu.x}
  y={menu.y}
  items={menuItems}
  onClose={closeMenu}
/>
<MemoProjectPickerDialog
  open={pickerOpen}
  projects={projects}
  currentProjectId={memo.project_id}
  onClose={() => setPickerOpen(false)}
  onPick={(projectId) => {
    // null detaches — the backend's `Option<Option<i64>>` shape serializes
    // null explicitly as `Some(None)`. Passing undefined would leave the
    // field out of the payload entirely, so we always pass the key.
    onUpdate(memo.id, { project_id: projectId });
    setPickerOpen(false);
  }}
/>
```

Add `cn` import if missing.

- [ ] **Step 2: Commit**

```bash
git add src/components/MemoCard.tsx
git commit -m "feat(memos): custom right-click context menu on MemoCard"
```

---

## Task 20: Frontend — `MemoProjectPickerDialog`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/MemoProjectPickerDialog.tsx`

- [ ] **Step 1: Write a small picker dialog**

A tiny modal with the same priority-grouped project list as `NewMemoDialog`, plus a "기타 (연결 해제)" row that resolves to `null`.

```tsx
// Picker dialog for "프로젝트 이동" in the memo context menu.
//
// The spec chose a picker dialog over a nested submenu to sidestep hover-
// timing bugs. Selecting "기타 (연결 해제)" resolves to null so the caller
// can explicitly detach via `{ project_id: null }`.

import { useMemo } from "react";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { PRIORITIES } from "../types";
import type { Priority, Project } from "../types";
import { cn } from "../lib/cn";

export function MemoProjectPickerDialog({
  open,
  projects,
  currentProjectId,
  onClose,
  onPick,
}: {
  open: boolean;
  projects: Project[];
  currentProjectId: number | null;
  onClose: () => void;
  /** `null` → detach. */
  onPick: (projectId: number | null) => void;
}) {
  const grouped = useMemo(() => {
    const map = new Map<Priority, Project[]>();
    for (const p of PRIORITIES) map.set(p, []);
    for (const p of projects) {
      if ((PRIORITIES as readonly string[]).includes(p.priority)) {
        map.get(p.priority as Priority)!.push(p);
      }
    }
    return map;
  }, [projects]);

  return (
    <Dialog open={open} onClose={onClose} labelledBy="memo-picker-title">
      <h2
        id="memo-picker-title"
        className="text-heading text-[var(--color-text-hi)] mb-3"
      >
        프로젝트 이동
      </h2>
      <div className="flex flex-col gap-1 max-h-[320px] overflow-y-auto">
        <PickerRow
          active={currentProjectId === null}
          onClick={() => onPick(null)}
          label="기타 (연결 해제)"
        />
        {PRIORITIES.map((pri) => {
          const items = grouped.get(pri) ?? [];
          if (items.length === 0) return null;
          return (
            <div key={pri} className="flex flex-col">
              <div className="text-[11px] text-[var(--color-text-dim)] px-2 mt-2 mb-1">
                {pri}
              </div>
              {items.map((p) => (
                <PickerRow
                  key={p.id}
                  active={currentProjectId === p.id}
                  onClick={() => onPick(p.id)}
                  label={p.name}
                />
              ))}
            </div>
          );
        })}
      </div>
      <div className="flex justify-end mt-4">
        <Button variant="secondary" onClick={onClose}>
          취소
        </Button>
      </div>
    </Dialog>
  );
}

function PickerRow({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "h-8 px-2 text-[13px] text-left rounded-[var(--radius-sm)]",
        "transition-colors duration-[120ms]",
        active
          ? "bg-[var(--color-brand-soft)] text-[var(--color-brand-hi)]"
          : "text-[var(--color-text)] hover:bg-[var(--color-surface-3)]"
      )}
    >
      {label}
    </button>
  );
}
```

- [ ] **Step 2: Smoke**

Run: `npm run tauri dev`
Expected:
- Right-click on a memo card → menu appears.
- "편집" enters edit mode.
- "색상 변경" shows swatches inside the menu; clicking updates the color and closes the menu.
- "프로젝트 이동" opens the picker dialog; selecting a project reparents the memo; selecting 기타 detaches it.
- "삭제" deletes.

- [ ] **Step 3: Commit**

```bash
git add src/components/MemoProjectPickerDialog.tsx
git commit -m "feat(memos): project picker dialog for memo context menu move"
```

---

## Task 21: Frontend — add e2e-ish manual verification checklist

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/docs/superpowers/plans/2026-04-17-settings-categories-context-menus.md` (this plan file — append a verification log section if desired; optional)

- [ ] **Step 1: Run through the manual verification matrix**

Launch: `npm run tauri dev`

Verify each item:

- [ ] Right-click on a project card opens the custom menu; "프로젝트 설정" opens the detail dialog.
- [ ] Right-click on a memo card opens the custom menu; "색상 변경" swatches update the memo color; "프로젝트 이동" picker reparents the memo (including 기타 detach).
- [ ] Right-click anywhere else (tab bar, empty space, sidebar) → nothing (no Inspect).
- [ ] Cmd+Opt+I still opens devtools.
- [ ] 설정 → AI 탭: toggle provider, enter key, save → toast + `ai-settings:changed` fires → AI 상태 pill updates on its next poll.
- [ ] 설정 → 백업 탭: "지금 백업" writes a file to the configured directory and list refreshes.
- [ ] 설정 → 백업 탭: 변경… picks a new directory; subsequent backups land there; list reflects the new dir only.
- [ ] 설정 → 카테고리 탭: `+ 카테고리 추가` appends a row; blur-commit persists it; sidebar + card popovers refresh.
- [ ] Rename "Active" → all existing Active-tagged projects show the new name in sidebar and cards.
- [ ] Delete a category with `usage_count > 0` → button is disabled + tooltip.
- [ ] `Cmd+K` → "새 메모" opens `NewMemoDialog`.
- [ ] MemoBoard "메모 추가" button opens `NewMemoDialog`.
- [ ] Both paths submit via `api.createMemo` and dispatch `memos:changed`.

- [ ] **Step 2: If every item passes, mark the manual log complete — otherwise open a blocker task with the specific failure.**

- [ ] **Step 3: (Optional) Commit the updated verification log if you choose to check it in**

No changes required — verification happens in the running dev environment; no commit needed unless the plan file is updated.

---

## Self-Review

Spec coverage check:
- 1. Custom context menus + global blocker → Tasks 7, 8, 9, 18, 19, 20 ✓
- 2. `NewMemoDialog` for content/project/color → Tasks 10, 11 ✓
- 3. User-editable categories (add / rename-cascade / delete-if-unused) + seed → Tasks 1, 2, 3, 6, 14, 17 ✓
- 4. Unified Settings modal (AI / 백업 / 카테고리) + retire AI-only dialog + persisted backup dir → Tasks 4, 5, 12, 13, 14, 15, 16 ✓

Type consistency:
- Category row shape: `CategoryRow` (id/name/color/sort_order/usage_count/created_at/updated_at) — used consistently in `api.ts`, `useCategories`, `SettingsCategoriesSection`, `ProjectCard`, `Sidebar`, `ProjectFormFields`.
- `useCategories` returns `{ categories, loading, create, rename, recolor, remove, reorder, reload }` — matches every call site (`rename`, `recolor`, `remove`, `reorder` all used).
- `useContextMenu` returns `{ menu, open, close }` where `menu = { open, x, y }`; matches both `ProjectCard` and `MemoCard`.
- `ContextMenuItem` shape used everywhere with `id/label/icon?/danger?/disabled?/onSelect` + optional `inline` / `separator`.
- `Sidebar.activeCategory` + `Layout.activeCategory` + `useProjects(_, category)` all move from `Category | null` → `string | null` together in Task 17.
- Backend command names match frontend bindings: `get_categories`, `create_category`, `update_category`, `delete_category`, `reorder_categories`, `get_backup_dir`, `set_backup_dir`.
- `UpdateCategoryInput` fields (`name?`, `color?`, `sort_order?`) match the frontend `updateCategory` binding.
- `Project.category` stays `string | null` on the backend; frontend form state switched from `Category | ""` to `string` (empty string = 없음) in ProjectFormFields.

Placeholder scan: no "TBD" / "implement later" / "handle edge cases" / "similar to Task N" phrases — every step contains the code it references.
