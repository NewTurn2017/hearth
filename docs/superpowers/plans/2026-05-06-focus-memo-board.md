# Focus Memo Board Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a third Hearth memo view named `Focus`, with a left filter rail, monitor-like free placement board, memo size/bold/color controls, persisted normalized positions, memo tags, and matching CLI/skill/export/import/audit support.

**Architecture:** Keep the feature local-first and dependency-free: extend the existing SQLite memo schema and `hearth_core::memos` contract first, expose the same contract through Tauri commands and the CLI, then render Focus using existing React, dnd-kit, ContextMenu, Toast, and hook patterns. Store position as normalized `focus_x` / `focus_y`, use `font_size` enum-like text plus `is_bold`, return nested `tags` from `get_memos`, and keep List/Matrix behavior intact by sharing only small memo action/rendering helpers.

**Tech Stack:** Rust 1.75, rusqlite 0.34, serde/serde_json, clap 4, Tauri 2, React 19, TypeScript 5.8, Vitest/Testing Library, dnd-kit already in the app. No new dependencies.

**Spec:** [2026-05-06-focus-memo-board-design.md](../specs/2026-05-06-focus-memo-board-design.md)

---

## Scope Check

This plan intentionally covers app, core, CLI, skill, export/import, and audit together because the accepted spec requires one coherent product surface: a memo styled or tagged in CLI must show correctly in the app, and a Focus drag from the app must survive export/import and audit undo/redo. The work is phased so each commit remains reviewable and testable.

Non-goals remain locked: no arbitrary resize handles, no rotation, no zoom, no pan persistence, no infinite canvas, no markdown/rich text, and no second memo category taxonomy.

## File Structure Map

### Rust core and persistence

- Modify: `src-tauri/core/src/models.rs`
  - Add `MemoFontSize`, `MemoTag`, and extend `Memo` with `font_size`, `is_bold`, `focus_x`, `focus_y`, `tags`.
- Modify: `src-tauri/core/src/db.rs`
  - Add idempotent migration helpers for memo style/position columns, `memo_tags`, `memo_tag_links`, and Korean-first seed tags.
- Modify: `src-tauri/core/src/memos.rs`
  - Validate font size, clamp Focus coordinates, list nested tags, create/update tag links, expose memo tag CRUD and reorder.
- Modify: `src-tauri/core/src/lib.rs`
  - Export a new `memo_tags` module only if tag CRUD is split out; otherwise keep tag helpers inside `memos.rs`.
- Modify: `src-tauri/core/src/export.rs`
  - Include memo style/position fields through the extended `Memo`; add raw `memo_tags` and `memo_tag_links` dump sections; import them in merge mode.
- Modify: `src-tauri/core/src/audit.rs`
  - Preserve new memo fields in undo/redo builders; add tag-table reverse/forward coverage for core-originated tag mutations.

### Tauri app API boundary

- Modify: `src-tauri/app/src/cmd_memos.rs`
  - Add create/update fields for style, position, and tag replacement; add Tauri commands for memo tag CRUD/reorder.
- Modify: `src-tauri/app/src/lib.rs`
  - Register new `get_memo_tags`, `create_memo_tag`, `update_memo_tag`, `delete_memo_tag`, `reorder_memo_tags` commands.
- Modify: `src/api.ts`
  - Extend `createMemo`, `updateMemo`, `updateMemoByNumber`; add memo tag API functions.
- Modify: `src/types.ts`
  - Add `MemoFontSize`, `MemoTag`; extend `Memo`.
- Create: `src/hooks/useMemoTags.ts`
  - Mirror `useCategories` with `memo-tags:changed` plus `memos:changed` where visible memo tags may change.

### CLI and agent skill

- Modify: `src-tauri/cli/src/cmd/memo.rs`
  - Add `--size`, `--bold`, `--tag`, `--clear-tags`, `--focus-x`, `--focus-y`.
- Create: `src-tauri/cli/src/cmd/memo_tag.rs`
  - Add `hearth memo-tag list/create/update/delete`.
- Modify: `src-tauri/cli/src/cmd/mod.rs`
  - Export `memo_tag`.
- Modify: `src-tauri/cli/src/main.rs`
  - Register top-level `MemoTag`.
- Modify: `src-tauri/cli/tests/smoke.rs`
  - Add temp-DB smoke tests for styled memos, tag replacement, clear-tags, focus coordinates, and memo-tag CRUD.
- Modify: `skills/hearth/SKILL.md`
  - Add routing recipes while preserving propose → approve → apply for mutations.
- Modify: `docs/hearth-cli-ko.md`
  - Document new memo style/tag/focus CLI examples.

### Frontend Focus view

- Modify: `src/components/MemoBoard.tsx`
  - Extend view state to `list | matrix | focus`, add Focus tab, pass projects/categories/tags into Focus.
- Create: `src/components/memoActions.tsx`
  - Small shared context-menu builders for edit/color/project/style/tag/delete items.
- Create: `src/components/MemoTagPickerDialog.tsx`
  - Reusable tag picker/create dialog for memo menu actions.
- Modify: `src/components/MemoCard.tsx`
  - Apply font size/bold classes, show tag chips, use shared menu helper.
- Modify: `src/components/MemoRow.tsx`
  - Apply compact size/bold indicators, show tag chips where space allows, use shared menu helper.
- Create: `src/components/FocusMemoBoard.tsx`
  - Render filter rail, monitor board, deterministic default cascade, drag end persistence.
- Create: `src/components/FocusMemoNote.tsx`
  - Render a positioned note with style, tags, project label, and shared menu actions.
- Create: `src/lib/focusMemoLayout.ts`
  - Pure coordinate cascade/clamp/filter helpers for targeted tests.
- Create: `src/components/__tests__/MemoBoardFocus.test.tsx`
  - View persistence, filters, style rendering, menu callback coverage.
- Create: `src/lib/__tests__/focusMemoLayout.test.ts`
  - Deterministic cascade and clamp tests.

## Phase 0 — Preflight and Baseline

### Task 0.1: Confirm branch, clean state, and accepted spec

**Files:**
- Read: `docs/superpowers/specs/2026-05-06-focus-memo-board-design.md`
- No code changes

- [ ] **Step 1: Confirm worktree and branch**

Run:
```bash
pwd
git branch --show-current
git rev-parse --short HEAD
git status --short
```
Expected:
```text
/Users/genie/dev/tools/hearth/.worktrees/memo-emphasis-categories
feat/memo-emphasis-categories
17367e2
```
`git status --short` prints no tracked or untracked implementation files before Phase 1.

- [ ] **Step 2: Read the accepted spec**

Run:
```bash
sed -n '1,620p' docs/superpowers/specs/2026-05-06-focus-memo-board-design.md
```
Expected: The spec names `Focus`, normalized coordinates, `small | normal | large`, boolean bold, memo tags, category filter reuse, CLI/skill alignment, export/import/audit coverage, and List/Matrix regression protection.

- [ ] **Step 3: Run baseline tests**

Run:
```bash
npm test
cd src-tauri && cargo test
cd .. && npm run build
```
Expected: all pass before implementation. If a baseline failure is unrelated and already present, record the exact command and failure in the implementation handoff before editing.

## Phase 1 — Core Schema, Types, and Memo Contracts

### Task 1.1: Add failing core tests for memo style, position, and tags

**Files:**
- Modify: `src-tauri/core/src/memos.rs`
- Modify: `src-tauri/core/src/db.rs`

- [ ] **Step 1: Add migration tests to `src-tauri/core/src/db.rs`**

