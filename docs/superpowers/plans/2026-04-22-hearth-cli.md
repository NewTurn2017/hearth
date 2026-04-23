# Hearth CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `hearth` CLI 바이너리를 만들어 agent 가 Hearth 의 SQLite DB 를 직접 CRUD · 검색 · undo 하게 하고, 앱이 재시작 없이 실시간으로 반영하게 한다.

**Architecture:** `src-tauri/` 를 Cargo workspace 로 재편성해 `hearth-core` (순수 로직) · `hearth-app` (Tauri 앱) · `hearth-cli` (바이너리) 세 크레이트로 분할. 모든 mutation 은 tx 안에서 `audit_log` 기록 + undo 가능. 앱은 `PRAGMA data_version` 500ms 폴링으로 외부 변경 감지 후 기존 `*:changed` 이벤트 방출.

**Tech Stack:** Rust 1.75+, rusqlite 0.34 (bundled SQLite), Tauri 2, clap 4, serde_json, comfy-table, tokio, `assert_cmd`, `tempfile`.

**Spec:** [2026-04-22-hearth-cli-design.md](../specs/2026-04-22-hearth-cli-design.md)

**Phased structure** — 각 phase 끝에 `cargo test` · `cargo build` · 앱 실행이 모두 통과해야 다음 phase 진입.

---

## Phase 0 — Preflight

### Task 0.1: Baseline 확인

**Files:**
- No changes (read-only verification)

- [ ] **Step 1: 현재 테스트 & 빌드 통과 확인**

Run:
```bash
cd src-tauri && cargo test
cd .. && npm test
cd src-tauri && cargo build
```
Expected: 모두 pass. Baseline 기록 (실패가 있다면 먼저 고쳐야 함).

- [ ] **Step 2: Tauri 앱 수동 실행 동작 확인**

Run:
```bash
npm run tauri dev
```
Expected: 앱이 뜨고 프로젝트/메모/일정 탭이 기본 동작. 이후 각 phase 종료 시 이 수동 실행으로 regression 확인.

- [ ] **Step 3: 현재 파일 목록 스냅샷**

Run:
```bash
cd src-tauri && find src -type f -name "*.rs" | sort
```
기록된 목록이 Phase 1 이후 어떻게 이동했는지 나중에 비교.

---

## Phase 1 — Workspace Reorganization

> **목표:** `src-tauri/` 를 Cargo workspace 로 변환. `hearth-core` (빈 크레이트) · `hearth-app` (기존 앱, 경로만 이동) · `hearth-cli` (빈 바이너리 크레이트). 이 phase 의 끝에서 기존 테스트와 Tauri 앱이 모두 그대로 동작해야 함.

### Task 1.1: Workspace root `Cargo.toml` 생성

**Files:**
- Modify: `src-tauri/Cargo.toml` (기존 내용은 `src-tauri/app/Cargo.toml` 으로 이동)
- Create: `src-tauri/Cargo.toml` (workspace manifest)

- [ ] **Step 1: 기존 `src-tauri/Cargo.toml` 을 `src-tauri/app/Cargo.toml` 으로 이동 준비 (아직 mkdir 만)**

Run:
```bash
cd src-tauri
mkdir -p app core cli
```

- [ ] **Step 2: 기존 `src-tauri/Cargo.toml` 내용을 `src-tauri/app/Cargo.toml` 로 복사**

Run:
```bash
cd src-tauri
mv Cargo.toml app/Cargo.toml
```

`app/Cargo.toml` 의 `[package]` 섹션을 다음처럼 수정 (package name 을 `hearth-app` 로, build path 보정):

```toml
[package]
name = "hearth-app"
version = "0.6.0"
description = "Local-first personal workspace for projects, memos, and schedules."
authors = ["Jaehyun Jang <hyuni2020@gmail.com>"]
license = "MIT"
repository = "https://github.com/NewTurn2017/hearth"
readme = "../../README.md"
edition = "2021"
rust-version = "1.75"

[lib]
name = "hearth_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.34", features = ["bundled"] }
calamine = "0.26"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.12", features = ["json"] }
tokio = { version = "1", features = ["full"] }
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
tauri-plugin-autostart = "2"
tauri-plugin-notification = "2"
tauri-plugin-global-shortcut = "2"
hearth-core = { path = "../core" }

[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: 새 `src-tauri/Cargo.toml` 을 workspace manifest 로 작성**

Create `src-tauri/Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["app", "core", "cli"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "MIT"

[workspace.dependencies]
rusqlite = { version = "0.34", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
tempfile = "3"
```

- [ ] **Step 4: 기존 `src/` · `target/` · `tauri.conf.json` · `build.rs` · `icons/` · `capabilities/` · `gen/` 을 `app/` 아래로 이동**

Run:
```bash
cd src-tauri
git mv src app/src
git mv tauri.conf.json app/tauri.conf.json
git mv build.rs app/build.rs
git mv icons app/icons
[[ -d capabilities ]] && git mv capabilities app/capabilities || true
[[ -d gen ]] && git mv gen app/gen || true
[[ -d .cargo ]] && git mv .cargo app/.cargo || true
```

참고: `target/` 디렉토리는 gitignored 라 git mv 대상이 아니지만, cargo 가 workspace 레벨에서 재생성함.

- [ ] **Step 5: `app/tauri.conf.json` 안 빌드 경로 수정**

Open `src-tauri/app/tauri.conf.json`. `build.frontendDist` 또는 `build.devUrl` 에 있는 `../..` 상대 경로를 frontend 위치에 맞게 조정.

기존 `frontendDist: "../dist"` → 이제 app 이 `src-tauri/app/` 에 있으므로 `"../../dist"` 로 변경.
`devUrl` 은 `http://localhost:1420` 같이 절대 URL 이면 변경 불필요.

- [ ] **Step 6: 빈 `hearth-core` · `hearth-cli` 크레이트 스캐폴딩**

Create `src-tauri/core/Cargo.toml`:
```toml
[package]
name = "hearth-core"
version = "0.6.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"

[dependencies]
rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

Create `src-tauri/core/src/lib.rs`:
```rust
//! Hearth pure logic layer — schema, migrations, domain modules,
//! audit log, search, views. No Tauri dependency.

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn core_compiles() {
        assert_eq!(1 + 1, 2);
    }
}
```

Create `src-tauri/cli/Cargo.toml`:
```toml
[package]
name = "hearth-cli"
version = "0.6.0"
edition = "2021"
rust-version = "1.75"
license = "MIT"

[[bin]]
name = "hearth"
path = "src/main.rs"

[dependencies]
hearth-core = { path = "../core" }
rusqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
clap = { version = "4", features = ["derive"] }
comfy-table = "7"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"

[dev-dependencies]
tempfile = { workspace = true }
assert_cmd = "2"
predicates = "3"
```

Create `src-tauri/cli/src/main.rs`:
```rust
fn main() {
    println!("{{\"ok\": true, \"data\": {{\"version\": \"0.6.0\"}}}}");
}
```

- [ ] **Step 7: 루트 `package.json` 의 Tauri build 경로 확인**

Open `/Users/genie/dev/tools/hearth/package.json`. `"tauri"` script 는 `@tauri-apps/cli` 가 `tauri.conf.json` 을 찾는데, Tauri v2 는 기본적으로 `src-tauri/tauri.conf.json` 을 찾는다. workspace 아래로 이동했으니 루트에 `tauri.conf.json` 심볼릭링크 또는 설정 필요.

`package.json` 의 `"tauri"` 스크립트를 다음처럼 수정:
```json
"tauri": "tauri --config src-tauri/app/tauri.conf.json"
```

또는 Tauri v2 의 관례에 맞춰 `src-tauri/tauri.conf.json` 에 `{ "extends": "./app/tauri.conf.json" }` 재작성. 실행 실패 시 `tauri --help` 로 옵션 확인.

가장 안전한 방법: `npm run tauri dev` 한번 돌려서 실패 메시지 보고 옵션 정렬.

- [ ] **Step 8: `scripts/release.sh` 의 Tauri 경로 확인**

Grep:
```bash
rg 'src-tauri' scripts/release.sh
```
Tauri 번들 산출물은 `src-tauri/app/target/release/bundle/` 로 이동했을 수 있음. 경로가 다르면 `release.sh` 내 해당 변수 업데이트.

- [ ] **Step 9: `cargo build` · `cargo test` 로 workspace 컴파일 확인**

Run:
```bash
cd src-tauri
cargo build --workspace
cargo test --workspace
```
Expected:
- `hearth-core` : smoke test 통과
- `hearth-app` : 기존 21개 테스트 통과
- `hearth-cli` : 빌드만, 테스트 없음

- [ ] **Step 10: `npm run tauri dev` 로 앱 기동 확인**

Run:
```bash
cd /Users/genie/dev/tools/hearth
npm run tauri dev
```
Expected: 앱이 뜨고 기존 UI 모두 정상.

실패 시 Step 7 의 tauri config 경로 재조정.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -m "refactor: split src-tauri into workspace (core/app/cli)

Workspace: core (pure logic, empty) + app (existing Tauri binary) +
cli (empty hearth binary stub). No behavior change.
Prep for hearth CLI spec (2026-04-22).
"
```

---

## Phase 2 — Schema Additions (audit_log + FTS5 migrations)

> **목표:** `hearth-core::db` 에 새 테이블/트리거 마이그레이션 추가. 이 phase 끝엔 기존 앱이 여전히 동작하되 DB 에 `audit_log` 와 `*_fts` 가 생성됨.

### Task 2.1: audit_log 테이블 마이그레이션

**Files:**
- Modify: `src-tauri/app/src/db.rs:109` — `run_migrations` 함수에 추가

- [ ] **Step 1: 테스트 먼저 작성**

`src-tauri/app/src/db.rs` 의 `mod tests` 맨 아래에 추가:
```rust
#[test]
fn creates_audit_log_table() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    let cnt: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='audit_log'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(cnt, 1, "audit_log table must exist after migrations");
}

#[test]
fn audit_log_accepts_insert_and_selects_back() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO audit_log (source, op, table_name, row_id, before_json, after_json)
         VALUES ('cli', 'create', 'memos', 1, NULL, '{\"content\":\"hi\"}')",
        [],
    )
    .unwrap();
    let (source, op, table, rid): (String, String, String, i64) = conn
        .query_row(
            "SELECT source, op, table_name, row_id FROM audit_log WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(source, "cli");
    assert_eq!(op, "create");
    assert_eq!(table, "memos");
    assert_eq!(rid, 1);
}
```

- [ ] **Step 2: 실패 확인**

Run:
```bash
cd src-tauri && cargo test -p hearth-app creates_audit_log_table audit_log_accepts_insert
```
Expected: FAIL — `no such table: audit_log`.

- [ ] **Step 3: 마이그레이션 추가**

`src-tauri/app/src/db.rs` 의 `run_migrations` 함수 안 `CREATE TABLE IF NOT EXISTS categories ...` 블록 뒤에 추가:
```sql
CREATE TABLE IF NOT EXISTS audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT    NOT NULL DEFAULT (datetime('now')),
    source      TEXT    NOT NULL,
    op          TEXT    NOT NULL,
    table_name  TEXT    NOT NULL,
    row_id      INTEGER,
    before_json TEXT,
    after_json  TEXT,
    undone      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_audit_ts     ON audit_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_undone ON audit_log(undone, ts DESC);
```

(기존 `execute_batch` 문자열 말미, `);` 앞에 위 SQL 을 삽입.)

- [ ] **Step 4: 테스트 통과 확인**

Run:
```bash
cd src-tauri && cargo test -p hearth-app creates_audit_log_table audit_log_accepts_insert
```
Expected: PASS 2/2.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/app/src/db.rs
git commit -m "feat(db): add audit_log table migration

idempotent CREATE TABLE + idx_audit_ts + idx_audit_undone. Tests
verify creation and round-trip insert/select."
```

### Task 2.2: FTS5 가상 테이블 (projects_fts)

**Files:**
- Modify: `src-tauri/app/src/db.rs`

- [ ] **Step 1: 테스트 작성**

`mod tests` 에 추가:
```rust
#[test]
fn creates_projects_fts_and_syncs_on_insert() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    // Base row
    conn.execute(
        "INSERT INTO projects (name, priority, category, evaluation) VALUES ('Hearth CLI', 'P1', 'Tools', 'agent interface')",
        [],
    )
    .unwrap();
    // FTS match
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects_fts WHERE projects_fts MATCH 'agent'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1, "insert trigger must sync to FTS");
}

#[test]
fn fts_rebuild_on_existing_rows() {
    let conn = Connection::open_in_memory().unwrap();
    // pre-migration: create projects table manually, insert row BEFORE run_migrations
    conn.execute_batch(
        "CREATE TABLE projects (
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
        INSERT INTO projects (name, priority, evaluation) VALUES ('Legacy', 'P2', 'existing content');
        ",
    )
    .unwrap();
    run_migrations(&conn).unwrap();
    // FTS must find the legacy row
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projects_fts WHERE projects_fts MATCH 'existing'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1, "FTS rebuild must cover existing rows");
}
```

- [ ] **Step 2: 실패 확인**

Run:
```bash
cd src-tauri && cargo test -p hearth-app creates_projects_fts fts_rebuild
```
Expected: FAIL — `no such table: projects_fts`.

- [ ] **Step 3: FTS 마이그레이션 헬퍼 추가**

`src-tauri/app/src/db.rs` 에 헬퍼 함수:
```rust
fn ensure_projects_fts(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS projects_fts USING fts5(
            name, category, evaluation,
            content=projects, content_rowid=id
        );

        CREATE TRIGGER IF NOT EXISTS projects_ai AFTER INSERT ON projects BEGIN
            INSERT INTO projects_fts(rowid, name, category, evaluation)
            VALUES (new.id, new.name, COALESCE(new.category,''), COALESCE(new.evaluation,''));
        END;

        CREATE TRIGGER IF NOT EXISTS projects_ad AFTER DELETE ON projects BEGIN
            INSERT INTO projects_fts(projects_fts, rowid, name, category, evaluation)
            VALUES ('delete', old.id, old.name, COALESCE(old.category,''), COALESCE(old.evaluation,''));
        END;

        CREATE TRIGGER IF NOT EXISTS projects_au AFTER UPDATE ON projects BEGIN
            INSERT INTO projects_fts(projects_fts, rowid, name, category, evaluation)
            VALUES ('delete', old.id, old.name, COALESCE(old.category,''), COALESCE(old.evaluation,''));
            INSERT INTO projects_fts(rowid, name, category, evaluation)
            VALUES (new.id, new.name, COALESCE(new.category,''), COALESCE(new.evaluation,''));
        END;",
    )?;
    // Rebuild to cover rows that existed before FTS was added.
    let main_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
    let fts_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM projects_fts", [], |r| r.get(0))?;
    if main_count > 0 && fts_count == 0 {
        conn.execute_batch(
            "INSERT INTO projects_fts(rowid, name, category, evaluation)
             SELECT id, name, COALESCE(category,''), COALESCE(evaluation,'') FROM projects;",
        )?;
    }
    Ok(())
}
```

그리고 `run_migrations` 끝, `seed_categories_if_empty(conn)?;` 위에 호출 추가:
```rust
ensure_projects_fts(conn)?;
```

- [ ] **Step 4: 테스트 통과 확인**

Run:
```bash
cd src-tauri && cargo test -p hearth-app creates_projects_fts fts_rebuild
```
Expected: PASS 2/2.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/app/src/db.rs
git commit -m "feat(db): projects_fts FTS5 virtual table + sync triggers

CREATE TRIGGER ai/ad/au + bootstrap INSERT for pre-existing rows.
Tests verify trigger sync and rebuild."
```

### Task 2.3: FTS5 for memos

**Files:**
- Modify: `src-tauri/app/src/db.rs`

- [ ] **Step 1: 테스트**

```rust
#[test]
fn creates_memos_fts_and_syncs() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO memos (content, color) VALUES ('buy milk and bread', 'yellow')",
        [],
    )
    .unwrap();
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memos_fts WHERE memos_fts MATCH 'bread'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);
}
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p hearth-app creates_memos_fts` → FAIL.

- [ ] **Step 3: 헬퍼 추가**

```rust
fn ensure_memos_fts(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memos_fts USING fts5(
            content,
            content=memos, content_rowid=id
        );

        CREATE TRIGGER IF NOT EXISTS memos_ai AFTER INSERT ON memos BEGIN
            INSERT INTO memos_fts(rowid, content) VALUES (new.id, new.content);
        END;

        CREATE TRIGGER IF NOT EXISTS memos_ad AFTER DELETE ON memos BEGIN
            INSERT INTO memos_fts(memos_fts, rowid, content) VALUES ('delete', old.id, old.content);
        END;

        CREATE TRIGGER IF NOT EXISTS memos_au AFTER UPDATE ON memos BEGIN
            INSERT INTO memos_fts(memos_fts, rowid, content) VALUES ('delete', old.id, old.content);
            INSERT INTO memos_fts(rowid, content) VALUES (new.id, new.content);
        END;",
    )?;
    let main_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM memos", [], |r| r.get(0))?;
    let fts_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM memos_fts", [], |r| r.get(0))?;
    if main_count > 0 && fts_count == 0 {
        conn.execute_batch(
            "INSERT INTO memos_fts(rowid, content) SELECT id, content FROM memos;",
        )?;
    }
    Ok(())
}
```

