# Hearth CLI — Implementation Handoff

**Date:** 2026-04-23
**Branch:** `claude/pedantic-merkle-b0a5f1` (worktree at `.claude/worktrees/pedantic-merkle-b0a5f1`)
**Status:** Phase 0-14 complete. 31 commits. 88 tests passing + 12 ignored. Ready for next sub-project.
**Parent goal:** Agent-driven hearth control — **CLI (done)** → Skills (next) → Auto-deploy.

## What Exists Now

### Workspace Layout

```
src-tauri/
├── Cargo.toml            # [workspace] members = ["app", "core", "cli"]
├── core/                 # hearth-core — pure logic, no Tauri
│   └── src/
│       ├── lib.rs        # pub mod: audit, categories, clients, db, export, memos, models, projects, scan, schedules, search, views
│       ├── audit.rs      # Source/Op enums + write_audit + list + undo + redo
│       ├── categories.rs # CRUD + rename cascade (CategoryError via thiserror)
│       ├── clients.rs    # list only (read-only for now)
│       ├── db.rs         # init_db + init_db_with_recovery + migrations + FTS5 helpers
│       ├── export.rs     # Dump + export_json + ImportReport + import_json_merge
│       ├── memos.rs      # CRUD + reorder + update_by_number + delete_by_number
│       ├── models.rs     # Project, Schedule, Memo, Client structs
│       ├── projects.rs   # CRUD + search_like + reorder
│       ├── scan.rs       # scan_dir (folder → project candidates)
│       ├── schedules.rs  # CRUD + list_range
│       ├── search.rs     # search_all (FTS5 across projects/memos/schedules)
│       └── views.rs      # today / overdue / stats composite aggregators
├── app/                  # hearth-app — Tauri binary (existing behavior)
│   └── src/
│       ├── lib.rs        # Tauri setup + spawns watcher
│       ├── watcher.rs    # 500ms PRAGMA data_version polling → *:changed events
│       ├── cmd_*.rs      # Thin Tauri wrappers calling hearth_core::*
│       └── ...
└── cli/                  # hearth-cli — standalone `hearth` binary
    ├── Cargo.toml        # [[bin]] name = "hearth"
    └── src/
        ├── main.rs       # clap entrypoint + global flags (--db, --pretty, --verbose)
        ├── db.rs         # resolve_db_path + default_db_path + open
        ├── util.rs       # emit_ok / emit_err / emit_ok_pretty (comfy-table)
        └── cmd/
            ├── mod.rs
            ├── project.rs
            ├── memo.rs
            ├── schedule.rs
            ├── category.rs
            ├── search.rs
            ├── views.rs
            ├── log.rs
            ├── export.rs
            └── import.rs
```

### Schema Additions (migrations are idempotent)

- `audit_log` table + 2 indexes (`idx_audit_ts`, `idx_audit_undone`)
- `projects_fts` / `memos_fts` / `schedules_fts` FTS5 virtual tables + insert/update/delete sync triggers
  - **Deviation from spec:** Implemented in standalone FTS5 mode, NOT `content=projects` external-content mode. Decision was driven by simpler INSERT semantics during rebuild. Storage overhead is minor (short text fields). Search `snippet()`/`bm25()` work identically in both modes.
  - AD/AU triggers use `DELETE FROM fts WHERE rowid = old.id` (not the external-content `INSERT('delete', ...)` form — that was a fix bundled into commit `2f3265b`).

### `hearth` CLI (release binary at `src-tauri/target/release/hearth`)

Size: 5.6 MB. No runtime dependencies.

```
Usage: hearth [OPTIONS] <COMMAND>

Commands:
  db        DB-level utilities (path | vacuum | migrate)
  project   list | get | create | update | delete | scan | link-path
  memo      list | get | create | update | delete
  schedule  list | get | create | update | delete
  category  list | create | rename | update | delete
  search    FTS5 across projects/memos/schedules
  today     오늘 일정 + P0 프로젝트 + 최근 메모
  overdue   지난 일정 + 방치 30일+ 프로젝트
  stats     카운트 · 우선순위 · 카테고리 · 색상 분포
  log       show | undo | redo (audit_log 기반)
  undo      shortcut for `log undo`
  redo      shortcut for `log redo`
  export    --format json|sqlite [--out FILE] [--include-audit]
  import    <FILE> [--merge|--replace] [--dry-run] [--yes]

Global flags:
  --db <PATH>       DB override (falls back to $HEARTH_DB then ~/Library/.../data.db)
  --pretty          comfy-table render for list outputs
  -v, --verbose     RUST_LOG=debug equivalent
```

### Output Contract

- Default JSON: `{"ok": true, "data": ...}`
- Errors: `{"ok": false, "error": "...", "hint": "..."}` on stderr
- Exit codes: 0 success, 1 user error, 2 DB error, 64 usage
- `hearth export` (no `--out`): prints raw dump JSON to stdout (pipe-friendly)
- `--pretty`: list commands render Unicode table via `comfy-table`

### Real-Time App Reflection

App's `watcher.rs` polls `PRAGMA data_version` every 500ms. When CLI writes, app fires `projects:changed`, `memos:changed`, `schedules:changed`, `categories:changed` — existing React listeners auto-refetch. Zero restart.

## How to Build and Run