Add tests in the existing `#[cfg(test)] mod tests`:
```rust
#[test]
fn migrates_memo_style_position_columns_idempotently() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    let cols: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info('memos')").unwrap();
        stmt.query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };

    assert!(cols.iter().any(|c| c == "font_size"));
    assert!(cols.iter().any(|c| c == "is_bold"));
    assert!(cols.iter().any(|c| c == "focus_x"));
    assert!(cols.iter().any(|c| c == "focus_y"));
}

#[test]
fn creates_and_seeds_memo_tags_once() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    run_migrations(&conn).unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM memo_tags", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 5);

    let important: String = conn
        .query_row(
            "SELECT color FROM memo_tags WHERE name='중요'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(important, "#ef4444");
}
```

- [ ] **Step 2: Add memo contract tests to `src-tauri/core/src/memos.rs`**

Add tests in the existing test module:
```rust
#[test]
fn create_defaults_style_position_and_tags() {
    let mut c = fresh();
    let created = create(
        &mut c,
        Source::Cli,
        &NewMemo {
            content: "plain",
            color: "yellow",
            project_id: None,
            font_size: None,
            is_bold: None,
            focus_x: None,
            focus_y: None,
            tag_names: vec![],
        },
    )
    .unwrap();

    assert_eq!(created.font_size, MemoFontSize::Normal);
    assert!(!created.is_bold);
    assert_eq!(created.focus_x, None);
    assert_eq!(created.focus_y, None);
    assert!(created.tags.is_empty());
}

#[test]
fn update_rejects_invalid_font_size() {
    let mut c = fresh();
    let m = create(
        &mut c,
        Source::Cli,
        &NewMemo {
            content: "x",
            color: "yellow",
            project_id: None,
            font_size: None,
            is_bold: None,
            focus_x: None,
            focus_y: None,
            tag_names: vec![],
        },
    )
    .unwrap();

    let err = update(
        &mut c,
        Source::Cli,
        m.id,
        &UpdateMemo {
            content: None,
            color: None,
            project_id: None,
            font_size: Some("huge"),
            is_bold: None,
            focus_x: None,
            focus_y: None,
            tag_names: None,
        },
    )
    .unwrap_err();

    assert!(err.to_string().contains("font_size"));
}

#[test]
fn update_clamps_focus_coordinates_and_replaces_tags() {
    let mut c = fresh();
    let m = create(
        &mut c,
        Source::Cli,
        &NewMemo {
            content: "tagged",
            color: "blue",
            project_id: None,
            font_size: Some("large"),
            is_bold: Some(true),
            focus_x: Some(1.5),
            focus_y: Some(-0.25),
            tag_names: vec!["검토".to_string(), "중요".to_string()],
        },
    )
    .unwrap();

    assert_eq!(m.font_size, MemoFontSize::Large);
    assert!(m.is_bold);
    assert_eq!(m.focus_x, Some(1.0));
    assert_eq!(m.focus_y, Some(0.0));
    assert_eq!(m.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["중요", "검토"]);

    let updated = update(
        &mut c,
        Source::Cli,
        m.id,
        &UpdateMemo {
            content: None,
            color: None,
            project_id: None,
            font_size: Some("small"),
            is_bold: Some(false),
            focus_x: Some(Some(0.42)),
            focus_y: Some(Some(0.18)),
            tag_names: Some(vec!["대기".to_string()]),
        },
    )
    .unwrap();

    assert_eq!(updated.font_size, MemoFontSize::Small);
    assert!(!updated.is_bold);
    assert_eq!(updated.focus_x, Some(0.42));
    assert_eq!(updated.focus_y, Some(0.18));
    assert_eq!(updated.tags.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["대기"]);
}
```

- [ ] **Step 3: Run tests and verify they fail for missing fields**

Run:
```bash
cd src-tauri && cargo test -p hearth-core memos::tests::create_defaults_style_position_and_tags db::tests::migrates_memo_style_position_columns_idempotently
```
Expected: FAIL because `NewMemo` and `Memo` do not yet include style/position/tag fields.

### Task 1.2: Extend schema and core memo model

**Files:**
- Modify: `src-tauri/core/src/models.rs`
- Modify: `src-tauri/core/src/db.rs`
- Modify: `src-tauri/core/src/memos.rs`

- [ ] **Step 1: Extend model types**

In `src-tauri/core/src/models.rs`, replace the current `Memo` block with:
```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemoFontSize {
    Small,
    Normal,
    Large,
}

impl Default for MemoFontSize {
    fn default() -> Self {
        Self::Normal
    }
}

impl MemoFontSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Normal => "normal",
            Self::Large => "large",
        }
    }

    pub fn parse(input: &str) -> rusqlite::Result<Self> {
        match input {
            "small" => Ok(Self::Small),
            "normal" => Ok(Self::Normal),
            "large" => Ok(Self::Large),
            other => Err(rusqlite::Error::ToSqlConversionFailure(
                format!("invalid font_size: {other}").into(),
            )),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct MemoTag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Memo {
    pub id: i64,
    pub content: String,
    pub color: String,
    pub project_id: Option<i64>,
    pub sort_order: i64,
    pub font_size: MemoFontSize,
    pub is_bold: bool,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tags: Vec<MemoTag>,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 2: Add idempotent DB helpers**

In `src-tauri/core/src/db.rs`, add helpers near existing `ensure_*` functions:
```rust
fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info('{table}')"))?;
    stmt.query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<Vec<_>>>()
}

fn ensure_memo_style_columns(conn: &Connection) -> Result<()> {
    let cols = table_columns(conn, "memos")?;
    if !cols.iter().any(|c| c == "font_size") {
        conn.execute_batch("ALTER TABLE memos ADD COLUMN font_size TEXT NOT NULL DEFAULT 'normal';")?;
    }
    if !cols.iter().any(|c| c == "is_bold") {
        conn.execute_batch("ALTER TABLE memos ADD COLUMN is_bold INTEGER NOT NULL DEFAULT 0;")?;
    }
    if !cols.iter().any(|c| c == "focus_x") {
        conn.execute_batch("ALTER TABLE memos ADD COLUMN focus_x REAL;")?;
    }
    if !cols.iter().any(|c| c == "focus_y") {
        conn.execute_batch("ALTER TABLE memos ADD COLUMN focus_y REAL;")?;
    }
    Ok(())
}

fn ensure_memo_tag_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memo_tags (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            color      TEXT    NOT NULL DEFAULT '#6b7280',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT    NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );
        CREATE TABLE IF NOT EXISTS memo_tag_links (
            memo_id INTEGER NOT NULL REFERENCES memos(id) ON DELETE CASCADE,
            tag_id  INTEGER NOT NULL REFERENCES memo_tags(id) ON DELETE CASCADE,
            PRIMARY KEY (memo_id, tag_id)
        );
        CREATE INDEX IF NOT EXISTS idx_memo_tag_links_tag ON memo_tag_links(tag_id);",
    )?;
    Ok(())
}