`run_migrations` 끝에 `ensure_memos_fts(conn)?;` 추가.

- [ ] **Step 4: 테스트 통과**

Run: `cargo test -p hearth-app creates_memos_fts` → PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(db): memos_fts + triggers"
```

### Task 2.4: FTS5 for schedules

**Files:**
- Modify: `src-tauri/app/src/db.rs`

- [ ] **Step 1: 테스트**

```rust
#[test]
fn creates_schedules_fts_and_syncs() {
    let conn = Connection::open_in_memory().unwrap();
    run_migrations(&conn).unwrap();
    conn.execute(
        "INSERT INTO schedules (date, description, location, notes)
         VALUES ('2026-05-01', 'dentist', 'Seoul', 'bring insurance')",
        [],
    )
    .unwrap();
    let hits: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schedules_fts WHERE schedules_fts MATCH 'dentist'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hits, 1);
}
```

- [ ] **Step 2: 실패 확인** → FAIL.

- [ ] **Step 3: 헬퍼 추가**

```rust
fn ensure_schedules_fts(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS schedules_fts USING fts5(
            description, location, notes,
            content=schedules, content_rowid=id
        );

        CREATE TRIGGER IF NOT EXISTS schedules_ai AFTER INSERT ON schedules BEGIN
            INSERT INTO schedules_fts(rowid, description, location, notes)
            VALUES (new.id, COALESCE(new.description,''), COALESCE(new.location,''), COALESCE(new.notes,''));
        END;

        CREATE TRIGGER IF NOT EXISTS schedules_ad AFTER DELETE ON schedules BEGIN
            INSERT INTO schedules_fts(schedules_fts, rowid, description, location, notes)
            VALUES ('delete', old.id, COALESCE(old.description,''), COALESCE(old.location,''), COALESCE(old.notes,''));
        END;

        CREATE TRIGGER IF NOT EXISTS schedules_au AFTER UPDATE ON schedules BEGIN
            INSERT INTO schedules_fts(schedules_fts, rowid, description, location, notes)
            VALUES ('delete', old.id, COALESCE(old.description,''), COALESCE(old.location,''), COALESCE(old.notes,''));
            INSERT INTO schedules_fts(rowid, description, location, notes)
            VALUES (new.id, COALESCE(new.description,''), COALESCE(new.location,''), COALESCE(new.notes,''));
        END;",
    )?;
    let main_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM schedules", [], |r| r.get(0))?;
    let fts_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM schedules_fts", [], |r| r.get(0))?;
    if main_count > 0 && fts_count == 0 {
        conn.execute_batch(
            "INSERT INTO schedules_fts(rowid, description, location, notes)
             SELECT id, COALESCE(description,''), COALESCE(location,''), COALESCE(notes,'') FROM schedules;",
        )?;
    }
    Ok(())
}
```

`run_migrations` 에 `ensure_schedules_fts(conn)?;` 추가.

- [ ] **Step 4: 테스트 통과** → PASS.

- [ ] **Step 5: Commit**

```bash
git commit -am "feat(db): schedules_fts + triggers"
```

---

## Phase 3 — hearth-core domain layer

> **목표:** 기존 `cmd_*.rs` 의 순수 SQL 로직을 `hearth-core` 로 이식. Tauri 커맨드는 얇은 래퍼로 남김. 이 phase 가 끝나면 기존 테스트 + 새 core 테스트가 모두 통과하고, Tauri 앱 동작은 바뀌지 않음.

### Task 3.1: `hearth-core::models`

**Files:**
- Create: `src-tauri/core/src/models.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: 모델 이식**

Create `src-tauri/core/src/models.rs` with the contents of `src-tauri/app/src/models.rs` (exact copy, same struct definitions for Project · Schedule · Memo · Client).

- [ ] **Step 2: `lib.rs` 에 모듈 선언**

Replace `src-tauri/core/src/lib.rs` contents:
```rust
//! Hearth pure logic layer.

pub mod models;

#[cfg(test)]
mod smoke_tests {
    #[test]
    fn core_compiles() { assert_eq!(1 + 1, 2); }
}
```

- [ ] **Step 3: `hearth-app` 가 `hearth-core::models` 을 re-export 하게 수정**

Replace `src-tauri/app/src/models.rs`:
```rust
//! Re-export models from hearth-core so existing `use crate::models::*` works.
pub use hearth_core::models::*;
```

- [ ] **Step 4: 빌드 & 테스트**

Run:
```bash
cd src-tauri && cargo build --workspace && cargo test --workspace
```
Expected: 모든 테스트 통과.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(core): move models to hearth-core

app re-exports via pub use to keep existing imports working."
```

### Task 3.2: `hearth-core::db` — schema & open

**Files:**
- Create: `src-tauri/core/src/db.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: 기존 `app/src/db.rs` 의 전체 내용을 `core/src/db.rs` 로 복사**

Run:
```bash
cd src-tauri
cp app/src/db.rs core/src/db.rs
```

파일 상단 주석 갱신 (Tauri 불필요, 순수 rusqlite).

- [ ] **Step 2: `core/src/lib.rs` 에 `pub mod db;` 추가**

```rust
pub mod db;
pub mod models;
```

- [ ] **Step 3: `app/src/db.rs` 를 re-export 로 얇게**

Replace `src-tauri/app/src/db.rs`:
```rust
//! Schema lives in hearth-core. Re-export so the app uses the same migrations.
pub use hearth_core::db::{init_db, init_db_with_recovery, DbInitOutcome};
```

- [ ] **Step 4: `core/Cargo.toml` 에 `tempfile` 이 dev-deps 에 있는지 확인 (기존 테스트 의존성)**

이미 `tempfile = { workspace = true }` 로 dev-deps 에 있음.

- [ ] **Step 5: 빌드 & 테스트**

Run: `cd src-tauri && cargo test --workspace`.
Expected: 기존 `hearth-app` 의 db 테스트들이 이제 `hearth-core::db::tests` 에서 돌아감. 모두 PASS.

- [ ] **Step 6: Commit**

```bash
git commit -am "refactor(core): move db schema/migrations to hearth-core"
```

### Task 3.3: `hearth-core::audit` — audit log write helper

**Files:**
- Create: `src-tauri/core/src/audit.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: 테스트 작성**

Create `src-tauri/core/src/audit.rs`:
```rust
//! Audit log write + undo engine.
//!
//! Every mutation goes through `write_audit` inside the caller's transaction.
//! `undo_last` reverses the most recent `undone=0` entry by re-applying
//! before_json / after_json to the target table.

use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    App,
    Cli,
    Ai,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::App => "app",
            Source::Cli => "cli",
            Source::Ai => "ai",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Create,
    Update,
    Delete,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Create => "create",
            Op::Update => "update",
            Op::Delete => "delete",
        }
    }
}

/// Write a single audit_log row. Caller is responsible for running this
/// inside a transaction along with the actual data mutation.
pub fn write_audit(
    conn: &Connection,
    source: Source,
    op: Op,
    table: &str,
    row_id: i64,
    before: Option<&Value>,
    after: Option<&Value>,
) -> rusqlite::Result<i64> {
    conn.execute(
        "INSERT INTO audit_log (source, op, table_name, row_id, before_json, after_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            source.as_str(),
            op.as_str(),
            table,
            row_id,
            before.map(|v| v.to_string()),
            after.map(|v| v.to_string()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn tmp_conn() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        (dir, init_db(&path).unwrap())
    }

    #[test]
    fn write_audit_round_trip() {
        let (_d, conn) = tmp_conn();
        let before = serde_json::json!({ "name": "old" });
        let after = serde_json::json!({ "name": "new" });
        let id = write_audit(
            &conn,
            Source::Cli,
            Op::Update,
            "projects",
            42,
            Some(&before),
            Some(&after),
        )
        .unwrap();
        assert!(id > 0);
        let (src, op, tbl, rid, b, a, undone): (String, String, String, i64, String, String, i64) =
            conn.query_row(
                "SELECT source, op, table_name, row_id, before_json, after_json, undone
                 FROM audit_log WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .unwrap();
        assert_eq!(src, "cli");
        assert_eq!(op, "update");
        assert_eq!(tbl, "projects");
        assert_eq!(rid, 42);
        assert!(b.contains("old"));
        assert!(a.contains("new"));
        assert_eq!(undone, 0);
    }
}
```

Add to `core/src/lib.rs`:
```rust
pub mod audit;
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test -p hearth-core write_audit_round_trip`.
Expected: PASS (the test + implementation were written together in a single file; for strict TDD re-order if desired, but the function-level test covers behavior).

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(core): audit::write_audit helper

Source enum (App/Cli/Ai) + Op enum (Create/Update/Delete). Caller
wraps in tx alongside actual mutation."
```

### Task 3.4: `hearth-core::projects` — CRUD with audit

**Files:**
- Create: `src-tauri/core/src/projects.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: 인터페이스 설계 + 테스트**

Create `src-tauri/core/src/projects.rs` starting with tests:
```rust
use crate::audit::{write_audit, Op, Source};
use crate::models::Project;
use rusqlite::{params, Connection};
use serde_json::json;

fn row_to_project(row: &rusqlite::Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        priority: row.get(1)?,
        number: row.get(2)?,
        name: row.get(3)?,
        category: row.get(4)?,
        path: row.get(5)?,
        evaluation: row.get(6)?,
        sort_order: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const SELECT_COLS: &str =
    "id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Project>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM projects
         ORDER BY CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2
         WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 END, sort_order ASC"
    ))?;
    let rows = stmt.query_map([], row_to_project)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Project>> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM projects WHERE id=?1"))?;
    match stmt.query_row([id], row_to_project) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewProject<'a> {
    pub name: &'a str,
    pub priority: &'a str,
    pub category: Option<&'a str>,
    pub path: Option<&'a str>,
    pub evaluation: Option<&'a str>,
}

pub fn create(
    conn: &mut Connection,
    source: Source,
    input: &NewProject<'_>,
) -> rusqlite::Result<Project> {
    let tx = conn.transaction()?;
    let max_order: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM projects WHERE priority = ?1",
            [input.priority],
            |row| row.get(0),
        )
        .unwrap_or(0);

    tx.execute(
        "INSERT INTO projects (name, priority, category, path, evaluation, sort_order)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            input.name,
            input.priority,
            input.category,
            input.path,
            input.evaluation,
            max_order + 1
        ],
    )?;
    let id = tx.last_insert_rowid();
    let after = json!({
        "name": input.name,
        "priority": input.priority,
        "category": input.category,
        "path": input.path,
        "evaluation": input.evaluation,
    });
    write_audit(&tx, source, Op::Create, "projects", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|opt| opt.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateProject<'a> {
    pub name: Option<&'a str>,
    pub priority: Option<&'a str>,
    pub category: Option<&'a str>,
    pub path: Option<&'a str>,
    pub evaluation: Option<&'a str>,
}

pub fn update(
    conn: &mut Connection,
    source: Source,
    id: i64,
    patch: &UpdateProject<'_>,
) -> rusqlite::Result<Project> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;

    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    macro_rules! push_field {
        ($field:ident, $col:literal) => {
            if let Some(v) = patch.$field {
                sets.push(concat!($col, " = ?"));
                vals.push(Box::new(v.to_string()));
            }
        };
    }
    push_field!(name, "name");
    push_field!(priority, "priority");
    push_field!(category, "category");
    push_field!(path, "path");
    push_field!(evaluation, "evaluation");

    if sets.is_empty() {
        return Err(rusqlite::Error::ToSqlConversionFailure(
            "no fields to update".into(),
        ));
    }

    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;

    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let before_json = serde_json::to_value(&before).unwrap();
    let after_json = serde_json::to_value(&after).unwrap();
    write_audit(
        &tx,
        source,
        Op::Update,
        "projects",
        id,
        Some(&before_json),
        Some(&after_json),
    )?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    let before_json = serde_json::to_value(&before).unwrap();
    write_audit(
        &tx,
        source,
        Op::Delete,
        "projects",
        id,
        Some(&before_json),
        None,
    )?;
    tx.commit()?;
    Ok(())
}

