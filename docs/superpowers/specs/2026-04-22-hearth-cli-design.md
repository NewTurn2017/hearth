# Hearth CLI — Design Spec

**Date:** 2026-04-22
**Status:** Approved, pre-implementation
**Branch:** `claude/pedantic-merkle-b0a5f1` (worktree-isolated)
**Sub-project of:** Agent-driven hearth control (parent goal — CLI + Skills + Auto-deploy)

## Context

Hearth 는 Tauri 2 · Rust 기반 로컬 퍼스트 데스크톱 앱으로, 프로젝트 · 메모 · 일정 · 카테고리를 SQLite 한 파일에 저장한다. 현재 모든 조작은 Tauri IPC (React UI) 또는 `ai_tools.rs` 의 18-tool OpenAI tool-calling 루프를 통해서만 가능하다.

외부 agent (Claude Code, Codex, 스크립트) 는 이 데이터를 조작할 진입점이 없다. 본 문서는 **독립적인 `hearth` CLI 바이너리**를 정의한다. 이 CLI 는:

1. 실행 중인 Hearth 앱과 무관하게 SQLite 를 직접 조작한다 (stateless).
2. Tauri 앱과 **같은 로직 크레이트** (`hearth-core`) 를 공유한다 — 드리프트 제로.
3. Agent 친화적 JSON 기본 출력 + audit log 기반 undo 를 제공한다.
4. 변경 발생 시 실행 중인 앱이 **재시작 없이 실시간 반영** 하도록 `PRAGMA data_version` 폴링을 쓴다.

CLI 는 본 스펙의 단일 산출물이다. 후속 스펙으로 (a) 이 CLI 를 감싸는 agent **Skills** 세트, (b) Homebrew/Release 기반 **자동 배포** 파이프라인이 예정돼 있으나 범위 밖이다.

## Goals

- Hearth 의 모든 핵심 데이터 (projects · memos · schedules · categories) 를 CLI 로 완전하게 CRUD + 검색 가능.
- Agent 가 실행 중인 앱의 UI 재시작 없이 변경사항을 실시간(0.5 초 이내) 으로 보게 한다.
- 모든 mutation 을 audit log 에 기록하고, 앱·CLI 어느 쪽이든 `undo` 로 되돌릴 수 있게 한다.
- Rust workspace 재편성으로 `hearth-core` · `hearth-app` · `hearth-cli` 세 크레이트가 같은 SQL 로직을 공유한다.
- 기본 출력은 JSON, `--pretty` 로 human-readable 테이블 렌더.
- CLI 는 런타임 의존성 없는 단일 정적 바이너리.

## Non-Goals

- Agent Skills 작성 (후속 스펙).
- CLI/스킬/앱 배포 자동화 (후속 스펙).
- CLI 가 OpenAI 를 프록시하는 `hearth ai "..."` 명령 (YAGNI — agent 측 LLM 이 이미 존재).
- 앱 UI 의 audit_log 뷰 ("Activity" 탭) — 후속 스펙.
- Windows/Linux native bundle — macOS 에 맞춰 시작, 정적 Rust 바이너리라 크로스 빌드는 파생 작업.
- 앱 OS 통합 설정 CLI 제어 (autostart 토글 · quick-capture global shortcut · 알림 권한) — 이것들은 Tauri plugin 기반 OS-side 상태라 CLI 범위에서 제외.
- `clients` 테이블 CLI 커맨드 — 현재 앱 UI 에서도 사용 비중이 낮음. Export/Import 에는 포함되나 전용 `hearth client ...` 서브커맨드는 빼고, 필요해질 때 추가.

## Architecture

### Workspace Re-organization

`src-tauri/` 를 Cargo workspace 루트로 재편성한다.