fn seed_memo_tags_if_empty(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM memo_tags", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let seed: [(&str, &str, i64); 5] = [
        ("중요", "#ef4444", 0),
        ("검토", "#f59e0b", 1),
        ("아이디어", "#a855f7", 2),
        ("대기", "#64748b", 3),
        ("회의", "#0ea5e9", 4),
    ];
    let tx = conn.unchecked_transaction()?;
    for (name, color, sort_order) in seed {
        tx.execute(
            "INSERT INTO memo_tags (name, color, sort_order) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, color, sort_order],
        )?;
    }
    tx.commit()
}
```

Then call these in `run_migrations` after base tables exist and before FTS rebuild:
```rust
ensure_memo_style_columns(conn)?;
ensure_memo_tag_tables(conn)?;
seed_memo_tags_if_empty(conn)?;
```

- [ ] **Step 3: Update memo row mapping and SELECT**

In `src-tauri/core/src/memos.rs`, import the new models:
```rust
use crate::models::{Memo, MemoFontSize, MemoTag};
```

Set:
```rust
const SELECT_COLS: &str = "id, content, color, project_id, sort_order, font_size, is_bold, focus_x, focus_y, created_at, updated_at";
```

Update `row_to_memo` to load tags separately after selecting rows, or use a private `MemoRow` struct first. The target shape for one row is:
```rust
fn row_to_memo_without_tags(row: &rusqlite::Row) -> rusqlite::Result<Memo> {
    let font_size_raw: String = row.get(5)?;
    Ok(Memo {
        id: row.get(0)?,
        content: row.get(1)?,
        color: row.get(2)?,
        project_id: row.get(3)?,
        sort_order: row.get(4)?,
        font_size: MemoFontSize::parse(&font_size_raw)?,
        is_bold: row.get::<_, i64>(6)? != 0,
        focus_x: row.get(7)?,
        focus_y: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        tags: Vec::new(),
    })
}
```

Then attach tags in `list` and `get` using:
```rust
fn tags_for_memo(conn: &Connection, memo_id: i64) -> rusqlite::Result<Vec<MemoTag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color, t.sort_order,
                COUNT(l2.memo_id) AS usage_count,
                t.created_at, t.updated_at
         FROM memo_tags t
         JOIN memo_tag_links l ON l.tag_id = t.id
         LEFT JOIN memo_tag_links l2 ON l2.tag_id = t.id
         WHERE l.memo_id = ?1
         GROUP BY t.id
         ORDER BY t.sort_order ASC, t.name ASC",
    )?;
    stmt.query_map([memo_id], row_to_tag)?
        .collect::<rusqlite::Result<Vec<_>>>()
}
```

- [ ] **Step 4: Update create/update inputs and validation helpers**

Extend `NewMemo` and `UpdateMemo`:
```rust
pub struct NewMemo<'a> {
    pub content: &'a str,
    pub color: &'a str,
    pub project_id: Option<i64>,
    pub font_size: Option<&'a str>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tag_names: Vec<String>,
}

pub struct UpdateMemo<'a> {
    pub content: Option<&'a str>,
    pub color: Option<&'a str>,
    pub project_id: Option<Option<i64>>,
    pub font_size: Option<&'a str>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<Option<f64>>,
    pub focus_y: Option<Option<f64>>,
    pub tag_names: Option<Vec<String>>,
}
```

Add:
```rust
fn clamp_focus(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn normalize_tag_name(name: &str) -> Option<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
```

- [ ] **Step 5: Implement tag replacement in one transaction**

Add:
```rust
fn ensure_tag_by_name(tx: &Connection, name: &str) -> rusqlite::Result<i64> {
    let normalized = normalize_tag_name(name).ok_or(rusqlite::Error::InvalidQuery)?;
    tx.execute(
        "INSERT OR IGNORE INTO memo_tags (name, sort_order)
         VALUES (?1, COALESCE((SELECT MAX(sort_order) + 1 FROM memo_tags), 0))",
        [normalized.as_str()],
    )?;
    tx.query_row("SELECT id FROM memo_tags WHERE name=?1", [normalized], |r| r.get(0))
}

fn replace_memo_tags(tx: &Connection, memo_id: i64, tag_names: &[String]) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM memo_tag_links WHERE memo_id=?1", [memo_id])?;
    let mut seen = std::collections::BTreeSet::new();
    for name in tag_names {
        let Some(normalized) = normalize_tag_name(name) else { continue; };
        if !seen.insert(normalized.clone()) {
            continue;
        }
        let tag_id = ensure_tag_by_name(tx, &normalized)?;
        tx.execute(
            "INSERT OR IGNORE INTO memo_tag_links (memo_id, tag_id) VALUES (?1, ?2)",
            params![memo_id, tag_id],
        )?;
    }
    Ok(())
}
```

- [ ] **Step 6: Run core tests**

Run:
```bash
cd src-tauri && cargo test -p hearth-core memos::tests db::tests
```
Expected: all `memos::tests` and `db::tests` pass.

- [ ] **Step 7: Commit Phase 1**

Create `/tmp/hearth-focus-phase1.msg`:
```text
Preserve memo emphasis and placement as first-class data

Constraint: Focus MVP requires app and CLI to share one durable memo contract without adding dependencies.
Rejected: Frontend-only localStorage styling | it would not round-trip through CLI, export, import, or audit.
Confidence: medium
Scope-risk: moderate
Directive: Keep future canvas features behind separate schema changes; do not overload font_size with rich text.
Tested: cd src-tauri && cargo test -p hearth-core memos::tests db::tests
Not-tested: Frontend and CLI are not wired in this phase.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add src-tauri/core/src/models.rs src-tauri/core/src/db.rs src-tauri/core/src/memos.rs
git commit -F /tmp/hearth-focus-phase1.msg
```

## Phase 2 — Memo Tags CRUD, Export/Import, and Audit Coverage

### Task 2.1: Add memo tag CRUD and reorder core tests

**Files:**
- Modify: `src-tauri/core/src/memos.rs`

- [ ] **Step 1: Add tests for tag CRUD**

Add:
```rust
#[test]
fn memo_tag_crud_and_reorder() {
    let mut c = fresh();
    let created = create_memo_tag(&mut c, Source::Cli, "긴급", Some("#f97316")).unwrap();
    assert_eq!(created.name, "긴급");
    assert_eq!(created.color, "#f97316");

    let renamed = update_memo_tag(
        &mut c,
        Source::Cli,
        created.id,
        &UpdateMemoTag {
            name: Some("긴급검토"),
            color: Some("#fb7185"),
            sort_order: Some(9),
        },
    )
    .unwrap();
    assert_eq!(renamed.name, "긴급검토");
    assert_eq!(renamed.sort_order, 9);

    reorder_memo_tags(&mut c, &[renamed.id]).unwrap();
    let listed = list_memo_tags(&c).unwrap();
    assert_eq!(listed[0].id, renamed.id);

    delete_memo_tag(&mut c, Source::Cli, renamed.id).unwrap();
    assert!(list_memo_tags(&c).unwrap().iter().all(|t| t.id != renamed.id));
}
```

- [ ] **Step 2: Implement public tag CRUD functions**

Add public functions in `memos.rs`:
```rust
pub struct UpdateMemoTag<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
    pub sort_order: Option<i64>,
}