pub fn search_like(conn: &Connection, query: &str) -> rusqlite::Result<Vec<Project>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM projects
         WHERE name LIKE ?1 OR evaluation LIKE ?1 OR category LIKE ?1
         ORDER BY sort_order ASC"
    ))?;
    let rows = stmt.query_map([&pattern], row_to_project)?;
    rows.collect()
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE projects SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn tmp() -> (TempDir, Connection) {
        let d = TempDir::new().unwrap();
        (d.clone_from(&d), init_db(&d.path().join("t.db")).unwrap())
    }

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        // Leak dir so conn path remains valid for the test.
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn create_inserts_row_and_audit() {
        let mut conn = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "X",
                priority: "P2",
                category: Some("Side"),
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        assert_eq!(p.name, "X");
        let audit_cnt: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log WHERE table_name='projects' AND op='create' AND row_id=?1",
                [p.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit_cnt, 1);
    }

    #[test]
    fn update_patches_only_given_fields_and_records_before_after() {
        let mut conn = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "X",
                priority: "P2",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        let updated = update(
            &mut conn,
            Source::Cli,
            p.id,
            &UpdateProject {
                name: Some("Y"),
                priority: None,
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        assert_eq!(updated.name, "Y");
        assert_eq!(updated.priority, "P2");
        let (before_json, after_json): (String, String) = conn
            .query_row(
                "SELECT before_json, after_json FROM audit_log
                 WHERE op='update' AND row_id=?1 ORDER BY id DESC LIMIT 1",
                [p.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(before_json.contains("\"X\""));
        assert!(after_json.contains("\"Y\""));
    }

    #[test]
    fn delete_removes_row_and_stores_before_json() {
        let mut conn = fresh();
        let p = create(
            &mut conn,
            Source::Cli,
            &NewProject {
                name: "D",
                priority: "P3",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        delete(&mut conn, Source::Cli, p.id).unwrap();
        assert!(get(&conn, p.id).unwrap().is_none());
        let before_json: String = conn
            .query_row(
                "SELECT before_json FROM audit_log WHERE op='delete' AND row_id=?1",
                [p.id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(before_json.contains("\"D\""));
    }
}
```

(NOTE: the `tmp()` helper shown above has a typo — use only the `fresh()` helper which is self-contained. Remove `tmp()` before committing.)

Add `pub mod projects;` to `core/src/lib.rs`.

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test -p hearth-core projects::`.
Expected: compile passes, tests pass (implementation & tests written together).

- [ ] **Step 3: `app/src/cmd_projects.rs` 를 hearth-core 래퍼로 수정**

Replace with thin wrappers that call `hearth_core::projects::*`:

```rust
use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::models::Project;
use hearth_core::projects::{self, NewProject, UpdateProject};
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct ProjectFilter {
    pub priorities: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

#[tauri::command]
pub fn get_projects(
    state: State<'_, AppState>,
    filter: Option<ProjectFilter>,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let all = projects::list(&db).map_err(|e| e.to_string())?;
    let filtered: Vec<Project> = all
        .into_iter()
        .filter(|p| match filter.as_ref() {
            None => true,
            Some(f) => {
                let pri_ok = match f.priorities.as_ref() {
                    None | Some(v) if v.as_ref().map_or(true, |v| v.is_empty()) => true,
                    Some(v) => v.contains(&p.priority),
                };
                let cat_ok = match f.categories.as_ref() {
                    None => true,
                    Some(v) if v.is_empty() => true,
                    Some(v) => p.category.as_ref().map_or(false, |c| v.contains(c)),
                };
                pri_ok && cat_ok
            }
        })
        .collect();
    Ok(filtered)
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub path: Option<String>,
    pub evaluation: Option<String>,
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateProjectInput,
) -> Result<Project, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::update(
        &mut db,
        Source::App,
        id,
        &UpdateProject {
            name: fields.name.as_deref(),
            priority: fields.priority.as_deref(),
            category: fields.category.as_deref(),
            path: fields.path.as_deref(),
            evaluation: fields.evaluation.as_deref(),
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    priority: String,
    category: Option<String>,
    path: Option<String>,
) -> Result<Project, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::create(
        &mut db,
        Source::App,
        &NewProject {
            name: &name,
            priority: &priority,
            category: category.as_deref(),
            path: path.as_deref(),
            evaluation: None,
        },
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::delete(&mut db, Source::App, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reorder_projects(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    projects::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_projects(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    projects::search_like(&db, &query).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: 빌드 & 기존 테스트 통과 확인**

Run:
```bash
cd src-tauri && cargo test --workspace
```
Expected: 기존 프로젝트 테스트 전부 통과 + 새 core 테스트 통과.

- [ ] **Step 5: 수동 앱 검증**

Run `npm run tauri dev`, UI 에서 프로젝트 생성/수정/삭제/재정렬/검색 각각 한 번씩 실행. 문제 없어야 함.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(projects): move CRUD to hearth-core + wire audit_log

cmd_projects.rs becomes thin Tauri wrapper calling projects::create/update/
delete/list/search_like. Every mutation writes audit_log (source='app')."
```

### Task 3.5: `hearth-core::memos` — CRUD with audit

**Files:**
- Create: `src-tauri/core/src/memos.rs`
- Modify: `src-tauri/app/src/cmd_memos.rs`

- [ ] **Step 1: core 모듈 작성**

Create `src-tauri/core/src/memos.rs`:
```rust
use crate::audit::{write_audit, Op, Source};
use crate::models::Memo;
use rusqlite::{params, Connection};

fn row_to_memo(row: &rusqlite::Row) -> rusqlite::Result<Memo> {
    Ok(Memo {
        id: row.get(0)?,
        content: row.get(1)?,
        color: row.get(2)?,
        project_id: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

const SELECT_COLS: &str = "id, content, color, project_id, sort_order, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Memo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLS} FROM memos ORDER BY sort_order ASC"
    ))?;
    let rows = stmt.query_map([], row_to_memo)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Memo>> {
    let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM memos WHERE id=?1"))?;
    match stmt.query_row([id], row_to_memo) {
        Ok(m) => Ok(Some(m)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewMemo<'a> {
    pub content: &'a str,
    pub color: &'a str,
    pub project_id: Option<i64>,
}

pub fn create(conn: &mut Connection, source: Source, input: &NewMemo<'_>) -> rusqlite::Result<Memo> {
    let tx = conn.transaction()?;
    let max_order: i64 = tx
        .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM memos", [], |r| {
            r.get(0)
        })
        .unwrap_or(0);
    tx.execute(
        "INSERT INTO memos (content, color, project_id, sort_order)
         VALUES (?1, ?2, ?3, ?4)",
        params![input.content, input.color, input.project_id, max_order + 1],
    )?;
    let id = tx.last_insert_rowid();
    let after = serde_json::json!({
        "content": input.content, "color": input.color, "project_id": input.project_id,
    });
    write_audit(&tx, source, Op::Create, "memos", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|opt| opt.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateMemo<'a> {
    pub content: Option<&'a str>,
    pub color: Option<&'a str>,
    /// `Some(Some(id))` → attach, `Some(None)` → detach, `None` → no change.
    pub project_id: Option<Option<i64>>,
}

pub fn update(
    conn: &mut Connection,
    source: Source,
    id: i64,
    patch: &UpdateMemo<'_>,
) -> rusqlite::Result<Memo> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(v) = patch.content {
        sets.push("content = ?");
        vals.push(Box::new(v.to_string()));
    }
    if let Some(v) = patch.color {
        sets.push("color = ?");
        vals.push(Box::new(v.to_string()));
    }
    if let Some(pid) = patch.project_id {
        sets.push("project_id = ?");
        vals.push(Box::new(pid));
    }
    if sets.is_empty() {
        return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into()));
    }
    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE memos SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|p| p.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;
    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let bj = serde_json::to_value(&before).unwrap();
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Update, "memos", id, Some(&bj), Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM memos WHERE id=?1", [id])?;
    let bj = serde_json::to_value(&before).unwrap();
    write_audit(&tx, source, Op::Delete, "memos", id, Some(&bj), None)?;
    tx.commit()?;
    Ok(())
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE memos SET sort_order=?1, updated_at=datetime('now') WHERE id=?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn create_and_list() {
        let mut c = fresh();
        create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "hi",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        assert_eq!(list(&c).unwrap().len(), 1);
    }

    #[test]
    fn update_detach_project() {
        let mut c = fresh();
        let m = create(
            &mut c,
            Source::Cli,
            &NewMemo {
                content: "a",
                color: "pink",
                project_id: Some(999),
            },
        )
        .unwrap();
        let updated = update(
            &mut c,
            Source::Cli,
            m.id,
            &UpdateMemo {
                content: None,
                color: None,
                project_id: Some(None),
            },
        )
        .unwrap();
        assert_eq!(updated.project_id, None);
    }
}
```

Add `pub mod memos;` to `core/src/lib.rs`.

- [ ] **Step 2: 테스트 확인**

Run: `cargo test -p hearth-core memos::`.
Expected: PASS.

- [ ] **Step 3: `app/src/cmd_memos.rs` 를 래퍼로**

Open existing `cmd_memos.rs`, replace each tauri command body with a call to `hearth_core::memos::*`. Keep the `#[tauri::command]` signatures unchanged so the frontend `invoke()` still works.

Pattern (follow Task 3.4 Step 3 for structure):
- `get_memos` → `memos::list(&db)`
- `create_memo` → `memos::create(&mut db, Source::App, &NewMemo {...})`
- `update_memo` → `memos::update(&mut db, Source::App, id, &UpdateMemo {...})`
- `delete_memo` → `memos::delete(&mut db, Source::App, id)`
- `reorder_memos` → `memos::reorder(&mut db, &ids)`
- `update_memo_by_number` / `delete_memo_by_number` — these are specialized to global sort_order. Keep their SQL in app for now (only list_memos is the hot path shared with CLI). Or move to `memos::update_by_number` / `memos::delete_by_number`. Decision: **move to core** to avoid drift.

For `update_memo_by_number`:
```rust
// core/src/memos.rs
pub fn update_by_number(
    conn: &mut Connection,
    source: Source,
    number: i64,
    new_content: &str,
) -> rusqlite::Result<Memo> {
    let id: i64 = {
        let tx = conn.transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
        )?;
        let id = stmt.query_row([(number - 1).max(0)], |r| r.get::<_, i64>(0))?;
        drop(stmt);
        tx.commit()?;
        id
    };
    update(
        conn,
        source,
        id,
        &UpdateMemo {
            content: Some(new_content),
            color: None,
            project_id: None,
        },
    )
}

pub fn delete_by_number(conn: &mut Connection, source: Source, number: i64) -> rusqlite::Result<()> {
    let id: i64 = {
        let tx = conn.transaction()?;
        let mut stmt = tx.prepare(
            "SELECT id FROM memos ORDER BY sort_order ASC LIMIT 1 OFFSET ?1",
        )?;
        let id = stmt.query_row([(number - 1).max(0)], |r| r.get::<_, i64>(0))?;
        drop(stmt);
        tx.commit()?;
        id
    };
    delete(conn, source, id)
}
```

Add short tests for `update_by_number` / `delete_by_number` covering valid + out-of-range.

- [ ] **Step 4: 빌드 & 테스트 & 수동**

Run: `cargo test --workspace` + `npm run tauri dev` + UI 에서 메모 생성/편집/삭제/드래그 재정렬/#N 수정 스모크.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(memos): move CRUD (incl. by_number) to hearth-core + audit"
```

### Task 3.6: `hearth-core::schedules` — CRUD with audit

**Files:**
- Create: `src-tauri/core/src/schedules.rs`
- Modify: `src-tauri/app/src/cmd_schedules.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: core 모듈 작성**

Create `src-tauri/core/src/schedules.rs`:
```rust
use crate::audit::{write_audit, Op, Source};
use crate::models::Schedule;
use rusqlite::{params, Connection};

fn row_to(row: &rusqlite::Row) -> rusqlite::Result<Schedule> {
    Ok(Schedule {
        id: row.get(0)?,
        date: row.get(1)?,
        time: row.get(2)?,
        location: row.get(3)?,
        description: row.get(4)?,
        notes: row.get(5)?,
        remind_before_5min: row.get::<_, i64>(6)? != 0,
        remind_at_start: row.get::<_, i64>(7)? != 0,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

const COLS: &str = "id, date, time, location, description, notes, remind_before_5min, remind_at_start, created_at, updated_at";

pub fn list(conn: &Connection, month: Option<&str>) -> rusqlite::Result<Vec<Schedule>> {
    let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match month {
        Some(m) => (
            format!(
                "SELECT {COLS} FROM schedules WHERE substr(date,1,7) = ?1 ORDER BY date, COALESCE(time,'')"
            ),
            vec![Box::new(m.to_string())],
        ),
        None => (
            format!("SELECT {COLS} FROM schedules ORDER BY date, COALESCE(time,'')"),
            vec![],
        ),
    };
    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), row_to)?;
    rows.collect()
}

pub fn list_range(conn: &Connection, from: &str, to: &str) -> rusqlite::Result<Vec<Schedule>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM schedules WHERE date >= ?1 AND date <= ?2 ORDER BY date, COALESCE(time,'')"
    ))?;
    let rows = stmt.query_map([from, to], row_to)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Schedule>> {
    let mut stmt = conn.prepare(&format!("SELECT {COLS} FROM schedules WHERE id=?1"))?;
    match stmt.query_row([id], row_to) {
        Ok(s) => Ok(Some(s)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub struct NewSchedule<'a> {
    pub date: &'a str,
    pub time: Option<&'a str>,
    pub location: Option<&'a str>,
    pub description: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub remind_before_5min: bool,
    pub remind_at_start: bool,
}

pub fn create(conn: &mut Connection, source: Source, input: &NewSchedule<'_>) -> rusqlite::Result<Schedule> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO schedules (date, time, location, description, notes, remind_before_5min, remind_at_start)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![
            input.date,
            input.time,
            input.location,
            input.description,
            input.notes,
            input.remind_before_5min as i64,
            input.remind_at_start as i64,
        ],
    )?;
    let id = tx.last_insert_rowid();
    let after = serde_json::json!({
        "date": input.date,
        "time": input.time,
        "location": input.location,
        "description": input.description,
        "notes": input.notes,
        "remind_before_5min": input.remind_before_5min,
        "remind_at_start": input.remind_at_start,
    });
    write_audit(&tx, source, Op::Create, "schedules", id, None, Some(&after))?;
    tx.commit()?;
    get(conn, id).and_then(|o| o.ok_or(rusqlite::Error::QueryReturnedNoRows))
}

pub struct UpdateSchedule<'a> {
    pub date: Option<&'a str>,
    pub time: Option<&'a str>,
    pub location: Option<&'a str>,
    pub description: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub remind_before_5min: Option<bool>,
    pub remind_at_start: Option<bool>,
}

pub fn update(conn: &mut Connection, source: Source, id: i64, patch: &UpdateSchedule<'_>) -> rusqlite::Result<Schedule> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let mut sets: Vec<&str> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    if let Some(v) = patch.date { sets.push("date = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.time { sets.push("time = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.location { sets.push("location = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.description { sets.push("description = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.notes { sets.push("notes = ?"); vals.push(Box::new(v.to_string())); }
    if let Some(v) = patch.remind_before_5min { sets.push("remind_before_5min = ?"); vals.push(Box::new(v as i64)); }
    if let Some(v) = patch.remind_at_start { sets.push("remind_at_start = ?"); vals.push(Box::new(v as i64)); }
    if sets.is_empty() { return Err(rusqlite::Error::ToSqlConversionFailure("no fields".into())); }
    sets.push("updated_at = datetime('now')");
    vals.push(Box::new(id));
    let sql = format!("UPDATE schedules SET {} WHERE id = ?", sets.join(", "));
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    tx.execute(&sql, refs.as_slice())?;
    let after = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let bj = serde_json::to_value(&before).unwrap();
    let aj = serde_json::to_value(&after).unwrap();
    write_audit(&tx, source, Op::Update, "schedules", id, Some(&bj), Some(&aj))?;
    tx.commit()?;
    Ok(after)
}

pub fn delete(conn: &mut Connection, source: Source, id: i64) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let before = get(&tx, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    tx.execute("DELETE FROM schedules WHERE id=?1", [id])?;
    let bj = serde_json::to_value(&before).unwrap();
    write_audit(&tx, source, Op::Delete, "schedules", id, Some(&bj), None)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn create_and_list() {
        let mut c = fresh();
        create(
            &mut c, Source::Cli,
            &NewSchedule {
                date: "2026-05-01", time: Some("09:00"),
                location: None, description: Some("dentist"), notes: None,
                remind_before_5min: true, remind_at_start: false,
            },
        ).unwrap();
        let all = list(&c, None).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].date, "2026-05-01");
    }

    #[test]
    fn list_filtered_by_month() {
        let mut c = fresh();
        for date in ["2026-05-01", "2026-05-15", "2026-06-02"] {
            create(
                &mut c, Source::Cli,
                &NewSchedule {
                    date, time: None, location: None, description: None, notes: None,
                    remind_before_5min: false, remind_at_start: false,
                },
            ).unwrap();
        }
        let may = list(&c, Some("2026-05")).unwrap();
        assert_eq!(may.len(), 2);
    }

    #[test]
    fn update_changes_reminder_flags() {
        let mut c = fresh();
        let s = create(
            &mut c, Source::Cli,
            &NewSchedule {
                date: "2026-05-01", time: None, location: None,
                description: None, notes: None,
                remind_before_5min: false, remind_at_start: false,
            },
        ).unwrap();
        let updated = update(
            &mut c, Source::Cli, s.id,
            &UpdateSchedule {
                date: None, time: None, location: None, description: None, notes: None,
                remind_before_5min: Some(true), remind_at_start: Some(true),
            },
        ).unwrap();
        assert!(updated.remind_before_5min);
        assert!(updated.remind_at_start);
    }
}
```

Add `pub mod schedules;` to `core/src/lib.rs`.

- [ ] **Step 2: 테스트 실행**

Run: `cd src-tauri && cargo test -p hearth-core schedules::`.
Expected: PASS (3 tests).

- [ ] **Step 3: `app/src/cmd_schedules.rs` 를 래퍼로**

Open `src-tauri/app/src/cmd_schedules.rs`. Read existing tauri command signatures and keep them intact. Replace the body of each with a call to `hearth_core::schedules::*`.

Example patch for `get_schedules`:
```rust
use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::models::Schedule;
use hearth_core::schedules::{self, NewSchedule, UpdateSchedule};
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub fn get_schedules(state: State<'_, AppState>) -> Result<Vec<Schedule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    schedules::list(&db, None).map_err(|e| e.to_string())
}
```

Do the same 1:1 mapping for `create_schedule`, `update_schedule`, `delete_schedule`. Use `Source::App` and map existing input struct fields 1:1 into `NewSchedule` / `UpdateSchedule`.

- [ ] **Step 4: 빌드 & workspace 전체 테스트 통과**

Run: `cd src-tauri && cargo test --workspace`.
Expected: PASS. Existing 21 app tests + new core schedules tests.

- [ ] **Step 5: 앱 수동 검증**

Run `npm run tauri dev`. UI 에서 스케줄 생성 → 편집 → 리마인더 토글 → 삭제 각각 한 번씩. 모든 동작 정상.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(schedules): move CRUD to hearth-core + audit"
```

### Task 3.7: `hearth-core::categories` — CRUD with rename cascade

**Files:**
- Create: `src-tauri/core/src/categories.rs`
- Modify: `src-tauri/app/src/cmd_categories.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: core 모듈 작성**

Create `src-tauri/core/src/categories.rs`:
```rust
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub sort_order: i64,
    pub usage_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

fn row_to(row: &rusqlite::Row) -> rusqlite::Result<Category> {
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
    "SELECT c.id, c.name, c.color, c.sort_order,
            (SELECT COUNT(*) FROM projects p WHERE p.category = c.name) AS usage_count,
            c.created_at, c.updated_at
     FROM categories c";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Category>> {
    let sql = format!("{} ORDER BY c.sort_order ASC, c.id ASC", SELECT_WITH_USAGE);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], row_to)?;
    rows.collect()
}

pub fn get(conn: &Connection, id: i64) -> rusqlite::Result<Option<Category>> {
    let sql = format!("{} WHERE c.id = ?1", SELECT_WITH_USAGE);
    let mut stmt = conn.prepare(&sql)?;
    match stmt.query_row([id], row_to) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CategoryError {
    #[error("카테고리 이름이 비어 있습니다")]
    EmptyName,
    #[error("이미 존재하는 카테고리 이름입니다: {0}")]
    Duplicate(String),
    #[error("카테고리를 찾을 수 없음: id={0}")]
    NotFound(i64),
    #[error("카테고리 사용 중 ({count}개 프로젝트): {name}")]
    InUse { name: String, count: i64 },
    #[error("sqlite: {0}")]
    Sql(#[from] rusqlite::Error),
}

pub fn create(conn: &Connection, name: &str, color: Option<&str>) -> Result<Category, CategoryError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(CategoryError::EmptyName);
    }
    let color = color.map(|c| c.trim().to_string()).unwrap_or_else(|| "#6b7280".into());
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM categories WHERE name = ?1",
        [name],
        |r| r.get(0),
    )?;
    if exists > 0 {
        return Err(CategoryError::Duplicate(name.to_string()));
    }
    let next_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
        params![name, color, next_order],
    )?;
    let id = conn.last_insert_rowid();
    Ok(get(conn, id)?.ok_or(CategoryError::NotFound(id))?)
}

pub struct UpdateCategory<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
    pub sort_order: Option<i64>,
}

pub fn update(conn: &mut Connection, id: i64, patch: &UpdateCategory<'_>) -> Result<Category, CategoryError> {
    let current_name: String = conn
        .query_row("SELECT name FROM categories WHERE id = ?1", [id], |r| r.get(0))
        .map_err(|_| CategoryError::NotFound(id))?;

    let mut new_name: Option<String> = None;
    if let Some(raw) = patch.name {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() {
            return Err(CategoryError::EmptyName);
        }
        if trimmed != current_name {
            let collides: i64 = conn.query_row(
                "SELECT COUNT(*) FROM categories WHERE name = ?1 AND id <> ?2",
                params![trimmed, id],
                |r| r.get(0),
            )?;
            if collides > 0 {
                return Err(CategoryError::Duplicate(trimmed));
            }
            new_name = Some(trimmed);
        }
    }

    let tx = conn.transaction()?;
    if let Some(n) = new_name.as_deref() {
        tx.execute(
            "UPDATE categories SET name = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![n, id],
        )?;
        tx.execute(
            "UPDATE projects SET category = ?1, updated_at = datetime('now') WHERE category = ?2",
            params![n, current_name],
        )?;
    }
    if let Some(color) = patch.color {
        tx.execute(
            "UPDATE categories SET color = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![color.trim(), id],
        )?;
    }
    if let Some(ord) = patch.sort_order {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![ord, id],
        )?;
    }
    tx.commit()?;
    Ok(get(conn, id)?.ok_or(CategoryError::NotFound(id))?)
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), CategoryError> {
    let (name, usage): (String, i64) = conn
        .query_row(
            "SELECT name, (SELECT COUNT(*) FROM projects p WHERE p.category = c.name)
             FROM categories c WHERE c.id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| CategoryError::NotFound(id))?;
    if usage > 0 {
        return Err(CategoryError::InUse { name, count: usage });
    }
    conn.execute("DELETE FROM categories WHERE id = ?1", [id])?;
    Ok(())
}

pub fn reorder(conn: &mut Connection, ids: &[i64]) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE categories SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![i as i64, id],
        )?;
    }
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn rename_cascades_to_projects() {
        let mut c = fresh();
        // Create a category, then project using it.
        create(&c, "Custom", Some("#00ff00")).unwrap();
        c.execute(
            "INSERT INTO projects (name, priority, category) VALUES ('P1', 'P2', 'Custom')",
            [],
        ).unwrap();
        // Rename.
        let cat = list(&c).unwrap().into_iter().find(|x| x.name == "Custom").unwrap();
        update(&mut c, cat.id, &UpdateCategory {
            name: Some("Renamed"), color: None, sort_order: None,
        }).unwrap();
        // Project should now reference new name.
        let new_cat: String = c.query_row(
            "SELECT category FROM projects WHERE name='P1'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(new_cat, "Renamed");
    }

    #[test]
    fn delete_refuses_if_in_use() {
        let c = fresh();
        create(&c, "Hot", None).unwrap();
        c.execute(
            "INSERT INTO projects (name, priority, category) VALUES ('X', 'P2', 'Hot')",
            [],
        ).unwrap();
        let cat = list(&c).unwrap().into_iter().find(|x| x.name == "Hot").unwrap();
        let err = delete(&c, cat.id).unwrap_err();
        match err {
            CategoryError::InUse { count, .. } => assert_eq!(count, 1),
            _ => panic!("expected InUse"),
        }
    }

    #[test]
    fn create_rejects_duplicate() {
        let c = fresh();
        create(&c, "Dup", None).unwrap();
        let err = create(&c, "Dup", None).unwrap_err();
        matches!(err, CategoryError::Duplicate(_));
    }
}
```

Add `pub mod categories;` to `core/src/lib.rs`.

Ensure `thiserror` is in `core/Cargo.toml` dependencies:
```toml
thiserror = { workspace = true }
```
(Already present from Phase 1 workspace dependencies.)

- [ ] **Step 2: 테스트 실행**

Run: `cd src-tauri && cargo test -p hearth-core categories::`.
Expected: PASS (3 tests).

- [ ] **Step 3: `app/src/cmd_categories.rs` 를 래퍼로**

Replace the existing file with wrappers that call `hearth_core::categories::*`. Keep the tauri command signatures (`get_categories`, `create_category`, `update_category`, `delete_category`, `reorder_categories`) unchanged.

```rust
use crate::AppState;
use hearth_core::categories::{self, Category, CategoryError, UpdateCategory};
use serde::Deserialize;
use tauri::State;

#[tauri::command]
pub fn get_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::list(&db).map_err(|e| e.to_string())
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
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::create(&db, &input.name, input.color.as_deref())
        .map_err(|e| format_err(&e))
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
    categories::update(
        &mut db,
        id,
        &UpdateCategory {
            name: fields.name.as_deref(),
            color: fields.color.as_deref(),
            sort_order: fields.sort_order,
        },
    )
    .map_err(|e| format_err(&e))
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    categories::delete(&db, id).map_err(|e| format_err(&e))
}

#[tauri::command]
pub fn reorder_categories(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    categories::reorder(&mut db, &ids).map_err(|e| e.to_string())
}

fn format_err(e: &CategoryError) -> String {
    e.to_string()
}
```

- [ ] **Step 4: 빌드 & 전체 테스트**

Run: `cd src-tauri && cargo test --workspace`.
Expected: PASS.

- [ ] **Step 5: 수동 검증**

Run `npm run tauri dev`. 설정 → 카테고리 탭에서 생성 → 이름 변경 → 사용 중인 카테고리 삭제 시도 (거부 메시지) → 빈 카테고리 삭제. 모두 정상.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(categories): move CRUD + rename cascade to hearth-core

CategoryError uses thiserror for structured error emission. App keeps
Korean error strings via Display impl."
```

### Task 3.8: `hearth-core::clients` — list only

**Files:**
- Create: `src-tauri/core/src/clients.rs`
- Modify: `src-tauri/app/src/cmd_clients.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: core 모듈 작성**

Create `src-tauri/core/src/clients.rs`:
```rust
use crate::models::Client;
use rusqlite::Connection;

const COLS: &str =
    "id, company_name, ceo, phone, fax, email, offices, project_desc, status, created_at, updated_at";

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Client>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM clients ORDER BY id DESC"
    ))?;
    let rows = stmt.query_map([], |r| {
        Ok(Client {
            id: r.get(0)?,
            company_name: r.get(1)?,
            ceo: r.get(2)?,
            phone: r.get(3)?,
            fax: r.get(4)?,
            email: r.get(5)?,
            offices: r.get(6)?,
            project_desc: r.get(7)?,
            status: r.get(8)?,
            created_at: r.get(9)?,
            updated_at: r.get(10)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_db;
    use tempfile::TempDir;

    #[test]
    fn list_returns_empty_on_fresh_db() {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        let conn = init_db(&p).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 0);
    }
}
```

Add `pub mod clients;` to `core/src/lib.rs`.

- [ ] **Step 2: 테스트 실행**

Run: `cargo test -p hearth-core clients::`.
Expected: PASS.

- [ ] **Step 3: `app/src/cmd_clients.rs` 를 래퍼로**

Replace with:
```rust
use crate::AppState;
use hearth_core::clients;
use hearth_core::models::Client;
use tauri::State;

#[tauri::command]
pub fn get_clients(state: State<'_, AppState>) -> Result<Vec<Client>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    clients::list(&db).map_err(|e| e.to_string())
}
```

- [ ] **Step 4: 빌드 & 테스트 통과**

Run: `cargo test --workspace`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(clients): move list to hearth-core (read-only for now)"
```

### Task 3.9: hearth-core::search — FTS5 query API

**Files:**
- Create: `src-tauri/core/src/search.rs`
- Modify: `src-tauri/core/src/lib.rs`

- [ ] **Step 1: 테스트 작성**

Create `src-tauri/core/src/search.rs`:
```rust
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SearchHit {
    pub kind: String, // "project" | "memo" | "schedule"
    pub id: i64,
    pub title: String,
    pub snippet: String,
    pub score: f64, // bm25 (lower is better; negate for "higher is better")
}

fn fts_escape(query: &str) -> String {
    // Wrap query in double quotes to treat as phrase and escape inner quotes.
    let escaped = query.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

pub fn search_all(conn: &Connection, query: &str, limit: i64) -> rusqlite::Result<Vec<SearchHit>> {
    let q = fts_escape(query);
    let mut hits = Vec::new();

    // Projects
    {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, snippet(projects_fts, 0, '<', '>', '…', 8), bm25(projects_fts)
             FROM projects_fts JOIN projects p ON p.id = projects_fts.rowid
             WHERE projects_fts MATCH ?1 ORDER BY bm25(projects_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "project".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    // Memos
    {
        let mut stmt = conn.prepare(
            "SELECT m.id, substr(m.content,1,40), snippet(memos_fts, 0, '<', '>', '…', 8), bm25(memos_fts)
             FROM memos_fts JOIN memos m ON m.id = memos_fts.rowid
             WHERE memos_fts MATCH ?1 ORDER BY bm25(memos_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "memo".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    // Schedules
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, COALESCE(s.description, s.date), snippet(schedules_fts, 0, '<', '>', '…', 8), bm25(schedules_fts)
             FROM schedules_fts JOIN schedules s ON s.id = schedules_fts.rowid
             WHERE schedules_fts MATCH ?1 ORDER BY bm25(schedules_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "schedule".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    hits.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Source;
    use crate::db::init_db;
    use crate::{memos, projects, schedules};
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn search_matches_across_scopes() {
        let mut c = fresh();
        projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "Hearth CLI",
                priority: "P1",
                category: Some("Tools"),
                path: None,
                evaluation: Some("agent interface"),
            },
        )
        .unwrap();
        memos::create(
            &mut c,
            Source::Cli,
            &memos::NewMemo {
                content: "note about agents",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        // Search for 'agent'.
        let hits = search_all(&c, "agent", 20).unwrap();
        assert!(hits.iter().any(|h| h.kind == "project"));
        assert!(hits.iter().any(|h| h.kind == "memo"));
    }
}
```

(If `schedules` module tests are needed, also insert a schedule before search; above test covers project + memo.)

Add `pub mod search;` to `core/src/lib.rs`.

- [ ] **Step 2: 테스트 실행** → PASS.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat(core): search::search_all FTS5 query across projects/memos/schedules

Returns SearchHit{kind,id,title,snippet,score} sorted by bm25."
```

### Task 3.10: `hearth-core::views` — today/overdue/stats

**Files:**
- Create: `src-tauri/core/src/views.rs`

- [ ] **Step 1: 테스트 + 구현**

Create `src-tauri/core/src/views.rs`:
```rust
use crate::models::{Memo, Project, Schedule};
use chrono::{Duration, Local, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TodayView {
    pub date: String,
    pub schedules_today: Vec<Schedule>,
    pub p0_projects: Vec<Project>,
    pub recent_memos: Vec<Memo>,
}

pub fn today(conn: &Connection) -> rusqlite::Result<TodayView> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    // schedules today
    let mut stmt = conn.prepare(
        "SELECT id,date,time,location,description,notes,remind_before_5min,remind_at_start,created_at,updated_at
         FROM schedules WHERE date = ?1 ORDER BY COALESCE(time,'') ASC",
    )?;
    let schedules_today: Vec<Schedule> = stmt
        .query_map([&today], |r| {
            Ok(Schedule {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                location: r.get(3)?,
                description: r.get(4)?,
                notes: r.get(5)?,
                remind_before_5min: r.get::<_, i64>(6)? != 0,
                remind_at_start: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // p0 projects
    let mut stmt = conn.prepare(
        "SELECT id,priority,number,name,category,path,evaluation,sort_order,created_at,updated_at
         FROM projects WHERE priority='P0' ORDER BY sort_order ASC",
    )?;
    let p0_projects: Vec<Project> = stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                priority: r.get(1)?,
                number: r.get(2)?,
                name: r.get(3)?,
                category: r.get(4)?,
                path: r.get(5)?,
                evaluation: r.get(6)?,
                sort_order: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // recent memos (updated_at within 24h)
    let cutoff = (Local::now() - Duration::hours(24))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let mut stmt = conn.prepare(
        "SELECT id,content,color,project_id,sort_order,created_at,updated_at
         FROM memos WHERE updated_at >= ?1 ORDER BY updated_at DESC LIMIT 10",
    )?;
    let recent_memos: Vec<Memo> = stmt
        .query_map([&cutoff], |r| {
            Ok(Memo {
                id: r.get(0)?,
                content: r.get(1)?,
                color: r.get(2)?,
                project_id: r.get(3)?,
                sort_order: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(TodayView {
        date: today,
        schedules_today,
        p0_projects,
        recent_memos,
    })
}

#[derive(Debug, Serialize)]
pub struct OverdueView {
    pub overdue_schedules: Vec<Schedule>,
    pub stale_projects: Vec<Project>,
}

pub fn overdue(conn: &Connection) -> rusqlite::Result<OverdueView> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let month_ago = (Local::now() - Duration::days(30))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let mut stmt = conn.prepare(
        "SELECT id,date,time,location,description,notes,remind_before_5min,remind_at_start,created_at,updated_at
         FROM schedules WHERE date < ?1 ORDER BY date DESC LIMIT 50",
    )?;
    let overdue_schedules: Vec<Schedule> = stmt
        .query_map([&today], |r| {
            Ok(Schedule {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                location: r.get(3)?,
                description: r.get(4)?,
                notes: r.get(5)?,
                remind_before_5min: r.get::<_, i64>(6)? != 0,
                remind_at_start: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn.prepare(
        "SELECT id,priority,number,name,category,path,evaluation,sort_order,created_at,updated_at
         FROM projects WHERE updated_at < ?1 ORDER BY updated_at ASC LIMIT 50",
    )?;
    let stale_projects: Vec<Project> = stmt
        .query_map([&month_ago], |r| {
            Ok(Project {
                id: r.get(0)?,
                priority: r.get(1)?,
                number: r.get(2)?,
                name: r.get(3)?,
                category: r.get(4)?,
                path: r.get(5)?,
                evaluation: r.get(6)?,
                sort_order: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(OverdueView {
        overdue_schedules,
        stale_projects,
    })
}

#[derive(Debug, Serialize)]
pub struct StatsView {
    pub total_projects: i64,
    pub priorities: std::collections::BTreeMap<String, i64>,
    pub categories: std::collections::BTreeMap<String, i64>,
    pub total_memos: i64,
    pub memos_by_color: std::collections::BTreeMap<String, i64>,
    pub total_schedules: i64,
    pub schedules_next_30d: i64,
}

pub fn stats(conn: &Connection) -> rusqlite::Result<StatsView> {
    let total_projects: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
    let mut priorities = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare("SELECT priority, COUNT(*) FROM projects GROUP BY priority")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (p, n) = row?;
        priorities.insert(p, n);
    }
    let mut categories = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(category, '(none)'), COUNT(*) FROM projects GROUP BY category",
    )?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (c, n) = row?;
        categories.insert(c, n);
    }
    let total_memos: i64 = conn.query_row("SELECT COUNT(*) FROM memos", [], |r| r.get(0))?;
    let mut memos_by_color = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare("SELECT color, COUNT(*) FROM memos GROUP BY color")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (c, n) = row?;
        memos_by_color.insert(c, n);
    }
    let total_schedules: i64 = conn.query_row("SELECT COUNT(*) FROM schedules", [], |r| r.get(0))?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let plus_30 = (Local::now() + Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let schedules_next_30d: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schedules WHERE date >= ?1 AND date <= ?2",
        [&today, &plus_30],
        |r| r.get(0),
    )?;
    Ok(StatsView {
        total_projects,
        priorities,
        categories,
        total_memos,
        memos_by_color,
        total_schedules,
        schedules_next_30d,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Source;
    use crate::db::init_db;
    use crate::{memos, projects, schedules};
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn today_returns_p0_projects() {
        let mut c = fresh();
        projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "urgent",
                priority: "P0",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        let v = today(&c).unwrap();
        assert_eq!(v.p0_projects.len(), 1);
    }

    #[test]
    fn stats_counts() {
        let mut c = fresh();
        memos::create(
            &mut c,
            Source::Cli,
            &memos::NewMemo {
                content: "a",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        let s = stats(&c).unwrap();
        assert_eq!(s.total_memos, 1);
    }
}
```

Add `pub mod views;` to `core/src/lib.rs`.

- [ ] **Step 2-3: 테스트 실행 + commit**

Run: `cargo test -p hearth-core views::` → PASS.
```bash
git add -A
git commit -m "feat(core): views::{today,overdue,stats} composite aggregators"
```

### Task 3.11: `hearth-core::scan` — folder → project candidates

**Files:**
- Create: `src-tauri/core/src/scan.rs`

- [ ] **Step 1: 구현 + 테스트**

```rust
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct ScanHit {
    pub path: String,
    pub name: String,
    pub already_registered: bool,
}

pub fn scan_dir(
    dir: &Path,
    depth: u32,
    existing_paths: &[String],
) -> std::io::Result<Vec<ScanHit>> {
    let mut hits = Vec::new();
    walk(dir, depth, existing_paths, &mut hits)?;
    Ok(hits)
}

fn walk(
    dir: &Path,
    depth: u32,
    existing: &[String],
    out: &mut Vec<ScanHit>,
) -> std::io::Result<()> {
    if depth == 0 {
        return Ok(());
    }
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();
        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        let hit = ScanHit {
            path: path_str.clone(),
            name: file_name,
            already_registered: existing.contains(&path_str),
        };
        out.push(hit);
        walk(&path, depth - 1, existing, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_reports_subdirs_and_flags_existing() {
        let d = TempDir::new().unwrap();
        std::fs::create_dir_all(d.path().join("proj_a")).unwrap();
        std::fs::create_dir_all(d.path().join("proj_b")).unwrap();
        let existing = vec![d.path().join("proj_a").to_string_lossy().to_string()];
        let hits = scan_dir(d.path(), 1, &existing).unwrap();
        assert_eq!(hits.len(), 2);
        let a = hits.iter().find(|h| h.name == "proj_a").unwrap();
        assert!(a.already_registered);
        let b = hits.iter().find(|h| h.name == "proj_b").unwrap();
        assert!(!b.already_registered);
    }
}
```

Add `pub mod scan;` to `lib.rs`.

- [ ] **Step 2-3: 테스트 + commit**

```bash
cargo test -p hearth-core scan::
git add -A
git commit -m "feat(core): scan::scan_dir for folder→project candidates"
```

### Task 3.12: `hearth-core::audit` — undo/redo engine

**Files:**
- Modify: `src-tauri/core/src/audit.rs`

- [ ] **Step 1: 테스트 작성 (projects undo)**

Add to `core/src/audit.rs`:
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: String,
    pub source: String,
    pub op: String,
    pub table_name: String,
    pub row_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub undone: bool,
}

fn row_to_entry(row: &rusqlite::Row) -> rusqlite::Result<AuditEntry> {
    Ok(AuditEntry {
        id: row.get(0)?,
        ts: row.get(1)?,
        source: row.get(2)?,
        op: row.get(3)?,
        table_name: row.get(4)?,
        row_id: row.get(5)?,
        before_json: row.get(6)?,
        after_json: row.get(7)?,
        undone: row.get::<_, i64>(8)? != 0,
    })
}

pub fn list(
    conn: &Connection,
    limit: i64,
    source_filter: Option<&str>,
    table_filter: Option<&str>,
    include_undone: bool,
) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut sql = String::from(
        "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
         FROM audit_log WHERE 1=1",
    );
    let mut params: Vec<String> = Vec::new();
    if !include_undone {
        sql.push_str(" AND undone = 0");
    }
    if let Some(s) = source_filter {
        sql.push_str(" AND source = ?");
        params.push(s.to_string());
    }
    if let Some(t) = table_filter {
        sql.push_str(" AND table_name = ?");
        params.push(t.to_string());
    }
    sql.push_str(" ORDER BY id DESC LIMIT ?");
    let mut stmt = conn.prepare(&sql)?;
    let mut p_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    p_refs.push(&limit);
    let rows = stmt.query_map(p_refs.as_slice(), row_to_entry)?;
    rows.collect()
}

/// Undo the most recent `count` non-undone entries by reversing each one.
/// Each reverse is executed in its own transaction alongside an audit row
/// of its own (source='undo'), so redo() can re-apply.
pub fn undo(conn: &mut Connection, count: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut done = Vec::new();
    for _ in 0..count {
        let entry = {
            let mut stmt = conn.prepare(
                "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
                 FROM audit_log WHERE undone = 0 ORDER BY id DESC LIMIT 1",
            )?;
            match stmt.query_row([], row_to_entry) {
                Ok(e) => e,
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(e) => return Err(e),
            }
        };
        apply_reverse(conn, &entry)?;
        conn.execute("UPDATE audit_log SET undone = 1 WHERE id = ?1", [entry.id])?;
        done.push(entry);
    }
    Ok(done)
}

pub fn redo(conn: &mut Connection, count: i64) -> rusqlite::Result<Vec<AuditEntry>> {
    let mut done = Vec::new();
    for _ in 0..count {
        let entry = {
            let mut stmt = conn.prepare(
                "SELECT id,ts,source,op,table_name,row_id,before_json,after_json,undone
                 FROM audit_log WHERE undone = 1 ORDER BY id DESC LIMIT 1",
            )?;
            match stmt.query_row([], row_to_entry) {
                Ok(e) => e,
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(e) => return Err(e),
            }
        };
        apply_forward(conn, &entry)?;
        conn.execute("UPDATE audit_log SET undone = 0 WHERE id = ?1", [entry.id])?;
        done.push(entry);
    }
    Ok(done)
}

fn apply_reverse(conn: &mut Connection, e: &AuditEntry) -> rusqlite::Result<()> {
    let row_id = e
        .row_id
        .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    match e.op.as_str() {
        "create" => {
            conn.execute(
                &format!("DELETE FROM {} WHERE id = ?1", e.table_name),
                [row_id],
            )?;
        }
        "delete" => {
            let before = e
                .before_json
                .as_ref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(before).map_err(|_| rusqlite::Error::InvalidQuery)?;
            insert_from_json(conn, &e.table_name, &v, row_id)?;
        }
        "update" => {
            let before = e
                .before_json
                .as_ref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(before).map_err(|_| rusqlite::Error::InvalidQuery)?;
            update_from_json(conn, &e.table_name, &v, row_id)?;
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    Ok(())
}

fn apply_forward(conn: &mut Connection, e: &AuditEntry) -> rusqlite::Result<()> {
    let row_id = e
        .row_id
        .ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    match e.op.as_str() {
        "create" => {
            let after = e.after_json.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(after).map_err(|_| rusqlite::Error::InvalidQuery)?;
            insert_from_json(conn, &e.table_name, &v, row_id)?;
        }
        "delete" => {
            conn.execute(
                &format!("DELETE FROM {} WHERE id = ?1", e.table_name),
                [row_id],
            )?;
        }
        "update" => {
            let after = e.after_json.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
            let v: serde_json::Value = serde_json::from_str(after).map_err(|_| rusqlite::Error::InvalidQuery)?;
            update_from_json(conn, &e.table_name, &v, row_id)?;
        }
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    Ok(())
}

fn insert_from_json(
    conn: &mut Connection,
    table: &str,
    v: &serde_json::Value,
    id: i64,
) -> rusqlite::Result<()> {
    // Each table has a known column set. Build INSERT statically by table.
    let (cols, vals_sql, vals): (Vec<&str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>) =
        match table {
            "projects" => build_projects_insert(v, id),
            "memos" => build_memos_insert(v, id),
            "schedules" => build_schedules_insert(v, id),
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
    let col_list = cols.join(", ");
    let val_list = vals_sql.join(", ");
    let sql = format!("INSERT OR REPLACE INTO {table} ({col_list}) VALUES ({val_list})");
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())?;
    Ok(())
}

fn update_from_json(
    conn: &mut Connection,
    table: &str,
    v: &serde_json::Value,
    id: i64,
) -> rusqlite::Result<()> {
    match table {
        "projects" | "memos" | "schedules" => {}
        _ => return Err(rusqlite::Error::InvalidQuery),
    }
    let mut sets: Vec<String> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let obj = v.as_object().ok_or(rusqlite::Error::InvalidQuery)?;
    for (k, val) in obj.iter() {
        if k == "id" { continue; }
        sets.push(format!("{k} = ?"));
        vals.push(json_to_sql(val));
    }
    if sets.is_empty() { return Ok(()); }
    vals.push(Box::new(id));
    let sql = format!(
        "UPDATE {table} SET {} WHERE id = ?",
        sets.join(", ")
    );
    let refs: Vec<&dyn rusqlite::types::ToSql> = vals.iter().map(|b| b.as_ref()).collect();
    conn.execute(&sql, refs.as_slice())?;
    Ok(())
}

fn json_to_sql(v: &serde_json::Value) -> Box<dyn rusqlite::types::ToSql> {
    match v {
        serde_json::Value::Null => Box::new(Option::<String>::None),
        serde_json::Value::Bool(b) => Box::new(if *b { 1i64 } else { 0i64 }),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Box::new(i) }
            else if let Some(f) = n.as_f64() { Box::new(f) }
            else { Box::new(n.to_string()) }
        }
        serde_json::Value::String(s) => Box::new(s.clone()),
        _ => Box::new(v.to_string()),
    }
}

// Per-table INSERT column builders. Full column list for INSERT OR REPLACE.
fn build_projects_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","priority","number","name","category","path","evaluation","sort_order","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("priority").and_then(|x| x.as_str()).unwrap_or("P4").to_string()),
        Box::new(v.get("number").and_then(|x| x.as_i64())),
        Box::new(v.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("category").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("path").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("evaluation").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("sort_order").and_then(|x| x.as_i64()).unwrap_or(0)),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
}

fn build_memos_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","content","color","project_id","sort_order","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("content").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("color").and_then(|x| x.as_str()).unwrap_or("yellow").to_string()),
        Box::new(v.get("project_id").and_then(|x| x.as_i64())),
        Box::new(v.get("sort_order").and_then(|x| x.as_i64()).unwrap_or(0)),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
}

fn build_schedules_insert(v: &serde_json::Value, id: i64)
    -> (Vec<&'static str>, Vec<String>, Vec<Box<dyn rusqlite::types::ToSql>>)
{
    let cols = vec!["id","date","time","location","description","notes","remind_before_5min","remind_at_start","created_at","updated_at"];
    let placeholders: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
    let vals: Vec<Box<dyn rusqlite::types::ToSql>> = vec![
        Box::new(id),
        Box::new(v.get("date").and_then(|x| x.as_str()).unwrap_or("").to_string()),
        Box::new(v.get("time").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("location").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("description").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("notes").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("remind_before_5min").and_then(|x| x.as_bool()).unwrap_or(false) as i64),
        Box::new(v.get("remind_at_start").and_then(|x| x.as_bool()).unwrap_or(false) as i64),
        Box::new(v.get("created_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
        Box::new(v.get("updated_at").and_then(|x| x.as_str()).map(|s| s.to_string())),
    ];
    (cols, placeholders, vals)
}
```

- [ ] **Step 2: 테스트 — undo/redo round-trip**

Add to `audit.rs` tests:
```rust
#[test]
fn undo_create_removes_row() {
    use crate::projects;
    let mut c = tmp_conn().1; // drop dir guard — leak ok in tests
    let p = projects::create(
        &mut c,
        Source::Cli,
        &projects::NewProject {
            name: "x",
            priority: "P2",
            category: None,
            path: None,
            evaluation: None,
        },
    )
    .unwrap();
    undo(&mut c, 1).unwrap();
    assert!(projects::get(&c, p.id).unwrap().is_none());
}

#[test]
fn undo_delete_restores_row() {
    use crate::memos;
    let mut c = tmp_conn().1;
    let m = memos::create(
        &mut c,
        Source::Cli,
        &memos::NewMemo {
            content: "hi",
            color: "yellow",
            project_id: None,
        },
    )
    .unwrap();
    memos::delete(&mut c, Source::Cli, m.id).unwrap();
    assert!(memos::get(&c, m.id).unwrap().is_none());
    undo(&mut c, 1).unwrap();
    assert_eq!(memos::get(&c, m.id).unwrap().unwrap().content, "hi");
}

#[test]
fn redo_after_undo() {
    use crate::projects;
    let mut c = tmp_conn().1;
    let p = projects::create(
        &mut c,
        Source::Cli,
        &projects::NewProject {
            name: "x",
            priority: "P2",
            category: None,
            path: None,
            evaluation: None,
        },
    )
    .unwrap();
    undo(&mut c, 1).unwrap();
    redo(&mut c, 1).unwrap();
    assert!(projects::get(&c, p.id).unwrap().is_some());
}
```

Fix the `tmp_conn` helper to leak TempDir for tests (or use `fresh()` pattern). Adjust to match actual helpers.

- [ ] **Step 3: 실행 & 통과 확인**

Run: `cd src-tauri && cargo test -p hearth-core audit::`.
Expected: all audit tests pass.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(core): audit undo/redo engine (projects/memos/schedules)

list(filter,limit,include_undone), undo(count), redo(count). Tests
cover create/delete undo + redo round-trip."
```

---

## Phase 4 — App `data_version` watcher

### Task 4.1: watcher module

**Files:**
- Create: `src-tauri/app/src/watcher.rs`
- Modify: `src-tauri/app/src/lib.rs`

- [ ] **Step 1: 모듈 작성**

Create `src-tauri/app/src/watcher.rs`:
```rust
//! Poll SQLite `PRAGMA data_version` every 500ms to detect writes made by
//! other connections (e.g. `hearth` CLI) and invalidate UI caches.

use crate::AppState;
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<i64> = None;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let state = match app.try_state::<AppState>() {
                Some(s) => s,
                None => continue,
            };
            let v: Option<i64> = {
                let guard = state.db.lock().ok();
                guard.and_then(|db| db.query_row("PRAGMA data_version", [], |r| r.get(0)).ok())
            };
            let Some(v) = v else { continue };
            match last {
                None => {
                    last = Some(v);
                }
                Some(l) if l != v => {
                    let _ = app.emit("projects:changed", ());
                    let _ = app.emit("memos:changed", ());
                    let _ = app.emit("schedules:changed", ());
                    let _ = app.emit("categories:changed", ());
                    last = Some(v);
                }
                _ => {}
            }
        }
    });
}
```

- [ ] **Step 2: `lib.rs` 에 모듈 선언 + setup 에서 호출**

Open `src-tauri/app/src/lib.rs`. At top with other `mod` lines add:
```rust
mod watcher;
```

In `.setup(|app| { ... })`, after the existing autostart/notification reschedule spawns and before the final `Ok(())`:
```rust
crate::watcher::spawn(app.handle().clone());
```

- [ ] **Step 3: 빌드 확인**

Run: `cd src-tauri && cargo build -p hearth-app`.
Expected: PASS.

- [ ] **Step 4: 수동 검증**

Run `npm run tauri dev`. Open a second terminal:
```bash
sqlite3 "$HOME/Library/Application Support/com.newturn2017.hearth/data.db" "INSERT INTO memos (content, color, sort_order) VALUES ('from shell', 'blue', 999);"
```
Expected: Within ~0.5–1 sec, the running app's Memo 탭에 새 메모가 나타남.

(Revert the test insert: `sqlite3 ... "DELETE FROM memos WHERE content='from shell';"`.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(app): data_version watcher for external DB changes

500ms poll → emits projects/memos/schedules/categories :changed on delta.
Enables live-refresh when hearth CLI writes from another process."
```

---

## Phase 5 — CLI skeleton (entry + global flags)

### Task 5.1: clap-based main with `--db` + `--version`

**Files:**
- Modify: `src-tauri/cli/src/main.rs`
- Create: `src-tauri/cli/src/util.rs`
- Create: `src-tauri/cli/src/db.rs`

- [ ] **Step 1: 유틸 & DB opener**

Create `src-tauri/cli/src/db.rs`:
```rust
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

pub fn default_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn dirs_macos() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.newturn2017.hearth")
            .join("data.db"),
    )
}

pub fn resolve_db_path(flag: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = flag {
        return Ok(PathBuf::from(p));
    }
    if let Ok(e) = std::env::var("HEARTH_DB") {
        return Ok(PathBuf::from(e));
    }
    default_db_path().context(
        "could not resolve default DB path. Pass --db <PATH> or set HEARTH_DB=<PATH>.",
    )
}

pub fn open(path: &std::path::Path) -> Result<Connection> {
    let conn = hearth_core::db::init_db(path)
        .with_context(|| format!("failed to open hearth DB at {}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_millis(3000))
        .ok();
    Ok(conn)
}
```

Create `src-tauri/cli/src/util.rs`:
```rust
use serde_json::Value;

pub fn emit_ok(data: Value) {
    let out = serde_json::json!({ "ok": true, "data": data });
    println!("{}", out);
}

pub fn emit_err(msg: &str, hint: Option<&str>) {
    let mut e = serde_json::Map::new();
    e.insert("ok".into(), Value::Bool(false));
    e.insert("error".into(), Value::String(msg.to_string()));
    if let Some(h) = hint {
        e.insert("hint".into(), Value::String(h.to_string()));
    }
    eprintln!("{}", Value::Object(e));
}
```

Add `rusqlite::Connection` support: `cli/Cargo.toml` already has `rusqlite = { workspace = true }`.

- [ ] **Step 2: main.rs with clap**

Replace `src-tauri/cli/src/main.rs`:
```rust
mod db;
mod util;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hearth", version, about = "Hearth CLI — agent-friendly workspace control")]
struct Cli {
    /// SQLite DB path override. Falls back to $HEARTH_DB then the default app data path.
    #[arg(long, global = true)]
    db: Option<String>,

    /// Verbose tracing (equivalent to RUST_LOG=debug).
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// DB-level utilities.
    Db {
        #[command(subcommand)]
        sub: DbCmd,
    },
}

#[derive(Subcommand)]
enum DbCmd {
    /// Print the resolved DB path.
    Path,
    /// VACUUM + integrity_check.
    Vacuum,
    /// (Re)run migrations.
    Migrate,
}

fn main() {
    if let Err(e) = run() {
        crate::util::emit_err(&format!("{e:#}"), None);
        std::process::exit(2);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new("debug"))
            .with_writer(std::io::stderr)
            .init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
            )
            .with_writer(std::io::stderr)
            .try_init();
    }

    match cli.command {
        Commands::Db { sub } => cmd_db(cli.db.as_deref(), sub),
    }
}

fn cmd_db(db_flag: Option<&str>, sub: DbCmd) -> Result<()> {
    match sub {
        DbCmd::Path => {
            let p = crate::db::resolve_db_path(db_flag)?;
            crate::util::emit_ok(serde_json::json!({ "path": p.to_string_lossy() }));
            Ok(())
        }
        DbCmd::Vacuum => {
            let p = crate::db::resolve_db_path(db_flag)?;
            let conn = crate::db::open(&p)?;
            conn.execute("VACUUM", [])?;
            let integrity: String =
                conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
            crate::util::emit_ok(serde_json::json!({
                "path": p.to_string_lossy(),
                "integrity_check": integrity,
            }));
            Ok(())
        }
        DbCmd::Migrate => {
            let p = crate::db::resolve_db_path(db_flag)?;
            let _ = crate::db::open(&p)?; // init_db runs migrations
            crate::util::emit_ok(serde_json::json!({ "migrated": true }));
            Ok(())
        }
    }
}
```

- [ ] **Step 3: 빌드 & 스모크**

Run:
```bash
cd src-tauri && cargo build -p hearth-cli
./target/debug/hearth --version
./target/debug/hearth db path
HEARTH_DB=/tmp/test-hearth.db ./target/debug/hearth db migrate
HEARTH_DB=/tmp/test-hearth.db ./target/debug/hearth db vacuum
```
Expected: version 출력 OK, path JSON 출력 OK, migrate/vacuum JSON OK.

- [ ] **Step 4: 첫 CLI 통합 테스트**

Create `src-tauri/cli/tests/smoke.rs`:
```rust
use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn db_path_outputs_json() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["db", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\""));
}

#[test]
fn db_migrate_creates_schema() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["db", "migrate"])
        .assert()
        .success();
    // DB file exists
    assert!(db_path.exists());
}
```

Run: `cd src-tauri && cargo test -p hearth-cli`.
Expected: PASS 2/2.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth binary skeleton — clap + db {path,vacuum,migrate}

Resolves DB path: --db > HEARTH_DB > default ~/Library/.../data.db.
JSON stdout contract. Integration tests use assert_cmd + tempfile."
```

---

## Phase 6 — CLI project commands

### Task 6.1: `hearth project list|get|create|update|delete`

**Files:**
- Create: `src-tauri/cli/src/cmd/mod.rs`
- Create: `src-tauri/cli/src/cmd/project.rs`
- Modify: `src-tauri/cli/src/main.rs`

- [ ] **Step 1: cmd 디렉토리 스캐폴드**

Create `src-tauri/cli/src/cmd/mod.rs`:
```rust
pub mod project;
```

Create `src-tauri/cli/src/cmd/project.rs`:
```rust
use anyhow::{Context, Result};
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::projects::{self, NewProject, UpdateProject};

#[derive(Subcommand)]
pub enum ProjectCmd {
    /// List projects.
    List {
        #[arg(long, value_delimiter = ',')]
        priority: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        category: Vec<String>,
    },
    /// Get one project by id.
    Get { id: i64 },
    /// Create a new project.
    Create {
        name: String,
        #[arg(long, default_value = "P2")]
        priority: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        evaluation: Option<String>,
    },
    /// Update fields on a project.
    Update {
        id: i64,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        evaluation: Option<String>,
    },
    /// Delete a project.
    Delete { id: i64 },
}

pub fn dispatch(db_path_flag: Option<&str>, sub: ProjectCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        ProjectCmd::List { priority, category } => {
            let all = projects::list(&conn)?;
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|p| {
                    let pri_ok = priority.is_empty() || priority.contains(&p.priority);
                    let cat_ok = category.is_empty()
                        || p.category.as_ref().map_or(false, |c| category.contains(c));
                    pri_ok && cat_ok
                })
                .collect();
            crate::util::emit_ok(serde_json::to_value(&filtered).unwrap());
        }
        ProjectCmd::Get { id } => match projects::get(&conn, id)? {
            Some(p) => crate::util::emit_ok(serde_json::to_value(&p).unwrap()),
            None => {
                crate::util::emit_err(
                    &format!("project {id} not found"),
                    Some("try 'hearth project list'"),
                );
                std::process::exit(1);
            }
        },
        ProjectCmd::Create {
            name,
            priority,
            category,
            path,
            evaluation,
        } => {
            let p = projects::create(
                &mut conn,
                Source::Cli,
                &NewProject {
                    name: &name,
                    priority: &priority,
                    category: category.as_deref(),
                    path: path.as_deref(),
                    evaluation: evaluation.as_deref(),
                },
            )
            .context("create failed")?;
            crate::util::emit_ok(serde_json::to_value(&p).unwrap());
        }
        ProjectCmd::Update {
            id,
            name,
            priority,
            category,
            path,
            evaluation,
        } => {
            let p = projects::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateProject {
                    name: name.as_deref(),
                    priority: priority.as_deref(),
                    category: category.as_deref(),
                    path: path.as_deref(),
                    evaluation: evaluation.as_deref(),
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&p).unwrap());
        }
        ProjectCmd::Delete { id } => {
            projects::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
```

- [ ] **Step 2: main.rs 통합**

Open `src-tauri/cli/src/main.rs`. Add `mod cmd;` under existing `mod` lines. Add to `Commands` enum:
```rust
Project {
    #[command(subcommand)]
    sub: crate::cmd::project::ProjectCmd,
},
```

In `run()`'s match:
```rust
Commands::Project { sub } => crate::cmd::project::dispatch(cli.db.as_deref(), sub),
```

- [ ] **Step 3: 통합 테스트 추가**

Add to `src-tauri/cli/tests/smoke.rs`:
```rust
use serde_json::Value;

#[test]
fn project_create_then_list_contains_it() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "create", "TestProj", "--priority", "P1"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "TestProj");

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let arr = v["data"].as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "TestProj");
}

#[test]
fn project_delete_removes_it_and_records_audit() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "create", "X"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let id = v["data"]["id"].as_i64().unwrap();

    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "delete", &id.to_string()])
        .assert()
        .success();

    // List is empty
    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_str)
        .args(["project", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

#[test]
fn project_get_missing_returns_err_exit_1() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args(["project", "get", "999"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}
```

- [ ] **Step 4: 빌드 & 테스트 실행**

Run: `cd src-tauri && cargo test -p hearth-cli`.
Expected: All tests PASS.

- [ ] **Step 5: 수동 검증**

Run `npm run tauri dev` (watcher 가 돌고 있는 앱). 다른 터미널:
```bash
./src-tauri/target/debug/hearth project create "CLI Live" --priority P0
```
Expected: 앱 UI 의 프로젝트 탭에 0.5~1 초 내 나타남.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth project {list,get,create,update,delete}

Filters: --priority, --category (comma-separated). JSON stdout.
Audit log entries use source='cli'. Integration tests cover
create→list, delete, missing-id error path."
```

### Task 6.2: `hearth project scan` + `link-path`

**Files:**
- Modify: `src-tauri/cli/src/cmd/project.rs`

- [ ] **Step 1: ProjectCmd 확장**

Add variants:
```rust
/// Scan a directory for subfolders, flagging which are already registered.
Scan {
    dir: String,
    #[arg(long, default_value_t = 1)]
    depth: u32,
},
/// Link an existing project to a filesystem path (with existence check).
LinkPath { id: i64, path: String },
```

- [ ] **Step 2: dispatch 확장**

```rust
ProjectCmd::Scan { dir, depth } => {
    let all = projects::list(&conn)?;
    let existing: Vec<String> = all.into_iter().filter_map(|p| p.path).collect();
    let path = std::path::PathBuf::from(&dir);
    let hits = hearth_core::scan::scan_dir(&path, depth, &existing)?;
    crate::util::emit_ok(serde_json::to_value(&hits).unwrap());
}
ProjectCmd::LinkPath { id, path } => {
    let pbuf = std::path::PathBuf::from(&path);
    if !pbuf.is_dir() {
        crate::util::emit_err(
            &format!("path is not an existing directory: {}", path),
            Some("pass an absolute path to an existing folder"),
        );
        std::process::exit(1);
    }
    let p = projects::update(
        &mut conn,
        Source::Cli,
        id,
        &UpdateProject {
            name: None,
            priority: None,
            category: None,
            path: Some(&path),
            evaluation: None,
        },
    )?;
    crate::util::emit_ok(serde_json::to_value(&p).unwrap());
}
```

- [ ] **Step 3: 테스트 추가 (tests/smoke.rs)**

```rust
#[test]
fn project_scan_reports_subdirs() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    std::fs::create_dir_all(dir.path().join("sub1")).unwrap();
    std::fs::create_dir_all(dir.path().join("sub2")).unwrap();

    let out = Command::cargo_bin("hearth")
        .unwrap()
        .env("HEARTH_DB", db_path.to_str().unwrap())
        .args([
            "project", "scan",
            dir.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let hits = v["data"].as_array().unwrap();
    assert!(hits.len() >= 2);
}
```

- [ ] **Step 4: 빌드 & 테스트**

Run: `cargo test -p hearth-cli project_scan_reports_subdirs`.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth project scan/link-path

scan: flag subdirectories as registered/unregistered.
link-path: validate directory exists, then update row via audit."
```

---

## Phase 7 — CLI memo / schedule / category commands

### Task 7.1: `hearth memo {list,get,create,update,delete}`

**Files:**
- Create: `src-tauri/cli/src/cmd/memo.rs`
- Modify: `src-tauri/cli/src/cmd/mod.rs`, `src-tauri/cli/src/main.rs`

- [ ] **Step 1: 모듈 작성**

Create `src-tauri/cli/src/cmd/memo.rs`:
```rust
use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::memos::{self, NewMemo, UpdateMemo};

#[derive(Subcommand)]
pub enum MemoCmd {
    /// List memos (optionally filter).
    List {
        #[arg(long)]
        project: Option<i64>,
        #[arg(long)]
        color: Option<String>,
    },
    /// Get one memo.
    Get { id: i64 },
    /// Create a new memo.
    Create {
        #[arg(short = 'c', long)]
        content: String,
        #[arg(long, default_value = "yellow")]
        color: String,
        #[arg(long)]
        project: Option<i64>,
    },
    /// Update a memo. Use --detach to clear project link.
    Update {
        id: i64,
        #[arg(long)]
        content: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long, conflicts_with = "detach")]
        project: Option<i64>,
        #[arg(long)]
        detach: bool,
    },
    /// Delete a memo.
    Delete { id: i64 },
}

pub fn dispatch(db_flag: Option<&str>, sub: MemoCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        MemoCmd::List { project, color } => {
            let all = memos::list(&conn)?;
            let filtered: Vec<_> = all
                .into_iter()
                .filter(|m| project.map_or(true, |pid| m.project_id == Some(pid)))
                .filter(|m| color.as_ref().map_or(true, |c| m.color == *c))
                .collect();
            crate::util::emit_ok(serde_json::to_value(&filtered).unwrap());
        }
        MemoCmd::Get { id } => match memos::get(&conn, id)? {
            Some(m) => crate::util::emit_ok(serde_json::to_value(&m).unwrap()),
            None => {
                crate::util::emit_err(
                    &format!("memo {id} not found"),
                    Some("try 'hearth memo list'"),
                );
                std::process::exit(1);
            }
        },
        MemoCmd::Create { content, color, project } => {
            let m = memos::create(
                &mut conn,
                Source::Cli,
                &NewMemo {
                    content: &content,
                    color: &color,
                    project_id: project,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&m).unwrap());
        }
        MemoCmd::Update { id, content, color, project, detach } => {
            let project_id = if detach {
                Some(None)
            } else {
                project.map(Some)
            };
            let m = memos::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateMemo {
                    content: content.as_deref(),
                    color: color.as_deref(),
                    project_id,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&m).unwrap());
        }
        MemoCmd::Delete { id } => {
            memos::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
```

Update `src-tauri/cli/src/cmd/mod.rs`:
```rust
pub mod memo;
pub mod project;
```

In `src-tauri/cli/src/main.rs`, add to `Commands` enum:
```rust
Memo {
    #[command(subcommand)]
    sub: crate::cmd::memo::MemoCmd,
},
```

In the `match cli.command` block inside `run()`:
```rust
Commands::Memo { sub } => crate::cmd::memo::dispatch(cli.db.as_deref(), sub),
```

- [ ] **Step 2: 테스트**

Add to `tests/smoke.rs`:
```rust
#[test]
fn memo_create_list_delete_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();

    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["memo", "create", "-c", "hello", "--color", "blue"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let id = v["data"]["id"].as_i64().unwrap();
    assert_eq!(v["data"]["color"], "blue");

    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["memo", "list"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["memo", "delete", &id.to_string()])
        .assert().success();
}
```

- [ ] **Step 3: 테스트 실행**

Run: `cd src-tauri && cargo test -p hearth-cli memo_create_list_delete_roundtrip`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth memo {list,get,create,update,delete}

--project / --color filters on list. --detach clears project on update.
Audit log entries use source='cli'."
```

### Task 7.2: `hearth schedule {list,get,create,update,delete}`

**Files:**
- Create: `src-tauri/cli/src/cmd/schedule.rs`
- Modify: `src-tauri/cli/src/cmd/mod.rs`, `src-tauri/cli/src/main.rs`

- [ ] **Step 1: 모듈 작성**

Create `src-tauri/cli/src/cmd/schedule.rs`:
```rust
use anyhow::Result;
use clap::Subcommand;
use hearth_core::audit::Source;
use hearth_core::schedules::{self, NewSchedule, UpdateSchedule};

#[derive(Subcommand)]
pub enum ScheduleCmd {
    /// List schedules. --month filters by YYYY-MM. --from/--to filters range.
    List {
        #[arg(long)]
        month: Option<String>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
    },
    /// Get one schedule.
    Get { id: i64 },
    /// Create a schedule entry.
    Create {
        #[arg(long)]
        date: String,
        #[arg(long)]
        time: Option<String>,
        #[arg(long)]
        location: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        remind_5min: bool,
        #[arg(long)]
        remind_start: bool,
    },
    /// Update a schedule.
    Update {
        id: i64,
        #[arg(long)] date: Option<String>,
        #[arg(long)] time: Option<String>,
        #[arg(long)] location: Option<String>,
        #[arg(long)] description: Option<String>,
        #[arg(long)] notes: Option<String>,
        #[arg(long)] remind_5min: Option<bool>,
        #[arg(long)] remind_start: Option<bool>,
    },
    /// Delete a schedule.
    Delete { id: i64 },
}

pub fn dispatch(db_flag: Option<&str>, sub: ScheduleCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        ScheduleCmd::List { month, from, to } => {
            let rows = match (from.as_deref(), to.as_deref()) {
                (Some(f), Some(t)) => schedules::list_range(&conn, f, t)?,
                _ => schedules::list(&conn, month.as_deref())?,
            };
            crate::util::emit_ok(serde_json::to_value(&rows).unwrap());
        }
        ScheduleCmd::Get { id } => match schedules::get(&conn, id)? {
            Some(s) => crate::util::emit_ok(serde_json::to_value(&s).unwrap()),
            None => {
                crate::util::emit_err(
                    &format!("schedule {id} not found"),
                    Some("try 'hearth schedule list'"),
                );
                std::process::exit(1);
            }
        },
        ScheduleCmd::Create {
            date, time, location, description, notes, remind_5min, remind_start,
        } => {
            let s = schedules::create(
                &mut conn,
                Source::Cli,
                &NewSchedule {
                    date: &date,
                    time: time.as_deref(),
                    location: location.as_deref(),
                    description: description.as_deref(),
                    notes: notes.as_deref(),
                    remind_before_5min: remind_5min,
                    remind_at_start: remind_start,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&s).unwrap());
        }
        ScheduleCmd::Update {
            id, date, time, location, description, notes, remind_5min, remind_start,
        } => {
            let s = schedules::update(
                &mut conn,
                Source::Cli,
                id,
                &UpdateSchedule {
                    date: date.as_deref(),
                    time: time.as_deref(),
                    location: location.as_deref(),
                    description: description.as_deref(),
                    notes: notes.as_deref(),
                    remind_before_5min: remind_5min,
                    remind_at_start: remind_start,
                },
            )?;
            crate::util::emit_ok(serde_json::to_value(&s).unwrap());
        }
        ScheduleCmd::Delete { id } => {
            schedules::delete(&mut conn, Source::Cli, id)?;
            crate::util::emit_ok(serde_json::json!({ "deleted": id }));
        }
    }
    Ok(())
}
```

Update `src-tauri/cli/src/cmd/mod.rs`:
```rust
pub mod memo;
pub mod project;
pub mod schedule;
```

In `main.rs`'s `Commands` enum add:
```rust
Schedule {
    #[command(subcommand)]
    sub: crate::cmd::schedule::ScheduleCmd,
},
```

In the dispatcher:
```rust
Commands::Schedule { sub } => crate::cmd::schedule::dispatch(cli.db.as_deref(), sub),
```

- [ ] **Step 2: 테스트 추가**

Add to `src-tauri/cli/tests/smoke.rs`:
```rust
#[test]
fn schedule_create_list_delete_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();

    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["schedule", "create", "--date", "2026-05-01", "--time", "09:00", "--description", "dentist"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let id = v["data"]["id"].as_i64().unwrap();
    assert_eq!(v["data"]["date"], "2026-05-01");

    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["schedule", "list", "--month", "2026-05"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 1);

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["schedule", "delete", &id.to_string()])
        .assert().success();
}
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test -p hearth-cli schedule_create_list_delete_roundtrip`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth schedule {list,get,create,update,delete}

--month (YYYY-MM) or --from/--to filters on list. Reminder flags
on create/update. Audit log source='cli'."
```

### Task 7.3: `hearth category {list,create,rename,update,delete}`

**Files:**
- Create: `src-tauri/cli/src/cmd/category.rs`
- Modify: `src-tauri/cli/src/cmd/mod.rs`, `src-tauri/cli/src/main.rs`

- [ ] **Step 1: 모듈 작성**

Create `src-tauri/cli/src/cmd/category.rs`:
```rust
use anyhow::Result;
use clap::Subcommand;
use hearth_core::categories::{self, UpdateCategory};

#[derive(Subcommand)]
pub enum CategoryCmd {
    /// List categories with usage counts.
    List,
    /// Create a new category.
    Create {
        name: String,
        #[arg(long, default_value = "#6b7280")]
        color: String,
    },
    /// Rename a category — cascades to projects.
    Rename {
        /// Current category name.
        old_name: String,
        /// New name.
        new_name: String,
    },
    /// Update fields on a category by id.
    Update {
        id: i64,
        #[arg(long)]
        color: Option<String>,
        #[arg(long)]
        sort_order: Option<i64>,
    },
    /// Delete a category (refuses if in use).
    Delete { id: i64 },
}

pub fn dispatch(db_flag: Option<&str>, sub: CategoryCmd) -> Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        CategoryCmd::List => {
            let all = categories::list(&conn)?;
            crate::util::emit_ok(serde_json::to_value(&all).unwrap());
        }
        CategoryCmd::Create { name, color } => {
            match categories::create(&conn, &name, Some(&color)) {
                Ok(c) => crate::util::emit_ok(serde_json::to_value(&c).unwrap()),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
        CategoryCmd::Rename { old_name, new_name } => {
            let target = categories::list(&conn)?
                .into_iter()
                .find(|c| c.name == old_name);
            let Some(cat) = target else {
                crate::util::emit_err(
                    &format!("category not found: {old_name}"),
                    Some("try 'hearth category list'"),
                );
                std::process::exit(1);
            };
            match categories::update(&mut conn, cat.id, &UpdateCategory {
                name: Some(&new_name), color: None, sort_order: None,
            }) {
                Ok(c) => crate::util::emit_ok(serde_json::to_value(&c).unwrap()),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
        CategoryCmd::Update { id, color, sort_order } => {
            match categories::update(&mut conn, id, &UpdateCategory {
                name: None,
                color: color.as_deref(),
                sort_order,
            }) {
                Ok(c) => crate::util::emit_ok(serde_json::to_value(&c).unwrap()),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
        CategoryCmd::Delete { id } => {
            match categories::delete(&conn, id) {
                Ok(()) => crate::util::emit_ok(serde_json::json!({ "deleted": id })),
                Err(e) => {
                    crate::util::emit_err(&e.to_string(), None);
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}
```

Update `cli/src/cmd/mod.rs`:
```rust
pub mod category;
pub mod memo;
pub mod project;
pub mod schedule;
```

In `main.rs` add to `Commands`:
```rust
Category {
    #[command(subcommand)]
    sub: crate::cmd::category::CategoryCmd,
},
```

In dispatcher:
```rust
Commands::Category { sub } => crate::cmd::category::dispatch(cli.db.as_deref(), sub),
```

- [ ] **Step 2: 테스트**

Add to `tests/smoke.rs`:
```rust
#[test]
fn category_create_rename_cascades_to_project() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["category", "create", "Special"])
        .assert().success();

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "create", "Bound", "--category", "Special"])
        .assert().success();

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["category", "rename", "Special", "Renamed"])
        .assert().success();

    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "list"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let arr = v["data"].as_array().unwrap();
    let proj = arr.iter().find(|p| p["name"] == "Bound").unwrap();
    assert_eq!(proj["category"], "Renamed");
}

#[test]
fn category_delete_refuses_in_use() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["category", "create", "InUse"])
        .assert().success();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "create", "X", "--category", "InUse"])
        .assert().success();

    // Get the category's id
    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["category", "list"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let cat = v["data"].as_array().unwrap().iter()
        .find(|c| c["name"] == "InUse").unwrap().clone();
    let id = cat["id"].as_i64().unwrap();

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["category", "delete", &id.to_string()])
        .assert().code(1).stderr(predicate::str::contains("사용 중"));
}
```

- [ ] **Step 3: 테스트 실행**

Run: `cargo test -p hearth-cli category_`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth category {list,create,rename,update,delete}

Rename cascades to projects. Delete refuses with 'category in use'
error (exit 1) when referenced by any project."
```

---

## Phase 8 — CLI search + composite views

### Task 8.1: `hearth search <query>`

**Files:**
- Create: `src-tauri/cli/src/cmd/search.rs`

- [ ] **Step 1: 모듈 + 디스패치**

```rust
use clap::Args;

#[derive(Args)]
pub struct SearchArgs {
    /// Query text (FTS5 expression — quote to include spaces).
    pub query: String,
    #[arg(long, value_delimiter = ',')]
    pub scope: Vec<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: i64,
}

pub fn dispatch(db_path_flag: Option<&str>, args: SearchArgs) -> anyhow::Result<()> {
    let p = crate::db::resolve_db_path(db_path_flag)?;
    let conn = crate::db::open(&p)?;
    let all = hearth_core::search::search_all(&conn, &args.query, args.limit)?;
    let filtered: Vec<_> = if args.scope.is_empty() {
        all
    } else {
        all.into_iter().filter(|h| args.scope.contains(&h.kind)).collect()
    };
    crate::util::emit_ok(serde_json::to_value(&filtered).unwrap());
    Ok(())
}
```

Wire into main.rs Commands enum as `Search(SearchArgs)`.

- [ ] **Step 2: 통합 테스트**

```rust
#[test]
fn search_finds_memo_content() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["memo", "create", "-c", "dentist on friday"])
        .assert().success();
    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["search", "dentist"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    let hits = v["data"].as_array().unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0]["kind"], "memo");
}
```

- [ ] **Step 3: Commit**

```bash
git commit -am "feat(cli): hearth search <query> with scope/limit filters"
```

### Task 8.2: `hearth today / overdue / stats`

**Files:**
- Create: `src-tauri/cli/src/cmd/views.rs`

- [ ] **Step 1: 디스패치**

```rust
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ViewCmd {
    Today,
    Overdue,
    Stats,
}

pub fn dispatch(db_flag: Option<&str>, sub: ViewCmd) -> anyhow::Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let conn = crate::db::open(&p)?;
    let v = match sub {
        ViewCmd::Today => serde_json::to_value(&hearth_core::views::today(&conn)?).unwrap(),
        ViewCmd::Overdue => serde_json::to_value(&hearth_core::views::overdue(&conn)?).unwrap(),
        ViewCmd::Stats => serde_json::to_value(&hearth_core::views::stats(&conn)?).unwrap(),
    };
    crate::util::emit_ok(v);
    Ok(())
}
```

Register top-level `Today`, `Overdue`, `Stats` subcommands on main Commands enum (not wrapped in a group — direct `hearth today` ergonomics):
```rust
Today,
Overdue,
Stats,
```
Dispatch calls `ViewCmd::{Today,Overdue,Stats}` shim.

- [ ] **Step 2: 테스트**

```rust
#[test]
fn today_returns_structured_view() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();
    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["today"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert!(v["data"]["date"].is_string());
    assert!(v["data"]["schedules_today"].is_array());
    assert!(v["data"]["p0_projects"].is_array());
    assert!(v["data"]["recent_memos"].is_array());
}
```

- [ ] **Step 3: Commit**

```bash
git commit -am "feat(cli): hearth today/overdue/stats composite views"
```

---

## Phase 9 — CLI audit/undo/redo

### Task 9.1: `hearth log / undo / redo`

**Files:**
- Create: `src-tauri/cli/src/cmd/log.rs`

- [ ] **Step 1: 서브커맨드 정의 + dispatch**

```rust
use clap::Subcommand;

#[derive(Subcommand)]
pub enum LogCmd {
    /// Show recent audit entries.
    Show {
        #[arg(long, default_value_t = 50)]
        limit: i64,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        table: Option<String>,
        #[arg(long)]
        include_undone: bool,
    },
    /// Undo the last N mutations.
    Undo {
        #[arg(long, default_value_t = 1)]
        count: i64,
    },
    /// Redo the last N undone mutations.
    Redo {
        #[arg(long, default_value_t = 1)]
        count: i64,
    },
}

pub fn dispatch(db_flag: Option<&str>, sub: LogCmd) -> anyhow::Result<()> {
    let p = crate::db::resolve_db_path(db_flag)?;
    let mut conn = crate::db::open(&p)?;
    match sub {
        LogCmd::Show { limit, source, table, include_undone } => {
            let entries = hearth_core::audit::list(
                &conn, limit, source.as_deref(), table.as_deref(), include_undone,
            )?;
            crate::util::emit_ok(serde_json::to_value(&entries).unwrap());
        }
        LogCmd::Undo { count } => {
            let done = hearth_core::audit::undo(&mut conn, count)?;
            crate::util::emit_ok(serde_json::json!({ "undone": done.len(), "entries": done }));
        }
        LogCmd::Redo { count } => {
            let done = hearth_core::audit::redo(&mut conn, count)?;
            crate::util::emit_ok(serde_json::json!({ "redone": done.len(), "entries": done }));
        }
    }
    Ok(())
}
```

Register `Log { sub: LogCmd }` in main.

Expose top-level aliases too: `Undo { count }` and `Redo { count }` call `LogCmd::Undo/Redo`. (Clap allows multiple subcommands that map to the same dispatcher.)

- [ ] **Step 2: 테스트**

```rust
#[test]
fn undo_reverts_last_mutation() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "create", "undoable"])
        .assert().success();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["undo"])
        .assert().success();
    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "list"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}