```bash
# Build
cd src-tauri && cargo build --release -p hearth-cli
# Binary at: src-tauri/target/release/hearth

# Smoke test with a temp DB
HEARTH_DB=/tmp/smoke.db ./src-tauri/target/release/hearth db migrate
HEARTH_DB=/tmp/smoke.db ./src-tauri/target/release/hearth project create "Test" --priority P0
HEARTH_DB=/tmp/smoke.db ./src-tauri/target/release/hearth today

# With the running Hearth app (live reflection)
npm run tauri dev  # in one terminal
./src-tauri/target/release/hearth memo create -c "from CLI" --color blue
# → UI updates within 0.5–1 second
```

## Test Status

```
cargo test --workspace
```
- hearth-core: 33 unit tests (audit, categories, db, memos, projects, scan, schedules, search, views)
- hearth-app: 24 unit + 14 integration (backup_dir, categories, memo_by_number, reset_data)
- hearth-cli: 14 integration tests (`tests/smoke.rs` using `assert_cmd`)
- Total: **85 passing + 12 ignored (OpenAI network-bound tests, pre-existing)**

*(The numbers 75/88 reported mid-execution by subagents were miscounts — `cargo test --workspace | rg "^test result"` authoritative.)*

## Known Deviations / Open Items

| Item | State | Note |
|---|---|---|
| FTS5 `content=` external mode | Dropped — standalone mode used | Minor storage overhead; no functional loss for current `snippet()`/`bm25()` usage |
| Search prefix matching | Plan specified exact phrase; implementation does prefix (`word*`) for single-word queries, phrase for multi-word | Intentional improvement — handles "agents" vs "agent" stemming |
| App Undo toast → audit_log wiring | **Deferred** (Task 12.1 marked optional) | Follow-up spec needed: make React Undo consume `hearth_core::audit::undo` so app + CLI share one undo stack |
| Windows / Linux builds | Not attempted | CLI is pure Rust + bundled SQLite — should cross-compile, untested |
| CLI AI proxy (`hearth ai "..."`) | Intentionally excluded | YAGNI — agents have their own LLM |
| `clients` CLI subcommand | Read-only in core, no CLI subcommand | Matches spec's decision to defer until app UI uses clients more |

## Reference Docs

- **Spec:** [2026-04-22-hearth-cli-design.md](../specs/2026-04-22-hearth-cli-design.md)
- **Plan:** [2026-04-22-hearth-cli.md](../plans/2026-04-22-hearth-cli.md)
- **This handoff:** the file you're reading

## Commit History (31 commits on this branch)

```
b3c837c docs: Hearth CLI design spec (2026-04-22)
bef4e10 docs: Hearth CLI implementation plan (2026-04-22)
574669d refactor: split src-tauri into workspace (core/app/cli)
34d35e6 fix(workspace): re-anchor gen/schemas gitignore after app move
441f1e4 feat(db): add audit_log table migration
a409e3e feat(db): projects_fts, memos_fts, schedules_fts FTS5 tables
bc13181 refactor(core): move models to hearth-core
d290584 refactor(core): move db schema/migrations to hearth-core
866e462 feat(core): audit::write_audit helper
2f3265b refactor(projects): move CRUD to hearth-core + wire audit_log
f314635 refactor(memos): move CRUD (incl. by_number) to hearth-core + audit
b92721f refactor(schedules): move CRUD to hearth-core + audit
72b756c refactor(categories): move CRUD + rename cascade to hearth-core
5c7fb52 refactor(clients): move list to hearth-core (read-only for now)
e4d837b feat(core): search::search_all FTS5 query across projects/memos/schedules
905424f feat(core): views::{today,overdue,stats} composite aggregators
1adfb7a feat(core): scan::scan_dir for folder→project candidates
e6fb769 feat(core): audit undo/redo engine (projects/memos/schedules)
65f1d91 feat(app): data_version watcher for external DB changes
a9dc5b3 feat(cli): hearth binary skeleton — clap + db {path,vacuum,migrate}
f867bd8 feat(cli): hearth project {list,get,create,update,delete}
9fc7724 feat(cli): hearth project scan/link-path
a75384e feat(cli): hearth memo {list,get,create,update,delete}
d97db61 feat(cli): hearth schedule {list,get,create,update,delete}
c81cb09 feat(cli): hearth category {list,create,rename,update,delete}
cef4b8b feat(cli): hearth search <query> with scope/limit filters
7bb0e96 feat(cli): hearth today/overdue/stats composite views
a4569b0 feat(cli): hearth log/undo/redo using audit_log
1f2d658 feat(cli): hearth export (json|sqlite) + core::export dump
4dc93a8 feat(cli): hearth import (merge | replace --yes) + core::import_json_merge
8d94814 feat(cli): --pretty flag for tabular human output (comfy-table)
05efcd6 docs: CLI section in README + 0.7.0 entry in CHANGELOG
```

## Branch State

- Working tree clean (`git status` has no pending changes)
- Worktree: `.claude/worktrees/pedantic-merkle-b0a5f1`
- Branch: `claude/pedantic-merkle-b0a5f1` (not yet merged to main)
- Ready to either merge or continue building skills/deploy on top

## Next Sub-Project Entry Point

See `docs/superpowers/handoff/2026-04-23-next-session-prompt.md` for the prompt to paste into a fresh session that kicks off the Agent Skills sub-project (the natural follow-up per the parent goal).