```
src-tauri/
├── Cargo.toml                  # [workspace] members = ["core", "app", "cli"]
├── core/                       # 신규 크레이트 — 순수 Rust (no Tauri deps)
│   ├── Cargo.toml              # rusqlite + chrono + serde + thiserror
│   └── src/
│       ├── lib.rs
│       ├── db.rs               # init + migrations + data_version helpers
│       ├── projects.rs         # 프로젝트 CRUD (기존 cmd_projects SQL 이식)
│       ├── memos.rs
│       ├── schedules.rs
│       ├── categories.rs
│       ├── clients.rs          # CRUD 함수는 이식하되 CLI 커맨드는 없음 (Non-Goals 참조)
│       ├── backup.rs           # 기존 앱 백업 로직 이동. CLI 의 export/import 도 여기를 재사용
│       ├── audit.rs            # 신규 — audit_log write + undo 엔진
│       ├── search.rs           # 신규 — FTS5 bootstrap + 쿼리
│       ├── views.rs            # 신규 — today/overdue/stats 합성 뷰
│       └── scan.rs             # 신규 — 폴더 스캔 → 프로젝트 후보
├── app/                        # 기존 src-tauri/src 이전
│   ├── Cargo.toml              # 기존 deps + hearth-core
│   └── src/
│       ├── lib.rs
│       ├── main.rs
│       ├── watcher.rs          # 신규 — data_version 폴링
│       └── cmd_*.rs            # hearth-core 를 얇게 래핑 (Tauri IPC 시그니처 유지)
└── cli/                        # 신규 바이너리 크레이트
    ├── Cargo.toml              # clap + hearth-core + serde_json + comfy-table
    └── src/
        ├── main.rs             # clap 진입점 + 에러 포맷
        └── cmd/
            ├── project.rs
            ├── memo.rs
            ├── schedule.rs
            ├── category.rs
            ├── search.rs
            ├── views.rs        # today/overdue/stats
            ├── log.rs          # log/undo/redo
            ├── export.rs
            ├── import.rs
            └── db.rs           # path/vacuum/integrity
```

**공유 원칙:** Tauri 커맨드 (`app/src/cmd_projects.rs` 등) 와 CLI 서브커맨드 (`cli/src/cmd/project.rs`) 는 모두 `hearth-core::projects::*` 함수를 호출한다. SQL 문자열은 `hearth-core` 내부에서만 존재.

### Runtime / Concurrency

- SQLite 는 기존대로 WAL 모드. Tauri 앱과 CLI 가 동시에 접근해도 WAL 이 다중 reader + 1 writer 를 처리.
- CLI 는 자체 `Connection` 을 열고 작업 후 drop — stateless.
- 앱은 기존 `AppState { db: Mutex<Connection> }` 유지.

## Data-Layer Changes

### 신규 테이블 — `audit_log`

```sql
CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT    NOT NULL DEFAULT (datetime('now')),
    source      TEXT    NOT NULL,          -- 'app' | 'cli' | 'ai'
    op          TEXT    NOT NULL,          -- 'create' | 'update' | 'delete'
    table_name  TEXT    NOT NULL,
    row_id      INTEGER,
    before_json TEXT,                      -- NULL for create
    after_json  TEXT,                      -- NULL for delete
    undone      INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_audit_ts      ON audit_log(ts DESC);
CREATE INDEX idx_audit_undone  ON audit_log(undone, ts DESC);
```

- 한 트랜잭션 안에서 `audit_log` INSERT + 실제 데이터 변경 → 반-원자적.
- `source` 는 `hearth-core::mutate(source: Source, ...)` 의 enum 파라미터로 전달. CLI 는 `"cli"`, Tauri 커맨드는 `"app"`, `cmd_ai.rs` 는 `"ai"`.
- `before_json`/`after_json` 은 `serde_json::to_string(&row)` 형태. 스키마가 바뀌어도 과거 undo 가 최대한 복원 가능.

### FTS5 가상 테이블 — `projects_fts` · `memos_fts` · `schedules_fts`

```sql
CREATE VIRTUAL TABLE projects_fts USING fts5(
    name, category, evaluation,
    content=projects, content_rowid=id
);
CREATE TRIGGER projects_ai AFTER INSERT ON projects
  BEGIN INSERT INTO projects_fts(rowid, name, category, evaluation)
    VALUES (new.id, new.name, new.category, new.evaluation); END;
-- AD / AU triggers 유사 (공식 FTS5 contentless-table 패턴)
```