pub fn list_memo_tags(conn: &Connection) -> rusqlite::Result<Vec<MemoTag>>;
pub fn create_memo_tag(conn: &mut Connection, source: Source, name: &str, color: Option<&str>) -> rusqlite::Result<MemoTag>;
pub fn update_memo_tag(conn: &mut Connection, source: Source, id: i64, patch: &UpdateMemoTag<'_>) -> rusqlite::Result<MemoTag>;
pub fn delete_memo_tag(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()>;
pub fn reorder_memo_tags(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()>;
```

Each mutation writes an `audit_log` row for `memo_tags`. Deleting a tag uses `ON DELETE CASCADE` to remove links and does not delete memos.

- [ ] **Step 3: Run tag CRUD tests**

Run:
```bash
cd src-tauri && cargo test -p hearth-core memos::tests::memo_tag_crud_and_reorder
```
Expected: pass.

### Task 2.2: Update export/import and audit undo/redo

**Files:**
- Modify: `src-tauri/core/src/export.rs`
- Modify: `src-tauri/core/src/audit.rs`
- Modify: `src-tauri/cli/tests/smoke.rs`

- [ ] **Step 1: Add CLI smoke test for export/import round-trip**

In `src-tauri/cli/tests/smoke.rs`, add:
```rust
#[test]
fn export_import_roundtrips_styled_tagged_memo() {
    let dir_a = TempDir::new().unwrap();
    let db_a = dir_a.path().join("a.db");
    let db_a_str = db_a.to_str().unwrap();

    let dir_b = TempDir::new().unwrap();
    let db_b = dir_b.path().join("b.db");
    let db_b_str = db_b.to_str().unwrap();

    hearth(db_a_str)
        .args([
            "memo", "create", "Focus memo",
            "--size", "large",
            "--bold",
            "--tag", "중요",
            "--focus-x", "0.42",
            "--focus-y", "0.18",
        ])
        .assert()
        .success();

    let export_path = dir_a.path().join("focus-export.json");
    let export_str = export_path.to_str().unwrap();
    hearth(db_a_str).args(["export", "--out", export_str]).assert().success();

    hearth(db_b_str).args(["import", export_str, "--merge"]).assert().success();

    let v = stdout_json(hearth(db_b_str).args(["memo", "list"]).assert());
    let memo = &v["data"][0];
    assert_eq!(memo["font_size"], "large");
    assert_eq!(memo["is_bold"], true);
    assert_eq!(memo["focus_x"], 0.42);
    assert_eq!(memo["focus_y"], 0.18);
    assert_eq!(memo["tags"][0]["name"], "중요");
}
```

- [ ] **Step 2: Extend `Dump` and import report**

In `export.rs`, add to `Dump`:
```rust
pub memo_tags: Vec<Value>,
pub memo_tag_links: Vec<Value>,
```

Add to `ImportReport`:
```rust
pub inserted_memo_tags: usize,
pub inserted_memo_tag_links: usize,
```

Query raw tags and links in `export_json`:
```rust
let memo_tags = raw_rows(conn, "SELECT id, name, color, sort_order, created_at, updated_at FROM memo_tags ORDER BY sort_order ASC, id ASC")?;
let memo_tag_links = raw_rows(conn, "SELECT memo_id, tag_id FROM memo_tag_links ORDER BY memo_id ASC, tag_id ASC")?;
```

Use a local helper that returns JSON objects with explicit column names.

- [ ] **Step 3: Import memo fields and tag links**

Update memo insert SQL:
```sql
INSERT INTO memos (content, color, project_id, sort_order, font_size, is_bold, focus_x, focus_y, created_at, updated_at)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
```

Import `memo_tags` by `name`, then import `memo_tag_links` by resolving exported tag IDs to local IDs. For merge semantics, skip a link when the memo duplicate was skipped and no matching local memo can be found by `content + color`.

- [ ] **Step 4: Extend audit builders for new memo fields and memo_tags**

In `audit.rs`, update `build_memos_insert` column list:
```rust
let cols = vec![
    "id", "content", "color", "project_id", "sort_order",
    "font_size", "is_bold", "focus_x", "focus_y", "created_at", "updated_at",
];
```

Add `"memo_tags"` support to `insert_from_json` and `update_from_json` with columns:
```rust
["id", "name", "color", "sort_order", "created_at", "updated_at"]
```

Keep `memo_tag_links` out of generic audit undo unless a later task adds grouped audit entries; tag replacement is audited through the memo `after_json` and restored by `replace_memo_tags` in memo update paths.

- [ ] **Step 5: Run export/import/audit tests**

Run:
```bash
cd src-tauri && cargo test -p hearth-core export audit
cd src-tauri && cargo test -p hearth-cli export_import_roundtrips_styled_tagged_memo
```
Expected: pass.

- [ ] **Step 6: Commit Phase 2**

Create `/tmp/hearth-focus-phase2.msg`:
```text
Keep memo tags portable across exports and audit history

Constraint: Styled and tagged memos must behave the same through app, CLI, backup, and audit flows.
Rejected: Treating tags as UI-only metadata | it would lose data during JSON export/import.
Confidence: medium
Scope-risk: moderate
Directive: Maintain semantic duplicate matching during import; do not introduce destructive replace imports.
Tested: cd src-tauri && cargo test -p hearth-core export audit; cd src-tauri && cargo test -p hearth-cli export_import_roundtrips_styled_tagged_memo
Not-tested: Tauri app command surface is wired in the next phase.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add src-tauri/core/src/export.rs src-tauri/core/src/audit.rs src-tauri/cli/tests/smoke.rs
git commit -F /tmp/hearth-focus-phase2.msg
```

## Phase 3 — Tauri Commands, TypeScript API, and Hooks

### Task 3.1: Wire Tauri memo and memo-tag commands

**Files:**
- Modify: `src-tauri/app/src/cmd_memos.rs`
- Modify: `src-tauri/app/src/lib.rs`
- Modify: `src/api.ts`
- Modify: `src/types.ts`
- Create: `src/hooks/useMemoTags.ts`

- [ ] **Step 1: Extend TypeScript types**

In `src/types.ts`, replace the current `Memo` interface with:
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

- [ ] **Step 2: Extend `src/api.ts` memo functions**

Set:
```ts
export type MemoInput = {
  content: string;
  color?: string;
  project_id?: number;
  font_size?: MemoFontSize;
  is_bold?: boolean;
  focus_x?: number | null;
  focus_y?: number | null;
  tag_names?: string[];
};

export type MemoUpdateInput = {
  content?: string;
  color?: string;
  project_id?: number | null;
  font_size?: MemoFontSize;
  is_bold?: boolean;
  focus_x?: number | null;
  focus_y?: number | null;
  tag_names?: string[];
};

export const createMemo = (data: MemoInput) =>
  invoke<Memo>("create_memo", { data });

export const updateMemo = (id: number, fields: MemoUpdateInput) =>
  invoke<Memo>("update_memo", { id, fields });
```

Add:
```ts
export const getMemoTags = () => invoke<MemoTag[]>("get_memo_tags");
export const createMemoTag = (input: { name: string; color?: string }) =>
  invoke<MemoTag>("create_memo_tag", { input });
export const updateMemoTag = (
  id: number,
  fields: { name?: string; color?: string; sort_order?: number },
) => invoke<MemoTag>("update_memo_tag", { id, fields });
export const deleteMemoTag = (id: number) =>
  invoke<void>("delete_memo_tag", { id });
export const reorderMemoTags = (ids: number[]) =>
  invoke<void>("reorder_memo_tags", { ids });
```

- [ ] **Step 3: Extend Tauri command structs**

In `cmd_memos.rs`, extend `MemoInput` and `UpdateMemoInput`:
```rust
pub struct MemoInput {
    pub content: String,
    pub color: Option<String>,
    pub project_id: Option<i64>,
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<f64>,
    pub focus_y: Option<f64>,
    pub tag_names: Option<Vec<String>>,
}

pub struct UpdateMemoInput {
    pub content: Option<String>,
    pub color: Option<String>,
    pub project_id: Option<Option<i64>>,
    pub font_size: Option<String>,
    pub is_bold: Option<bool>,
    pub focus_x: Option<Option<f64>>,
    pub focus_y: Option<Option<f64>>,
    pub tag_names: Option<Vec<String>>,
}
```

Map these into `NewMemo` / `UpdateMemo`. For create, use `data.tag_names.unwrap_or_default()`.

- [ ] **Step 4: Add Tauri tag commands**

Add command structs:
```rust
#[derive(Debug, Deserialize)]
pub struct MemoTagInput {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoTagInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}
```

Add commands:
```rust
#[tauri::command]
pub fn get_memo_tags(state: State<'_, AppState>) -> Result<Vec<MemoTag>, String>;

#[tauri::command]
pub fn create_memo_tag(state: State<'_, AppState>, input: MemoTagInput) -> Result<MemoTag, String>;

#[tauri::command]
pub fn update_memo_tag(state: State<'_, AppState>, id: i64, fields: UpdateMemoTagInput) -> Result<MemoTag, String>;

#[tauri::command]
pub fn delete_memo_tag(state: State<'_, AppState>, id: i64) -> Result<(), String>;

#[tauri::command]
pub fn reorder_memo_tags(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String>;
```

Register them in `src-tauri/app/src/lib.rs` inside `tauri::generate_handler![...]`.

- [ ] **Step 5: Create `useMemoTags`**

Create `src/hooks/useMemoTags.ts`:
```ts
import { useCallback, useEffect, useState } from "react";
import type { MemoTag } from "../types";
import * as api from "../api";

export function useMemoTags() {
  const [tags, setTags] = useState<MemoTag[]>([]);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setTags(await api.getMemoTags());
    } catch (e) {
      console.error("Failed to load memo tags:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    const onChanged = () => load();
    window.addEventListener("memo-tags:changed", onChanged);
    return () => window.removeEventListener("memo-tags:changed", onChanged);
  }, [load]);

  const notifyTags = () => window.dispatchEvent(new CustomEvent("memo-tags:changed"));
  const notifyMemos = () => window.dispatchEvent(new CustomEvent("memos:changed"));

  const create = async (input: { name: string; color?: string }) => {
    const created = await api.createMemoTag(input);
    notifyTags();
    return created;
  };

  const rename = async (id: number, name: string) => {
    const updated = await api.updateMemoTag(id, { name });
    notifyTags();
    notifyMemos();
    return updated;
  };

  const recolor = async (id: number, color: string) => {
    const updated = await api.updateMemoTag(id, { color });
    notifyTags();
    notifyMemos();
    return updated;
  };

  const remove = async (id: number) => {
    await api.deleteMemoTag(id);
    notifyTags();
    notifyMemos();
  };

  const reorder = async (ids: number[]) => {
    await api.reorderMemoTags(ids);
    notifyTags();
  };

  return { tags, loading, create, rename, recolor, remove, reorder, reload: load };
}
```

- [ ] **Step 6: Run compile checks**

Run:
```bash
cd src-tauri && cargo check -p hearth-app
npm run build
```
Expected: both pass.

- [ ] **Step 7: Commit Phase 3**

Create `/tmp/hearth-focus-phase3.msg`:
```text
Expose styled memo tags through the app command boundary

Constraint: React, Tauri, and core must share one shape for styled and tagged memos.
Rejected: Separate frontend-only tag cache | it would drift from external CLI writes.
Confidence: medium
Scope-risk: moderate
Directive: Keep memo tag events explicit as memo-tags:changed and refresh memos when visible tag labels change.
Tested: cd src-tauri && cargo check -p hearth-app; npm run build
Not-tested: CLI flags and Focus UI are still separate phases.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add src-tauri/app/src/cmd_memos.rs src-tauri/app/src/lib.rs src/api.ts src/types.ts src/hooks/useMemoTags.ts
git commit -F /tmp/hearth-focus-phase3.msg
```

## Phase 4 — CLI, Korean Docs, and Hearth Skill Routing

### Task 4.1: Extend `hearth memo` and add `hearth memo-tag`

**Files:**
- Modify: `src-tauri/cli/src/cmd/memo.rs`
- Create: `src-tauri/cli/src/cmd/memo_tag.rs`
- Modify: `src-tauri/cli/src/cmd/mod.rs`
- Modify: `src-tauri/cli/src/main.rs`
- Modify: `src-tauri/cli/tests/smoke.rs`

- [ ] **Step 1: Add CLI smoke tests**

Add:
```rust
#[test]
fn memo_create_update_style_tags_and_focus() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    let v = stdout_json(
        hearth(db_str)
            .args([
                "memo", "create", "기술 검토",
                "--color", "yellow",
                "--size", "large",
                "--bold",
                "--tag", "검토",
                "--tag", "중요",
                "--focus-x", "0.42",
                "--focus-y", "0.18",
            ])
            .assert(),
    );
    let id = v["data"]["id"].as_i64().unwrap();
    assert_eq!(v["data"]["font_size"], "large");
    assert_eq!(v["data"]["is_bold"], true);
    assert_eq!(v["data"]["tags"].as_array().unwrap().len(), 2);

    let v = stdout_json(
        hearth(db_str)
            .args([
                "memo", "update", &id.to_string(),
                "--size", "small",
                "--bold", "false",
                "--tag", "대기",
            ])
            .assert(),
    );
    assert_eq!(v["data"]["font_size"], "small");
    assert_eq!(v["data"]["is_bold"], false);
    assert_eq!(v["data"]["tags"][0]["name"], "대기");

    let v = stdout_json(
        hearth(db_str)
            .args(["memo", "update", &id.to_string(), "--clear-tags"])
            .assert(),
    );
    assert!(v["data"]["tags"].as_array().unwrap().is_empty());
}

#[test]
fn memo_tag_cli_crud() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("t.db");
    let db_str = db.to_str().unwrap();

    let created = stdout_json(
        hearth(db_str)
            .args(["memo-tag", "create", "긴급", "--color", "#f97316"])
            .assert(),
    );
    let id = created["data"]["id"].as_i64().unwrap();

    let updated = stdout_json(
        hearth(db_str)
            .args(["memo-tag", "update", &id.to_string(), "--name", "긴급검토"])
            .assert(),
    );
    assert_eq!(updated["data"]["name"], "긴급검토");

    hearth(db_str)
        .args(["memo-tag", "delete", &id.to_string()])
        .assert()
        .success();

    let listed = stdout_json(hearth(db_str).args(["memo-tag", "list"]).assert());
    assert!(listed["data"].as_array().unwrap().iter().all(|t| t["id"] != id));
}
```

- [ ] **Step 2: Extend `MemoCmd`**

Add fields:
```rust
#[arg(long = "size", value_parser = ["small", "normal", "large"])]
size: Option<String>,
#[arg(long)]
bold: bool,
#[arg(long = "tag")]
tags: Vec<String>,
#[arg(long)]
focus_x: Option<f64>,
#[arg(long)]
focus_y: Option<f64>,
```
for `Create`.

For `Update`, add:
```rust
#[arg(long = "size", value_parser = ["small", "normal", "large"])]
size: Option<String>,
#[arg(long)]
bold: Option<bool>,
#[arg(long = "tag", conflicts_with = "clear_tags")]
tags: Vec<String>,
#[arg(long)]
clear_tags: bool,
#[arg(long)]
focus_x: Option<f64>,
#[arg(long)]
focus_y: Option<f64>,
```

Map update tags as:
```rust
let tag_names = if clear_tags {
    Some(Vec::new())
} else if !tags.is_empty() {
    Some(tags)
} else {
    None
};
```

- [ ] **Step 3: Add `memo_tag.rs`**

Create a command file with:
```rust
use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::memos::{self, UpdateMemoTag};

#[derive(Subcommand)]
pub enum MemoTagCmd {
    List,
    Create {
        name: String,
        #[arg(long)]
        color: Option<String>,
    },
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        sort_order: Option<i64>,
    },
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: MemoTagCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        MemoTagCmd::List => crate::util::emit_ok(serde_json::to_value(memos::list_memo_tags(&conn)?).unwrap()),
        MemoTagCmd::Create { name, color } => {
            let tag = memos::create_memo_tag(&mut conn, Source::Cli, &name, color.as_deref())?;
            crate::util::emit_ok(serde_json::to_value(tag).unwrap());
        }
        MemoTagCmd::Update { id, name, color, sort_order } => {
            let tag = memos::update_memo_tag(
                &mut conn,
                Source::Cli,
                id,
                &UpdateMemoTag { name: name.as_deref(), color: color.as_deref(), sort_order },
            )?;
            crate::util::emit_ok(serde_json::to_value(tag).unwrap());
        }
        MemoTagCmd::Delete { id } => {
            memos::delete_memo_tag(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Register top-level command**

In `cmd/mod.rs`:
```rust
pub mod memo_tag;
```

In `main.rs`, add:
```rust
MemoTag {
    #[command(subcommand)]
    sub: crate::cmd::memo_tag::MemoTagCmd,
},
```
and dispatch it:
```rust
Commands::MemoTag { sub } => crate::cmd::memo_tag::dispatch(cli.db.as_deref(), sub),
```

- [ ] **Step 5: Run CLI tests**

Run:
```bash
cd src-tauri && cargo test -p hearth-cli memo_create_update_style_tags_and_focus memo_tag_cli_crud
```
Expected: pass.

### Task 4.2: Update skill and CLI docs

**Files:**
- Modify: `skills/hearth/SKILL.md`
- Modify: `docs/hearth-cli-ko.md`

- [ ] **Step 1: Update skill routing**

In `skills/hearth/SKILL.md`, add a memo style/tag section with these command recipes:
```markdown
### Styled memo recipes

- "크게 표시해줘", "큰 메모로 남겨줘" → propose:
  `hearth memo create "<content>" --size large`
- "진하게 표시해줘", "강조해줘" → propose:
  `hearth memo create "<content>" --bold`
- "중요 태그 달아줘" → propose:
  `hearth memo create "<content>" --tag 중요`
- Existing memo update:
  `hearth memo update <id> --size <small|normal|large> --bold <true|false>`
- Replace tags:
  `hearth memo update <id> --tag 검토 --tag 중요`
- Clear tags:
  `hearth memo update <id> --clear-tags`

Mutation rule remains unchanged: propose the exact command first, wait for user approval, then apply.
Before mutating existing memo tags, run:
`hearth memo list`
`hearth memo-tag list`
```

- [ ] **Step 2: Update Korean CLI docs**

In `docs/hearth-cli-ko.md`, add examples:
```markdown
### 메모 스타일과 태그

```bash
hearth memo create "기술 검토" --size large --bold --tag 검토 --tag 중요
hearth memo update 24 --size normal --bold false
hearth memo update 24 --tag 검토 --tag 대기
hearth memo update 24 --clear-tags
hearth memo update 24 --focus-x 0.42 --focus-y 0.18

hearth memo-tag list
hearth memo-tag create 중요 --color "#ef4444"
hearth memo-tag update 1 --name 긴급검토 --color "#f97316"
hearth memo-tag delete 1
```
```

- [ ] **Step 3: Run docs and CLI checks**

Run:
```bash
cd src-tauri && cargo test -p hearth-cli
grep -n "memo-tag" skills/hearth/SKILL.md docs/hearth-cli-ko.md
```
Expected: CLI tests pass and both docs contain `memo-tag` recipes.

- [ ] **Step 4: Commit Phase 4**

Create `/tmp/hearth-focus-phase4.msg`:
```text
Align CLI and agent recipes with styled Focus memos

Constraint: The Hearth skill is the only exposed agent router and must stay in sync with CLI behavior.
Rejected: App-only style controls | agent workflows need copyable commands for memo emphasis and tags.
Confidence: medium
Scope-risk: narrow
Directive: Preserve the skill propose-approve-apply gate for every mutating memo recipe.
Tested: cd src-tauri && cargo test -p hearth-cli; grep -n "memo-tag" skills/hearth/SKILL.md docs/hearth-cli-ko.md
Not-tested: Focus React board is implemented in the next phase.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add src-tauri/cli/src/cmd/memo.rs src-tauri/cli/src/cmd/memo_tag.rs src-tauri/cli/src/cmd/mod.rs src-tauri/cli/src/main.rs src-tauri/cli/tests/smoke.rs skills/hearth/SKILL.md docs/hearth-cli-ko.md
git commit -F /tmp/hearth-focus-phase4.msg
```

## Phase 5 — Frontend Shared Memo Actions and Focus Board

### Task 5.1: Add pure Focus layout helpers and tests

**Files:**
- Create: `src/lib/focusMemoLayout.ts`
- Create: `src/lib/__tests__/focusMemoLayout.test.ts`

- [ ] **Step 1: Create layout tests**

Create `src/lib/__tests__/focusMemoLayout.test.ts`:
```ts
import { clampFocusCoordinate, defaultFocusPosition, filterFocusMemos } from "../focusMemoLayout";
import type { Memo, Project } from "../../types";

const memo = (id: number, fields: Partial<Memo> = {}): Memo => ({
  id,
  content: `memo ${id}`,
  color: "yellow",
  project_id: null,
  sort_order: id,
  font_size: "normal",
  is_bold: false,
  focus_x: null,
  focus_y: null,
  tags: [],
  created_at: "now",
  updated_at: "now",
  ...fields,
});

const projects: Project[] = [
  {
    id: 1,
    priority: "P2",
    number: null,
    name: "Hearth",
    category: "Tools",
    path: null,
    evaluation: null,
    sort_order: 0,
    created_at: "now",
    updated_at: "now",
  },
];

it("clamps focus coordinates", () => {
  expect(clampFocusCoordinate(-0.1)).toBe(0);
  expect(clampFocusCoordinate(0.42)).toBe(0.42);
  expect(clampFocusCoordinate(2)).toBe(1);
});

it("uses deterministic default cascade", () => {
  expect(defaultFocusPosition(0)).toEqual({ x: 0.08, y: 0.1 });
  expect(defaultFocusPosition(4)).toEqual({ x: 0.08, y: 0.28 });
});

it("filters important memos by bold large or important tag", () => {
  const result = filterFocusMemos(
    [
      memo(1, { is_bold: true }),
      memo(2, { font_size: "large" }),
      memo(3, { tags: [{ id: 1, name: "중요", color: "#ef4444", sort_order: 0, usage_count: 1, created_at: "now", updated_at: "now" }] }),
      memo(4),
    ],
    projects,
    { quick: "important", category: null, tag: null },
  );
  expect(result.map((m) => m.id)).toEqual([1, 2, 3]);
});

it("filters by linked project category and memo tag", () => {
  const result = filterFocusMemos(
    [
      memo(1, { project_id: 1, tags: [{ id: 2, name: "검토", color: "#f59e0b", sort_order: 1, usage_count: 1, created_at: "now", updated_at: "now" }] }),
      memo(2, { project_id: 1 }),
      memo(3, { tags: [{ id: 2, name: "검토", color: "#f59e0b", sort_order: 1, usage_count: 1, created_at: "now", updated_at: "now" }] }),
    ],
    projects,
    { quick: "all", category: "Tools", tag: "검토" },
  );
  expect(result.map((m) => m.id)).toEqual([1]);
});
```

- [ ] **Step 2: Implement helpers**

Create `src/lib/focusMemoLayout.ts`:
```ts
import type { Memo, Project } from "../types";

export type FocusQuickFilter = "all" | "important" | "unlinked";

export type FocusFilters = {
  quick: FocusQuickFilter;
  category: string | null;
  tag: string | null;
};

export function clampFocusCoordinate(value: number) {
  if (Number.isNaN(value)) return 0;
  return Math.min(1, Math.max(0, Number(value.toFixed(4))));
}

export function defaultFocusPosition(index: number) {
  const x = 0.08 + (index % 4) * 0.21;
  const y = 0.1 + Math.floor(index / 4) * 0.18;
  return {
    x: clampFocusCoordinate(Math.min(x, 0.82)),
    y: clampFocusCoordinate(Math.min(y, 0.82)),
  };
}

export function memoIsImportant(memo: Memo) {
  return (
    memo.is_bold ||
    memo.font_size === "large" ||
    memo.tags.some((tag) => tag.name === "중요")
  );
}

export function filterFocusMemos(
  memos: Memo[],
  projects: Project[],
  filters: FocusFilters,
) {
  return memos.filter((memo) => {
    if (filters.quick === "important" && !memoIsImportant(memo)) return false;
    if (filters.quick === "unlinked" && memo.project_id !== null) return false;

    if (filters.category) {
      const project = projects.find((p) => p.id === memo.project_id);
      if (project?.category !== filters.category) return false;
    }

    if (filters.tag && !memo.tags.some((tag) => tag.name === filters.tag)) {
      return false;
    }

    return true;
  });
}
```

- [ ] **Step 3: Run helper tests**

Run:
```bash
npm test -- src/lib/__tests__/focusMemoLayout.test.ts
```
Expected: pass.

### Task 5.2: Extract shared memo menu and tag picker

**Files:**
- Create: `src/components/memoActions.tsx`
- Create: `src/components/MemoTagPickerDialog.tsx`
- Modify: `src/components/MemoCard.tsx`
- Modify: `src/components/MemoRow.tsx`

- [ ] **Step 1: Create shared action helper**

Create `src/components/memoActions.tsx` with exported `buildMemoMenuItems`. The helper must preserve existing edit/color/project/delete actions and add:
```tsx
{
  id: "font-size",
  label: "글씨 크기",
  inline: (
    <div className="grid grid-cols-3 gap-1">
      {[
        ["small", "작게"],
        ["normal", "일반"],
        ["large", "크게"],
      ].map(([value, label]) => (
        <button
          key={value}
          type="button"
          onClick={() => {
            onUpdate(memo.id, { font_size: value });
            closeMenu();
          }}
          className={cn(
            "h-7 rounded border text-[11px]",
            memo.font_size === value
              ? "border-[var(--color-brand-hi)] text-[var(--color-text-hi)]"
              : "border-[var(--color-border)] text-[var(--color-text-muted)]",
          )}
        >
          {label}
        </button>
      ))}
    </div>
  ),
}
```
and a bold toggle:
```tsx
{
  id: "bold",
  label: memo.is_bold ? "굵게 해제" : "굵게 표시",
  icon: Bold,
  onSelect: () => onUpdate(memo.id, { is_bold: !memo.is_bold }),
}
```

- [ ] **Step 2: Create tag picker dialog**

Create `src/components/MemoTagPickerDialog.tsx`. Required behavior:
```tsx
export function MemoTagPickerDialog({
  open,
  memo,
  tags,
  onClose,
  onApply,
  onCreateTag,
}: {
  open: boolean;
  memo: Memo;
  tags: MemoTag[];
  onClose: () => void;
  onApply: (tagNames: string[]) => void;
  onCreateTag: (name: string) => Promise<MemoTag>;
}) { /* local selected names, input text, apply button */ }
```
Use existing `Dialog`, `Button`, and `Input` components. Applying calls `onApply([...selectedNames])` and closes. Creating a new tag trims input, calls `onCreateTag`, adds the returned tag name to selection, and clears the input.

- [ ] **Step 3: Apply style rendering to Card and Row**

In `MemoCard.tsx`, apply classes:
```tsx
const textSizeClass = {
  small: "text-xs",
  normal: "text-sm",
  large: "text-base",
}[memo.font_size];
```
Use:
```tsx
<p className={cn(textSizeClass, memo.is_bold && "font-bold", "whitespace-pre-wrap [overflow-wrap:anywhere] cursor-pointer")}>
```
Render tags above the footer:
```tsx
{memo.tags.length > 0 && (
  <div className="mt-2 flex flex-wrap gap-1">
    {memo.tags.map((tag) => (
      <span key={tag.id} className="rounded-full px-1.5 py-0.5 text-[10px] bg-black/10" style={{ color: colorDef.text }}>
        #{tag.name}
      </span>
    ))}
  </div>
)}
```
In `MemoRow.tsx`, keep compact layout and add a short tag strip after the preview.

- [ ] **Step 4: Run frontend build**

Run:
```bash
npm run build
```
Expected: TypeScript and Vite build pass.

### Task 5.3: Add Focus board and Focus note UI

**Files:**
- Create: `src/components/FocusMemoBoard.tsx`
- Create: `src/components/FocusMemoNote.tsx`
- Modify: `src/components/MemoBoard.tsx`
- Create: `src/components/__tests__/MemoBoardFocus.test.tsx`

- [ ] **Step 1: Add Focus view tests**

Create `src/components/__tests__/MemoBoardFocus.test.tsx` with module mocks for `useMemos`, `useProjects`, `useCategories`, and `useMemoTags`. Include:
```tsx
it("persists and restores focus view", async () => {
  localStorage.setItem("hearth.memoboard.view", "focus");
  render(<MemoBoard />);
  expect(screen.getByRole("tab", { name: /포커스/i })).toHaveAttribute("aria-selected", "true");
});

it("renders important filter results", async () => {
  render(<MemoBoard />);
  await userEvent.click(screen.getByRole("tab", { name: /포커스/i }));
  await userEvent.click(screen.getByRole("button", { name: "중요" }));
  expect(screen.getByText("large memo")).toBeInTheDocument();
  expect(screen.queryByText("plain memo")).not.toBeInTheDocument();
});
```
Use current test setup conventions from existing component tests.

- [ ] **Step 2: Create `FocusMemoNote`**

Create a positioned note component:
```tsx
export function FocusMemoNote({
  memo,
  projects,
  tags,
  sequenceNumber,
  position,
  highlighted,
  onUpdate,
  onDelete,
  onCreateTag,
}: {
  memo: Memo;
  projects: Project[];
  tags: MemoTag[];
  sequenceNumber: number;
  position: { x: number; y: number };
  highlighted?: boolean;
  onUpdate: (id: number, fields: api.MemoUpdateInput) => Promise<Memo> | void;
  onDelete: (id: number) => void;
  onCreateTag: (name: string) => Promise<MemoTag>;
}) { /* absolute left/top %, memo style, tags, context menu */ }
```
Use `useDraggable({ id: memo.id })` from `@dnd-kit/core`. Apply style:
```tsx
style={{
  left: `${position.x * 100}%`,
  top: `${position.y * 100}%`,
  backgroundColor: colorDef.bg,
  color: colorDef.text,
  transform: CSS.Translate.toString(transform),
}}
```

- [ ] **Step 3: Create `FocusMemoBoard`**

Create state:
```tsx
const [filters, setFilters] = useState<FocusFilters>({
  quick: "all",
  category: null,
  tag: null,
});
```
Use `DndContext` with `PointerSensor`. On drag end:
```tsx
const rect = boardRef.current?.getBoundingClientRect();
if (!rect) return;
const original = positions.get(activeIdNum) ?? defaultFocusPosition(index);
const next = {
  x: clampFocusCoordinate(original.x + event.delta.x / rect.width),
  y: clampFocusCoordinate(original.y + event.delta.y / rect.height),
};
await onUpdate(activeIdNum, { focus_x: next.x, focus_y: next.y });
```
On error, show `toast.error("Focus 위치 저장 실패: ...")` and call `onReload()`.

Render rail buttons:
- `전체`
- `중요`
- `미연결`
- category names from `categories`
- tag names from `tags`

- [ ] **Step 4: Wire `MemoBoard`**

Update view type:
```ts
type MemoBoardView = "list" | "matrix" | "focus";
```
LocalStorage restore:
```ts
const [view, setView] = useState<MemoBoardView>(() => {
  const v = localStorage.getItem("hearth.memoboard.view");
  return v === "matrix" || v === "focus" ? v : "list";
});
```
Import `PanelTop` or another existing Lucide icon and add:
```tsx
<ViewTab active={view === "focus"} onClick={() => setView("focus")} icon={PanelTop} label="포커스" />
```
Render:
```tsx
{view === "focus" ? (
  <FocusMemoBoard
    memos={memos}
    projects={projects}
    categories={categories}
    tags={memoTags}
    sequence={seq}
    highlightedId={highlightedId}
    onUpdate={update}
    onDelete={remove}
    onReload={reload}
    onCreateTag={createMemoTag}
  />
) : view === "matrix" ? ( ... ) : ( ... )}
```

- [ ] **Step 5: Run Focus tests**

Run:
```bash
npm test -- src/lib/__tests__/focusMemoLayout.test.ts src/components/__tests__/MemoBoardFocus.test.tsx
```
Expected: pass.

- [ ] **Step 6: Commit Phase 5**

Create `/tmp/hearth-focus-phase5.msg`:
```text
Add a tactile Focus surface without replacing List or Matrix

Constraint: Focus must feel freeform while staying fixed-size, accessible, and dependency-free.
Rejected: Building an infinite canvas abstraction | the MVP only needs normalized positions and memo emphasis.
Confidence: medium
Scope-risk: moderate
Directive: Keep Focus-specific layout in FocusMemoBoard and pure helpers; do not entangle List/Matrix drag ordering with Focus placement.
Tested: npm test -- src/lib/__tests__/focusMemoLayout.test.ts src/components/__tests__/MemoBoardFocus.test.tsx; npm run build
Not-tested: Manual app smoke is in the final phase.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add src/lib/focusMemoLayout.ts src/lib/__tests__/focusMemoLayout.test.ts src/components/FocusMemoBoard.tsx src/components/FocusMemoNote.tsx src/components/memoActions.tsx src/components/MemoTagPickerDialog.tsx src/components/MemoBoard.tsx src/components/MemoCard.tsx src/components/MemoRow.tsx src/components/__tests__/MemoBoardFocus.test.tsx
git commit -F /tmp/hearth-focus-phase5.msg
```

## Phase 6 — Full Verification and Regression Guard

### Task 6.1: Run full automated verification

**Files:**
- No code changes unless failures reveal implementation defects

- [ ] **Step 1: Run frontend tests**

Run:
```bash
npm test
```
Expected: all Vitest suites pass, including existing List/Matrix smoke and new Focus tests.

- [ ] **Step 2: Run frontend build**

Run:
```bash
npm run build
```
Expected: TypeScript and Vite build pass.

- [ ] **Step 3: Run Rust workspace tests**

Run:
```bash
cd src-tauri && cargo test
```
Expected: all app/core/CLI tests pass.

- [ ] **Step 4: Run targeted CLI temp-DB smoke**

Run:
```bash
tmpdb="$(mktemp -t hearth-focus.XXXXXX.db)"
src-tauri/target/debug/hearth --db "$tmpdb" memo create "Focus smoke" --size large --bold --tag 중요 --focus-x 0.4 --focus-y 0.2
src-tauri/target/debug/hearth --db "$tmpdb" memo list
src-tauri/target/debug/hearth --db "$tmpdb" memo-tag list
rm -f "$tmpdb"
```
Expected: JSON output shows `font_size: "large"`, `is_bold: true`, one `중요` tag, and Focus coordinates.

### Task 6.2: Manual app smoke

**Files:**
- No code changes unless the smoke reveals defects

- [ ] **Step 1: Run app locally**

Run:
```bash
npm run tauri dev
```
Expected: Hearth opens.

- [ ] **Step 2: Verify Focus UX**

Manual checks:
```text
1. Create at least three memos.
2. Switch view tabs: List → Matrix → Focus → List.
3. Return to Focus and confirm the tab persisted through refresh.
4. Drag one memo on the board, switch to Matrix, return to Focus, confirm position remains.
5. Set one memo to large + bold + color + 중요 tag.
6. Click quick filter 중요 and confirm only important memos remain.
7. Filter by one project category and one memo tag.
8. Confirm List and Matrix still show all memos and preserve existing project grouping/order behavior.
```

- [ ] **Step 3: Verify external CLI refresh**

With app still open, run:
```bash
db="$(src-tauri/target/debug/hearth db path | python3 -c 'import json,sys; print(json.load(sys.stdin)["data"]["path"])')"
src-tauri/target/debug/hearth --db "$db" memo create "CLI Focus live" --tag 검토 --size large
```
Expected: app refreshes memos through existing DB-change bridge; if tag rows change but memo view does not refresh, dispatch `memos:changed` from the bridge path that handles memo/table data version changes.

### Task 6.3: Final review and commit

**Files:**
- All changed files

- [ ] **Step 1: Static diff checks**

Run:
```bash
git diff --check
git status --short
```
Expected: no whitespace errors; only intentional changed files.

- [ ] **Step 2: Self-review against spec**

Run:
```bash
grep -n "Focus\\|font_size\\|is_bold\\|memo_tags\\|memo-tag\\|export\\|audit" docs/superpowers/specs/2026-05-06-focus-memo-board-design.md
```
For each acceptance criterion, point to the implementing files:
```text
Focus view: src/components/MemoBoard.tsx, FocusMemoBoard.tsx
Position persistence: src/components/FocusMemoBoard.tsx, src-tauri/core/src/memos.rs
Style controls: memoActions.tsx, MemoCard.tsx, MemoRow.tsx, FocusMemoNote.tsx
Tags: useMemoTags.ts, MemoTagPickerDialog.tsx, memos.rs, CLI memo-tag
Category filters: FocusMemoBoard.tsx uses useCategories/useProjects data
CLI: src-tauri/cli/src/cmd/memo.rs, memo_tag.rs
Skill: skills/hearth/SKILL.md
Export/import/audit: export.rs, audit.rs
List/Matrix regression: MemoCard/MemoRow/MemoMatrix tests plus manual smoke
```

- [ ] **Step 3: Commit final fixes or verification notes**

If Phase 6 produced code fixes, commit them with `/tmp/hearth-focus-final.msg`:
```text
Stabilize Focus memo board after full regression verification

Constraint: Focus must ship without regressing existing memo List and Matrix workflows.
Rejected: Leaving export/import or audit gaps for follow-up | memo metadata would be unsafe for real local-first use.
Confidence: high
Scope-risk: moderate
Directive: Keep future canvas expansion additive and do not change the stored normalized coordinate contract casually.
Tested: npm test; npm run build; cd src-tauri && cargo test; manual tauri dev Focus smoke
Not-tested: Packaged MAS build unless explicitly requested.

Co-authored-by: OmX <omx@oh-my-codex.dev>
```

Run:
```bash
git add .
git commit -F /tmp/hearth-focus-final.msg
```

If Phase 6 produced no code fixes, do not create an empty commit; record the verification evidence in the final response.

## Acceptance Coverage Checklist

- [ ] Users can switch to `Focus` and see a monitor-like board: Phase 5.
- [ ] Users can drag a memo and persist position: Phases 1, 3, 5, 6.
- [ ] Users can set text size `small | normal | large`: Phases 1, 3, 4, 5.
- [ ] Users can toggle bold emphasis: Phases 1, 3, 4, 5.
- [ ] Users can assign memo tags and filter by them: Phases 1, 2, 3, 4, 5.
- [ ] Users can filter Focus by existing project categories: Phase 5.
- [ ] CLI can create/update style, tags, and Focus position: Phase 4.
- [ ] Single Hearth skill documents new CLI recipes under approval gate: Phase 4.
- [ ] List and Matrix continue to display all memos correctly: Phases 5 and 6.
- [ ] Export/import/audit preserve new fields and tags: Phase 2.

## Implementation Notes for Workers

- Do not add dependencies.
- Do not edit the main worktree at `/Users/genie/dev/tools/hearth`; this branch lives in `.worktrees/memo-emphasis-categories`.
- Keep each phase green before moving on.
- Keep memo tag names trimmed and unique by exact string.
- Keep Focus coordinates normalized and clamped in Rust core; frontend clamping is user-experience protection, not the source of truth.
- Keep existing project categories as filters only; do not add memo categories.
- Keep commits Lore-format with `Co-authored-by: OmX <omx@oh-my-codex.dev>`.