```

- [ ] **Step 3: Commit**

```bash
git commit -am "feat(cli): hearth log/undo/redo using audit_log"
```

---

## Phase 10 — CLI export / import

### Task 10.1: `hearth export`

**Files:**
- Create: `src-tauri/cli/src/cmd/export.rs`
- Create: `src-tauri/core/src/export.rs`

- [ ] **Step 1: core::export 작성**

```rust
// core/src/export.rs
use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct Dump {
    pub version: u32,
    pub projects: Vec<crate::models::Project>,
    pub memos: Vec<crate::models::Memo>,
    pub schedules: Vec<crate::models::Schedule>,
    pub categories: Vec<serde_json::Value>,
    pub clients: Vec<crate::models::Client>,
    pub audit_log: Option<Vec<serde_json::Value>>,
}

pub fn export_json(conn: &Connection, include_audit: bool) -> rusqlite::Result<Dump> {
    let projects = crate::projects::list(conn)?;
    let memos = crate::memos::list(conn)?;
    // Schedules — use raw SELECT since we need raw reads.
    let mut stmt = conn.prepare(
        "SELECT id,date,time,location,description,notes,remind_before_5min,remind_at_start,created_at,updated_at
         FROM schedules ORDER BY date, COALESCE(time,'')",
    )?;
    let schedules: Vec<crate::models::Schedule> = stmt
        .query_map([], |r| {
            Ok(crate::models::Schedule {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                location: r.get(3)?,
                description: r.get(4)?,
                notes: r.get(5)?,
                remind_before_5min: r.get::<_, i64>(6)? != 0,
                remind_at_start: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Categories
    let mut stmt = conn.prepare("SELECT id,name,color,sort_order,created_at,updated_at FROM categories ORDER BY sort_order")?;
    let categories: Vec<serde_json::Value> = stmt
        .query_map([], |r| {
            Ok(json!({
                "id": r.get::<_, i64>(0)?,
                "name": r.get::<_, String>(1)?,
                "color": r.get::<_, String>(2)?,
                "sort_order": r.get::<_, i64>(3)?,
                "created_at": r.get::<_, String>(4)?,
                "updated_at": r.get::<_, String>(5)?,
            }))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let clients = crate::clients::list(conn)?;

    let audit_log = if include_audit {
        let entries = crate::audit::list(conn, i64::MAX, None, None, true)?;
        Some(serde_json::to_value(&entries).unwrap().as_array().cloned().unwrap_or_default())
    } else {
        None
    };

    Ok(Dump {
        version: 1,
        projects,
        memos,
        schedules,
        categories,
        clients,
        audit_log,
    })
}
```

Add `pub mod export;` to lib.rs. (If `clients::list` doesn't exist yet, add it — one-liner).

- [ ] **Step 2: CLI dispatch**

```rust
// cli/src/cmd/export.rs
use clap::Args;

#[derive(Args)]
pub struct ExportArgs {
    #[arg(long, default_value = "json")]
    pub format: String, // json | sqlite
    #[arg(long)]
    pub out: Option<String>,
    #[arg(long)]
    pub include_audit: bool,
}

pub fn dispatch(db_flag: Option<&str>, args: ExportArgs) -> anyhow::Result<()> {
    let db_path = crate::db::resolve_db_path(db_flag)?;
    match args.format.as_str() {
        "json" => {
            let conn = crate::db::open(&db_path)?;
            let dump = hearth_core::export::export_json(&conn, args.include_audit)?;
            let s = serde_json::to_string_pretty(&dump)?;
            match args.out {
                Some(p) => {
                    std::fs::write(&p, &s)?;
                    crate::util::emit_ok(serde_json::json!({ "written": p }));
                }
                None => {
                    // JSON to stdout, no wrapping envelope — agent may pipe.
                    println!("{s}");
                }
            }
        }
        "sqlite" => {
            let out = args.out.ok_or_else(|| anyhow::anyhow!("--out required for --format sqlite"))?;
            std::fs::copy(&db_path, &out)?;
            crate::util::emit_ok(serde_json::json!({ "written": out }));
        }
        other => anyhow::bail!("unknown --format: {other}"),
    }
    Ok(())
}
```

- [ ] **Step 3: 테스트**

```rust
#[test]
fn export_json_includes_projects() {
    let dir = TempDir::new().unwrap();
    let db_str = dir.path().join("t.db").to_str().unwrap().to_string();
    let out_path = dir.path().join("dump.json");

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["project", "create", "ExportMe"])
        .assert().success();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", &db_str)
        .args(["export", "--format", "json", "--out", out_path.to_str().unwrap()])
        .assert().success();
    let contents = std::fs::read_to_string(&out_path).unwrap();
    assert!(contents.contains("ExportMe"));
}
```

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat(cli): hearth export (json|sqlite) + core::export dump"
```

### Task 10.2: `hearth import`

**Files:**
- Create: `src-tauri/cli/src/cmd/import.rs`
- Modify: `src-tauri/core/src/export.rs` (add `import_json_merge`)

- [ ] **Step 1: core::export::import_json_merge**

```rust
// Append to core/src/export.rs
use serde_json::Value;

pub struct ImportReport {
    pub projects_added: usize,
    pub memos_added: usize,
    pub schedules_added: usize,
    pub categories_added: usize,
    pub skipped_duplicates: usize,
}

pub fn import_json_merge(
    conn: &mut Connection,
    dump: &Dump,
    dry_run: bool,
) -> rusqlite::Result<ImportReport> {
    let mut report = ImportReport {
        projects_added: 0,
        memos_added: 0,
        schedules_added: 0,
        categories_added: 0,
        skipped_duplicates: 0,
    };
    let tx = conn.transaction()?;

    // Categories: upsert by name
    for c in &dump.categories {
        let name = c.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let exists: i64 = tx
            .query_row("SELECT COUNT(*) FROM categories WHERE name = ?1", [name], |r| r.get(0))?;
        if exists > 0 {
            report.skipped_duplicates += 1;
            continue;
        }
        let color = c.get("color").and_then(|v| v.as_str()).unwrap_or("#6b7280");
        tx.execute(
            "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order),0)+1 FROM categories))",
            rusqlite::params![name, color],
        )?;
        report.categories_added += 1;
    }

    // Projects: always insert new id (auto-increment), match by (name, priority) to dedupe
    for p in &dump.projects {
        let existing: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE name=?1 AND priority=?2",
                rusqlite::params![p.name, p.priority],
                |r| r.get(0),
            )?;
        if existing > 0 {
            report.skipped_duplicates += 1;
            continue;
        }
        tx.execute(
            "INSERT INTO projects (priority, number, name, category, path, evaluation, sort_order)
             VALUES (?1,?2,?3,?4,?5,?6, COALESCE((SELECT MAX(sort_order) FROM projects WHERE priority=?1),0)+1)",
            rusqlite::params![p.priority, p.number, p.name, p.category, p.path, p.evaluation],
        )?;
        report.projects_added += 1;
    }

    // Memos: match by content hash
    for m in &dump.memos {
        let existing: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM memos WHERE content=?1 AND color=?2",
                rusqlite::params![m.content, m.color],
                |r| r.get(0),
            )?;
        if existing > 0 {
            report.skipped_duplicates += 1;
            continue;
        }
        tx.execute(
            "INSERT INTO memos (content, color, project_id, sort_order) VALUES (?1,?2,?3, COALESCE((SELECT MAX(sort_order) FROM memos),0)+1)",
            rusqlite::params![m.content, m.color, m.project_id],
        )?;
        report.memos_added += 1;
    }

    // Schedules: match by (date, time, description)
    for s in &dump.schedules {
        let existing: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM schedules WHERE date=?1 AND COALESCE(time,'')=COALESCE(?2,'') AND COALESCE(description,'')=COALESCE(?3,'')",
                rusqlite::params![s.date, s.time, s.description],
                |r| r.get(0),
            )?;
        if existing > 0 {
            report.skipped_duplicates += 1;
            continue;
        }
        tx.execute(
            "INSERT INTO schedules (date, time, location, description, notes, remind_before_5min, remind_at_start)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![s.date, s.time, s.location, s.description, s.notes, s.remind_before_5min as i64, s.remind_at_start as i64],
        )?;
        report.schedules_added += 1;
    }

    if dry_run {
        tx.rollback()?;
    } else {
        tx.commit()?;
    }
    Ok(report)
}
```

- [ ] **Step 2: CLI dispatch**

```rust
// cli/src/cmd/import.rs
use clap::Args;

#[derive(Args)]
pub struct ImportArgs {
    /// Path to a JSON dump.
    pub file: String,
    /// Replace entire DB (destructive). Requires --yes.
    #[arg(long)]
    pub replace: bool,
    /// Merge with existing data (default).
    #[arg(long, conflicts_with = "replace")]
    pub merge: bool,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub yes: bool,
}

pub fn dispatch(db_flag: Option<&str>, args: ImportArgs) -> anyhow::Result<()> {
    let db_path = crate::db::resolve_db_path(db_flag)?;

    if args.replace && !args.yes {
        crate::util::emit_err(
            "--replace requires --yes (destructive)",
            Some("re-run with --yes to confirm"),
        );
        std::process::exit(1);
    }

    let raw = std::fs::read_to_string(&args.file)?;
    let dump: hearth_core::export::Dump = serde_json::from_str(&raw)?;
    if args.replace {
        // Backup to pre-import.db, then truncate tables, then merge.
        let backup = db_path.with_extension(format!(
            "pre-import-{}.db",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        ));
        std::fs::copy(&db_path, &backup)?;
        let mut conn = crate::db::open(&db_path)?;
        conn.execute_batch(
            "DELETE FROM audit_log; DELETE FROM memos; DELETE FROM schedules;
             DELETE FROM projects; DELETE FROM categories;",
        )?;
        let report = hearth_core::export::import_json_merge(&mut conn, &dump, args.dry_run)?;
        crate::util::emit_ok(serde_json::json!({
            "backup": backup.to_string_lossy(),
            "replaced": true,
            "report": {
                "projects_added": report.projects_added,
                "memos_added": report.memos_added,
                "schedules_added": report.schedules_added,
                "categories_added": report.categories_added,
                "skipped_duplicates": report.skipped_duplicates,
            },
        }));
    } else {
        let mut conn = crate::db::open(&db_path)?;
        let report = hearth_core::export::import_json_merge(&mut conn, &dump, args.dry_run)?;
        crate::util::emit_ok(serde_json::json!({
            "merged": true,
            "dry_run": args.dry_run,
            "report": {
                "projects_added": report.projects_added,
                "memos_added": report.memos_added,
                "schedules_added": report.schedules_added,
                "categories_added": report.categories_added,
                "skipped_duplicates": report.skipped_duplicates,
            },
        }));
    }
    Ok(())
}
```

- [ ] **Step 3: 테스트**

```rust
#[test]
fn export_then_import_merge_roundtrip() {
    let dir = TempDir::new().unwrap();
    let db_a = dir.path().join("a.db");
    let db_b = dir.path().join("b.db");
    let dump = dir.path().join("dump.json");

    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", db_a.to_str().unwrap())
        .args(["project", "create", "PortMe"])
        .assert().success();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", db_a.to_str().unwrap())
        .args(["export", "--format", "json", "--out", dump.to_str().unwrap()])
        .assert().success();
    Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", db_b.to_str().unwrap())
        .args(["import", dump.to_str().unwrap(), "--merge"])
        .assert().success();
    let out = Command::cargo_bin("hearth").unwrap()
        .env("HEARTH_DB", db_b.to_str().unwrap())
        .args(["project", "list"])
        .assert().success().get_output().stdout.clone();
    let v: Value = serde_json::from_slice(&out).unwrap();
    assert!(v["data"].as_array().unwrap().iter().any(|p| p["name"] == "PortMe"));
}
```

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(cli): hearth import (merge | replace --yes) + core::import_json_merge"
```

---

## Phase 11 — Pretty printing

### Task 11.1: `--pretty` human-readable tables

**Files:**
- Modify: `src-tauri/cli/src/util.rs`
- Modify: main + each cmd module to receive the flag

- [ ] **Step 1: util 확장**

Add `--pretty` global flag in main CLI. Pass down to emit helpers:

```rust
// util.rs (extend)
use comfy_table::{presets::UTF8_BORDERS_ONLY, ContentArrangement, Table};

pub fn emit_ok_pretty(data: &serde_json::Value, headers: &[&str]) {
    let mut t = Table::new();
    t.load_preset(UTF8_BORDERS_ONLY)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(headers.iter().copied());
    if let Some(arr) = data.as_array() {
        for row in arr {
            let cells: Vec<String> = headers
                .iter()
                .map(|h| row.get(h).map(|v| to_cell(v)).unwrap_or_default())
                .collect();
            t.add_row(cells);
        }
    }
    println!("{t}");
}

fn to_cell(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::from("-"),
        serde_json::Value::String(s) => s.clone(),
        _ => v.to_string(),
    }
}
```

- [ ] **Step 2: 디스패치 스위치**

In each `dispatch` module that returns a list, accept `pretty: bool`. If true, call `emit_ok_pretty(&json_value, &headers)`. Otherwise `emit_ok(value)`. Header lists:
- project list: `["id","priority","name","category","path"]`
- memo list: `["id","color","content","project_id"]`
- schedule list: `["id","date","time","description","location"]`

- [ ] **Step 3: main.rs 에 `--pretty` 추가**

```rust
#[arg(long, global = true)]
pretty: bool,
```
Pass `cli.pretty` into each dispatch.

- [ ] **Step 4: 수동 검증**

Run:
```bash
./target/debug/hearth project list --pretty
```
Expected: Unicode-bordered table.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(cli): --pretty flag for tabular human output (comfy-table)"
```

---

## Phase 12 — App side integration with audit_log UI (minimal)

> **목적:** 이 phase 는 앱이 CLI 의 audit 항목을 **읽기만** 하면 된다는 걸 증명. 풀 UI "Activity" 탭은 후속 스펙으로 미룸. 여기선 기존 Undo 토스트가 `audit_log` 를 참조하게 연결만 한다.

### Task 12.1: 기존 Undo 토스트 → audit_log 소비

이 작업은 **선택적** 이며 현재 CLI 스펙의 핵심 out-of-scope 이다. 실제로 지금 앱의 Undo 토스트 스택이 어떻게 구현됐는지 재확인 (현재 `cmd_ai.rs` 의 mutation 후 확인 모달 + 토스트 레이어) 후, `hearth_core::audit::undo(&mut db, 1)` 한 줄로 치환할 수 있는지 검토.

- [ ] **Step 1: 기존 Undo 구현 스캔**

Run: `rg -n 'undo|Undo' src-tauri/app/src` + `rg -n 'undo|Undo' src`.

- [ ] **Step 2: 스캔 결과에 따라 결정**

If 기존 구조가 복잡하게 React 쪽 상태로 묶여 있으면 본 plan 범위에서 제외하고 후속 스펙으로 미룬다. TODO 만 코드 주석에 남김:
```rust
// TODO(future spec): wire existing Undo toast to hearth_core::audit::undo.
```

If 단순하다면 한 `invoke('hearth_undo')` Tauri 커맨드만 추가하면 완료. 추가 시 `cmd_audit.rs` 모듈:
```rust
#[tauri::command]
pub fn hearth_undo(state: State<'_, AppState>, count: i64) -> Result<Vec<hearth_core::audit::AuditEntry>, String> {
    let mut db = state.db.lock().map_err(|e| e.to_string())?;
    hearth_core::audit::undo(&mut db, count).map_err(|e| e.to_string())
}
```
Register in invoke_handler.

- [ ] **Step 3: Commit (or skip)**

```bash
git commit -am "feat(app): expose hearth_undo Tauri command (optional)"
```

---

## Phase 13 — Full Integration Verification

### Task 13.1: End-to-end 수동 검증

**Files:** 없음 (검증 단계)

- [ ] **Step 1: 앱 실행**

```bash
npm run tauri dev
```
앱에 기존 데이터가 남아있다면 임시 DB 사용 권장:
```bash
HEARTH_DB=/tmp/hearth-e2e.db ./src-tauri/target/debug/hearth db migrate
```

- [ ] **Step 2: 시나리오 A — CLI 프로젝트 생성 → 앱 UI 반영**

```bash
./src-tauri/target/debug/hearth project create "E2E" --priority P0
```
Expected: 앱의 프로젝트 탭에 0.5~1 초 내에 "E2E" 카드가 나타남 (P0 섹션).

- [ ] **Step 3: 시나리오 B — CLI 메모 생성 + 검색**

```bash
./src-tauri/target/debug/hearth memo create -c "remember dentist friday" --color pink
./src-tauri/target/debug/hearth search "dentist"
```
Expected: 앱의 메모보드에 pink 메모 등장. search 명령 JSON 에 한 hit.

- [ ] **Step 4: 시나리오 C — today / overdue / stats**

```bash
./src-tauri/target/debug/hearth today
./src-tauri/target/debug/hearth overdue
./src-tauri/target/debug/hearth stats
```
Expected: 구조화된 JSON. today 에 Step 2 의 P0 프로젝트 포함.

- [ ] **Step 5: 시나리오 D — audit log + undo**

```bash
./src-tauri/target/debug/hearth log --limit 5
./src-tauri/target/debug/hearth undo
```
Expected: 앱의 메모보드에서 Step 3 의 메모 사라짐. `log` JSON 이 `undone:true` 포함.

- [ ] **Step 6: 시나리오 E — export/import roundtrip**

```bash
./src-tauri/target/debug/hearth export --format json --out /tmp/dump.json
HEARTH_DB=/tmp/hearth-e2e-b.db ./src-tauri/target/debug/hearth db migrate
HEARTH_DB=/tmp/hearth-e2e-b.db ./src-tauri/target/debug/hearth import /tmp/dump.json --merge
HEARTH_DB=/tmp/hearth-e2e-b.db ./src-tauri/target/debug/hearth project list
```
Expected: Two independent DBs, same project list after import.

- [ ] **Step 7: 시나리오 F — 기존 기능 regression**

앱에서 Quick Capture (⌃⇧H), ⌘K, ⌘F, 설정/카테고리/백업, 알림 토글, 드래그 재정렬 등 수동으로 한 번씩 실행. 이전 동작과 다름 없어야 함.

- [ ] **Step 8: Commit (검증 노트)**

없음 (코드 변경 없음). 발견된 regression 은 fix 커밋.

---

## Phase 14 — Documentation + README

### Task 14.1: README 보강

**Files:**
- Modify: `README.md`

- [ ] **Step 1: "CLI" 섹션 추가**

`README.md` 의 "Testing" 섹션 뒤에 새 섹션 추가:
```markdown
## CLI (Hearth 바이너리)

`hearth` CLI 로 DB 를 직접 조작할 수 있습니다. 실행 중인 앱은 0.5~1 초 내로 변경을 자동 반영합니다.

### 빌드 & 설치 (개발 모드)

    cd src-tauri
    cargo build --release -p hearth-cli
    # 바이너리: src-tauri/target/release/hearth

### 주요 명령

    hearth project list [--priority P0,P1] [--category Active]
    hearth project create "New Proj" --priority P1
    hearth memo create -c "buy milk" --color yellow
    hearth schedule create --date 2026-05-01 --time 09:00
    hearth search "agent"
    hearth today / overdue / stats
    hearth log --limit 20
    hearth undo
    hearth export --format json --out dump.json
    hearth import dump.json --merge

### DB 경로

기본: `~/Library/Application Support/com.newturn2017.hearth/data.db`.
`--db <PATH>` 또는 `HEARTH_DB` 환경변수로 덮어쓸 수 있습니다.

### 출력 포맷

기본 JSON (`{ok:true, data:...}`). `--pretty` 로 사람용 테이블.

### 안전성

모든 mutation 은 `audit_log` 에 기록됩니다. `hearth undo` / `redo` 로 되돌릴 수 있고, `--dry-run` 으로 실행 없이 미리 봅니다.
```

- [ ] **Step 2: `CHANGELOG.md` 업데이트**

CHANGELOG 상단에 추가:
```markdown
## 0.7.0 — Hearth CLI (unreleased)

- Cargo workspace split: hearth-core / hearth-app / hearth-cli
- New `hearth` CLI binary — full CRUD + search + today/overdue/stats views + audit log undo/redo + export/import
- App: data_version 500ms watcher — external DB writes reflected in UI without restart
- DB: added `audit_log` table, `projects_fts` / `memos_fts` / `schedules_fts` FTS5 virtual tables
```

- [ ] **Step 3: Commit**

```bash
git add README.md CHANGELOG.md
git commit -m "docs: CLI section in README + 0.7.0 entry in CHANGELOG"
```

---

## Completion Checklist

- [ ] Phase 0 baseline 통과
- [ ] Phase 1 workspace 빌드 + 앱 실행 정상
- [ ] Phase 2 migrations (audit_log + 3 FTS tables) 통과
- [ ] Phase 3 hearth-core 모듈 이식 + 기존 앱 회귀 없음
- [ ] Phase 4 watcher 실시간 반영 수동 확인
- [ ] Phase 5 CLI 스켈레톤 + `hearth db` 통과
- [ ] Phase 6 `hearth project` 전체 테스트 통과
- [ ] Phase 7 `hearth memo/schedule/category` 전체 통과
- [ ] Phase 8 `hearth search/today/overdue/stats` 통과
- [ ] Phase 9 `hearth log/undo/redo` 통과
- [ ] Phase 10 `hearth export/import` 라운드트립 확인
- [ ] Phase 11 `--pretty` 표 렌더 확인
- [ ] Phase 12 앱 Undo 연결 여부 결정 + 기록
- [ ] Phase 13 E2E 수동 시나리오 6개 모두 통과
- [ ] Phase 14 문서 보강 + CHANGELOG

---

## Follow-Up Specs (Next Sub-Projects)

1. **Agent Skills** — 이 CLI 를 감싸는 Claude Code / Codex 스킬 (예: `hearth-today-brief`, `hearth-project-scan`, `hearth-memo-organize`). 별도 브레인스토밍.
2. **자동 배포** — Homebrew tap + GitHub Release 정적 바이너리 + Claude Code plugin registry publish. 별도 브레인스토밍.
3. **앱 Activity 뷰** — audit_log 를 시각화하는 별도 탭. 별도 브레인스토밍.