- `memos_fts(content)` / `schedules_fts(description, location, notes)` 도 같은 패턴.
- 마이그레이션에서 기존 DB 가 FTS 테이블 없으면 1회 rebuild: `INSERT INTO projects_fts(rowid,...) SELECT id,... FROM projects;`

### 스키마 마이그레이션 전략

- 기존 `run_migrations` 함수에 `ensure_audit_log_table` · `ensure_fts_tables` 추가 — 모두 idempotent `CREATE TABLE IF NOT EXISTS`.
- 기존 `ensure_schedule_reminder_columns` 패턴 그대로 따름.
- `data_version` 은 pragma 라 스키마 변경 불필요.

## Real-Time Sync — App Watcher

새 모듈 `app/src/watcher.rs`:

```rust
pub fn spawn(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: i64 = 0;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let state = app.state::<AppState>();
            let db = match state.db.lock() { Ok(g) => g, Err(_) => continue };
            let v: i64 = match db.query_row(
                "PRAGMA data_version", [], |r| r.get(0)
            ) { Ok(v) => v, Err(_) => continue };
            drop(db);
            if last == 0 { last = v; continue; }
            if v != last {
                let _ = app.emit("projects:changed", ());
                let _ = app.emit("memos:changed", ());
                let _ = app.emit("schedules:changed", ());
                let _ = app.emit("categories:changed", ());
                last = v;
            }
        }
    });
}
```

- `lib.rs::run().setup()` 에서 호출.
- 0.5 초 지연은 사용자 체감상 즉시. CPU 부담 무시할 수준 (쿼리 자체가 in-memory counter).
- 프론트엔드는 이미 4종 이벤트 리스너가 있어 **UI 변경 0**. `useProjects` · `useMemos` · `useSchedules` · `useCategories` 가 자동 refetch.
- 세밀한 테이블 구분이 필요해지면 `audit_log` 의 최신 `table_name` 을 같이 조회해 해당 이벤트만 쏘는 걸로 진화 가능 (범위 밖).

## CLI Command Surface

모든 mutation 은 기본값 auto-execute + audit_log 기록. `--dry-run` 으로 SQL 실행 없이 의도한 JSON 결과만 출력. `--yes` 플래그는 파괴적 명령 (`reset`, `import --replace`) 에서만 의미.

### Projects

```
hearth project list [--priority P0,P1] [--category Active] [--limit N]
hearth project get <id>
hearth project create <name> [--priority P2] [--category Side] [--path PATH] [--evaluation "..."]
hearth project update <id> [--name ...] [--priority ...] [--category ...] [--path ...] [--evaluation ...]
hearth project delete <id>
hearth project scan <dir> [--depth 1]     # 하위 디렉토리 → 미등록 프로젝트 후보 JSON
hearth project link-path <id> <path>      # path 유효성 검증 후 설정
```

### Memos

```
hearth memo list [--project <id>] [--color yellow] [--limit N]
hearth memo get <id>
hearth memo create -c "..." [--project <id>] [--color yellow]
hearth memo update <id> [--content ...] [--color ...] [--project <id>|--detach]
hearth memo delete <id>
```

### Schedules

```
hearth schedule list [--month 2026-04] [--from 2026-04-01] [--to 2026-04-30]
hearth schedule get <id>
hearth schedule create --date 2026-04-22 [--time 15:00] [--location "..."] [--description "..."] [--notes "..."] [--remind-5min] [--remind-start]
hearth schedule update <id> [--date ...] [--time ...] [--location ...] [--description ...] [--notes ...]
hearth schedule delete <id>
```

### Categories

```
hearth category list
hearth category create <name> [--color "#22c55e"]
hearth category rename <old-name> <new-name>   # 앱 기존 cascade 로직 재사용
hearth category update <id> [--color ...] [--sort-order N]
hearth category delete <id>                    # 사용 중이면 에러 (기존 규칙)
```

### Search

```
hearth search <query> [--scope projects,memos,schedules] [--limit 20]
```
- FTS5 기반. scope 생략 시 전 테이블 병합 (결과에 `type` 필드 동봉).
- `type="memo"` · `"project"` · `"schedule"`, `score` (bm25), `preview` (하이라이트 스니펫).

### Composite Views

```
hearth today
```
출력:
```json
{
  "date": "2026-04-22",
  "schedules_today": [ ... ],
  "p0_projects": [ ... ],
  "recent_memos": [ ... ]        // last 24h
}
```

```
hearth overdue
```
- `schedules.date < today` 이면서 `notes` 에 "완료" 마커 없는 일정 (범위 밖: 완료 마커 필드 — 현재는 단순 날짜 기반)
- 30일+ 업데이트 없는 프로젝트 (미완성 평가 기준)
- 출력은 세 섹션 배열 동봉 JSON.

```
hearth stats
```
- 프로젝트 수 / 우선순위 분포 / 카테고리 분포
- 메모 수 / 색상 분포 / 프로젝트 링크 여부
- 일정 수 / 이번 달 · 다음 달

### Audit / Undo

```
hearth log [--limit 50] [--source cli|app|ai] [--table projects] [--include-undone]
hearth undo [--count 1]
hearth redo [--count 1]          # undo 된 최근 N 개 재적용
```

- `undo`: `undone=0` 중 가장 최신 → 역실행. op 별 분기:
  - `create` 역 → `DELETE FROM <table> WHERE id=?`
  - `delete` 역 → `INSERT INTO <table> ...` from `before_json`
  - `update` 역 → `UPDATE <table> SET ... = before_json WHERE id=?`
  - 역실행도 하나의 tx 안에서 audit_log 에 `source='undo'` 형태로 기록 (혹은 기존 레코드의 `undone=1` 만 토글 — 구현 상세는 계획 단계에서 확정).
- 앱 쪽 기존 Undo 토스트 → 같은 audit_log 를 조회하도록 리팩터 (별도 후속 스펙으로 분리 가능, 현재 스펙에서는 CLI 가 audit_log 를 쓰기 시작하는 것까지만).

### Export / Import

```
hearth export [--format json|sqlite] [--out FILE]
hearth import <file> [--merge|--replace] [--dry-run]
```
- `json`: 전 테이블 덤프 (audit_log 제외 기본, `--include-audit` 로 포함).
- `sqlite`: `.db` 파일 복사 (기존 백업 로직 재사용).
- `import --merge`: id 충돌 시 신규 id 할당 + 이름 매핑.
- `import --replace --yes`: 현재 DB drop 후 교체. 실행 전 자동 pre-reset 백업.

### DB Utilities

```
hearth db path                # 현재 DB 경로 출력
hearth db vacuum              # VACUUM + PRAGMA integrity_check
hearth db migrate             # (수동) 마이그레이션 재실행 (idempotent)
```

### Global Flags

```
--db <PATH>     # DB 경로 override. HEARTH_DB 환경변수로도 지정 가능
--json          # 기본값 (명시 생략 가능)
--pretty        # comfy-table 렌더 + colorized
--dry-run       # mutation 은 실행하지 않고 의도 JSON 만
--yes           # 파괴적 명령 (reset/import replace) 확정
-v, --verbose   # RUST_LOG=debug 와 동일
```

## Output / Error Contract

### Success

```json
{"ok": true, "data": {...}}
```

- `data` 는 명령별 형태. 리스트는 배열, 단건은 객체.
- `--pretty` 시 `data` 를 `comfy-table` 으로 렌더, metadata 는 헤더에.

### Error

```json
{"ok": false, "error": "project 42 not found", "hint": "try 'hearth project list'"}
```

- exit code: `0` success · `1` user error (not found, validation) · `2` DB error · `64` usage (clap parse fail)
- `--pretty` 시 stderr 로 색칠된 에러 + hint.

### Logging

- `tracing` crate. `RUST_LOG=debug` 또는 `-v` 로 디버그 로그 stderr.
- stdout 은 **항상** JSON 또는 pretty 테이블만 — 로그 간섭 없음.

## Testing Strategy

### `hearth-core` 단위 테스트

- `Connection::open_in_memory()` 기반.
- 기존 `cmd_*.rs` 의 테스트 (21 개) 를 `core` 크레이트로 이전.
- 신규 테스트:
  - audit_log round-trip (create → undo → state 복원)
  - audit_log op 3종 + undo 각각
  - FTS5 bootstrap + 쿼리 랭킹
  - today/overdue/stats 뷰 가공 로직

### `hearth-cli` 통합 테스트

- `assert_cmd` + `predicates` 크레이트.
- 임시 디렉토리에 빈 DB 생성 → `cargo run --bin hearth -- memo create -c "test" --db /tmp/.../data.db` → stdout JSON 파싱 → assert.
- 각 서브커맨드마다 happy-path + error-path 최소 1 개.

### 앱 watcher 테스트

- tokio 테스트. 임시 DB 열고 watcher spawn → 다른 connection 으로 INSERT → `app.emit` 스파이에 이벤트 도착 확인.

### 비-테스트 검증 (수동)

- Tauri 앱 실행 중 → 다른 터미널에서 `hearth memo create -c "from cli"` → UI 가 0.5~1 초 내 refetch 하는지 눈으로 확인.

## Rollout / Migration Notes

- workspace 재편성 PR 은 이 스펙의 일부. 다음 plan 단계 첫 번째 단계로 수행.
- workspace 재편성 후 기존 `cargo test` 경로가 `src-tauri/app/` 로 바뀌나, CI 가 없으므로 README · `.claude` 설정 · `scripts/release.sh` 에서 경로를 점검.
- `scripts/release.sh` 는 현재 `src-tauri/target/` 경로 사용 — workspace 아래 `src-tauri/target/` 로 통합되므로 대체로 무변경. 확인 필요.
- 첫 실행 시 기존 유저 DB 에 `audit_log` + `*_fts` 테이블이 추가된다. 기존 데이터는 FTS rebuild 1회.

## Risks / Mitigations

| 리스크 | 완화 |
|---|---|
| workspace 재편성 중 Tauri plugin wiring 이 깨짐 | `hearth-app` 가 `hearth-core` 함수를 호출하는 시점에 Tauri 커맨드 시그니처는 바꾸지 않는다. 프론트엔드 `invoke` 호출은 0 변경 |
| `data_version` 폴링 지연으로 agent 가 빠르게 쓴 뒤 UI 가 0.5~1 초 옛 상태 | 사용자 체감 "즉시" 범위. 필요 시 250ms 로 조정 |
| audit_log 무한 증가 | 별도 후속 작업으로 `prune` 명령 (30일+ `undone=1` 삭제). 단기엔 무시 |
| FTS5 인덱스와 본테이블 동기화 실패 (트리거 누락) | 마이그레이션 부팅 시 `SELECT COUNT(*) FROM main = FROM fts` 검증 → 불일치 시 rebuild |
| 동시 writer (앱 + CLI) 가 SQLITE_BUSY | WAL 모드 + `busy_timeout=3000ms` 설정. CLI 도 같은 pragma 적용 |

## Open Questions

- 앱의 "Undo 토스트" 를 audit_log 기반으로 통합하는 시점 — 이 스펙에서는 CLI 가 audit_log 를 _쓰기_ 시작하는 것까지만 다루고, 앱의 기존 Undo UX 를 같은 소스에서 _읽도록_ 바꾸는 건 후속 스펙으로 분리 가능. 구현 계획 단계에서 재확인.

## Out of Scope (후속 스펙)

1. **Agent Skills 세트** — 이 CLI 를 감싸 Claude Code / Codex 에서 호출하는 스킬들. 예: 폴더 정리, today 브리핑, 메모 컬러 재정리.
2. **자동 배포 파이프라인** — Homebrew tap + GitHub Release 정적 바이너리 + Claude Code plugin registry publish.
3. **앱 Activity 뷰** — audit_log 를 시각화.
4. **CLI 의 OpenAI 프록시** — `hearth ai "..."` 형태. YAGNI.
