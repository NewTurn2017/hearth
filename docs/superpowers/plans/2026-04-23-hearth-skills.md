# Hearth Agent Skills v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship 3 v1 agent skills (`hearth-today-brief`, `hearth-project-scan`, `hearth-memo-organize`) plus a manual install script and an end-to-end smoke test. All skills are standard `SKILL.md` files callable from Claude Code, Codex, or any host that loads the format.

**Architecture:** Flat `skills/<name>/SKILL.md` directory at repo root. Each SKILL.md is self-contained (YAML frontmatter + Markdown body) with a pinned Preamble → Read → Synthesis → (Mutation → Close) flow. Mutating skills always require an explicit user approval gate before applying CLI writes. `scripts/install-skills.sh` symlinks skill folders into a user-chosen target dir. `scripts/smoke-skills.sh` seeds a fixture DB and exercises every CLI recipe each skill depends on.

**Tech Stack:** Markdown + YAML (SKILL.md), bash (`install-skills.sh`, `smoke-skills.sh`), `jq`, existing `hearth` CLI binary. No new Rust code.

**Spec:** `docs/superpowers/specs/2026-04-23-hearth-skills-design.md` (commit `6f7c0e0`).

---

## Verified CLI contract (cross-checked against CLI source)

| Recipe | Exact command | Data shape the skill parses |
|---|---|---|
| Today brief — today | `"$HEARTH" today` | `data.{date, schedules_today[], p0_projects[], recent_memos[]}` |
| Today brief — overdue | `"$HEARTH" overdue` | `data.{overdue_schedules[], stale_projects[]}` |
| Project scan | `"$HEARTH" project scan "$DIR"` | `data[]` — each item `{path, name, already_registered}` (no `has_git`/`size_mb`) |
| Existing projects | `"$HEARTH" project list` | `data[]` — each item `{id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at}` |
| Project create | `"$HEARTH" project create "<name>" --priority <P> --path "<path>"` | returns `data` = created project row |
| Memo list | `"$HEARTH" memo list` | `data[]` — each item `{id, content, color, project_id, sort_order, created_at, updated_at}` (no `--limit` flag; slice client-side if needed) |
| Memo update | `"$HEARTH" memo update <id> --project <pid>` (or `--detach`, `--color`, `--content`) | returns `data` = updated memo row |
| Audit log | `"$HEARTH" log show --limit <N>` | `data[]` — audit entries |
| Undo | `"$HEARTH" undo [count]` | `data.{undone, entries}` |

Every success response has `"ok": true`. Errors have `"ok": false`, plus `"error"` and optional `"hint"` — skills MUST forward those fields verbatim to the user.

**Corrections vs the spec text** (intentional — plan resolves them, spec open questions 10.1 and 10.2 answered):
- Spec wrote `--linked-path` → CLI flag is actually `--path`. Plan uses `--path`.
- Spec wrote `hearth log --limit N` → CLI subcommand is actually `hearth log show --limit N`. Plan uses `show`.
- Spec wrote `hearth memo list --limit 200` → CLI has no such flag. Plan drops the flag; skill uses bare `hearth memo list`.
- Spec wrote `hearth overdue` returns `past_schedules` → actual field is `overdue_schedules`. Plan uses the actual field name.
- Spec open question on `has_git`/`size_mb`: confirmed NOT emitted. Plan pins `{path, name, already_registered}` only.
- Spec open question on memo color heuristic: project color is derived from its category (hex like `#22c55e`), while memo colors are named (e.g. `yellow`) — not directly comparable. v1 drops the color-based suggestion and only proposes `--project` reassignment when the project name token clearly appears in the memo content AND exactly one project matches. `--color` changes are deferred to v2.

---

## File Structure

Created by this plan:

```
skills/
├── README.md                              # prereq + install + troubleshooting
├── hearth-today-brief/SKILL.md            # read-only briefing
├── hearth-project-scan/SKILL.md           # folder → project registration (mutation-on-approval)
└── hearth-memo-organize/SKILL.md          # memo → project reassignment (mutation-on-approval)
scripts/
├── install-skills.sh                      # symlink skills/* into user-specified dir
├── smoke-skills.sh                        # regression: seeds DB + exercises recipes
└── tests/
    └── test_install_skills.sh             # TDD harness for install-skills.sh
CHANGELOG.md                               # (modified) add skills section under [0.7.0]
docs/superpowers/specs/2026-04-23-hearth-skills-design.md   # (modified) check off deliverables
```

Not modified: the `hearth` binary itself, any Rust crate, or any app-side code. v1 is pure docs + shell.

---

## Task 1: skills/ root + README.md

**Files:**
- Create: `skills/README.md`
- Create: `skills/` directory (implicit when README is written)

- [ ] **Step 1: Create `skills/README.md`**

Write this content to `skills/README.md`:

````markdown
# Hearth Agent Skills

Agent-facing skills that orchestrate the `hearth` CLI. They work with any host that loads standard `SKILL.md` files (Claude Code, Codex, and others that adopt the format).

## Prerequisite

Build and install the `hearth` binary (from this repo):

```bash
cd src-tauri && cargo build --release -p hearth-cli
# option A: copy to a dir on PATH
cp target/release/hearth /usr/local/bin/hearth
# option B: point $HEARTH_BIN at the build output
export HEARTH_BIN="$(pwd)/target/release/hearth"
# sanity check
hearth db path   # prints a path and exits 0
```

Every skill resolves the binary in this order: `$HEARTH_BIN` → `hearth` on PATH → abort with an install hint.

## Install (manual, v1)

Symlink the skill folders into your agent host's skills directory:

```bash
# Claude Code (example default)
./scripts/install-skills.sh --into ~/.claude/skills
# Codex (example default)
./scripts/install-skills.sh --into ~/.codex/skills
# Uninstall (removes only the symlinks this script created)
./scripts/install-skills.sh --into ~/.claude/skills --remove
```

The script requires `--into` explicitly (no silent default) and refuses to overwrite non-symlink files that already exist at the target.

## Skills shipped in v1

| Name | Kind | What it does |
|---|---|---|
| `hearth-today-brief` | read-only | 3–5문장 한국어 브리핑 (오늘 일정 + P0 + 최근 메모 + 연체 항목) |
| `hearth-project-scan` | read + mutate-on-approval | 디렉토리 스캔 → 미등록 폴더를 프로젝트로 등록 제안 |
| `hearth-memo-organize` | read + mutate-on-approval | 메모를 프로젝트로 보수적 재연결 제안 |

Mutating skills always propose a plan and wait for explicit user approval before running any `hearth ... create|update|delete`.

## Smoke test

```bash
./scripts/smoke-skills.sh       # seeds fixture DB, runs each skill's CLI recipes, asserts ok:true
```

## Troubleshooting

- `hearth: command not found` — binary not built/installed. See Prerequisite.
- `{"ok":false,"error":"..."}` — CLI surfaced an error; the skill forwards `error`/`hint` verbatim. Run the command yourself to see details.
- Skill never triggers — confirm the host reads the `description` field from YAML frontmatter. Host-specific matching varies; each SKILL.md lists KR + EN trigger phrases.

## See also

- Design spec: `docs/superpowers/specs/2026-04-23-hearth-skills-design.md`
- Implementation plan: `docs/superpowers/plans/2026-04-23-hearth-skills.md`
- CLI reference: top-level `README.md` → "Hearth CLI" section
````

- [ ] **Step 2: Verify the file exists**

Run:

```bash
ls -la skills/README.md
```

Expected: file exists, non-zero size.

- [ ] **Step 3: Commit**

```bash
git add skills/README.md
git commit -m "docs(skills): v1 skills README (prereq + install + troubleshooting)"
```

---

## Task 2: `hearth-today-brief` skill (read-only briefing)

**Files:**
- Create: `skills/hearth-today-brief/SKILL.md`

- [ ] **Step 1: Create the skill file**

Write this content to `skills/hearth-today-brief/SKILL.md`:

````markdown
---
name: hearth-today-brief
description: |
  "오늘 뭐해", "오늘 브리핑", "today brief", "morning rundown" — hearth 로컬 워크스페이스에서 오늘 일정·P0 프로젝트·최근 메모·연체 항목을 한국어로 요약. Orchestrates `hearth today` and `hearth overdue` (read-only) to produce a 3–5 sentence Korean briefing.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정 (순서대로 시도, 첫 성공을 `HEARTH` 로 사용):
   - `$HEARTH_BIN` 이 설정되어 있고 해당 파일이 실행 가능하면 그 값.
   - PATH 의 `hearth` (`command -v hearth` 이 성공하는 경우).
   - 둘 다 실패하면 아래 문구로 즉시 중단하고 이후 어떤 CLI 도 호출하지 마세요:
     > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. 동작 확인: `"$HEARTH" db path` 를 실행. exit code 0 이 아니거나 `"ok": false` 면 stdout 의 `error`/`hint` 필드를 그대로 사용자에게 전달한 뒤 중단.

# Trigger

- KR: "오늘 뭐해", "오늘 브리핑", "오늘 할 일", "모닝 브리핑", "오늘 일정 요약"
- EN: "today brief", "morning rundown", "what's on my plate today", "hearth today summary"
- Recommended cadence: 매일 아침. 스케줄링(예: Codex cron `0 7 * * *`, CC session-start hook)은 각 에이전트 호스트가 담당.

# Read phase (순서 고정)

1. `"$HEARTH" today`
   - 응답의 `ok == true` 확인. 실패면 즉시 중단하고 `error`/`hint` 전달.
   - 파싱할 필드: `data.date`, `data.schedules_today[]`, `data.p0_projects[]`, `data.recent_memos[]`.
2. `"$HEARTH" overdue`
   - 응답의 `ok == true` 확인. 실패면 즉시 중단.
   - 파싱할 필드: `data.overdue_schedules[]`, `data.stale_projects[]`.
   - 두 배열이 모두 비어 있으면 요약에서 연체 문장을 생략.

# Synthesis

아래 조건에 맞춰 **한국어 3~5문장** 브리핑을 작성하세요. 존재하지 않는 정보는 지어내지 마세요.

1. 첫 문장: `data.date` + 오늘 일정 개수 + (있으면) 가장 이른 일정의 `time`·`description`. 일정이 0건이면 "오늘 등록된 일정 없음".
2. 두 번째 문장: `p0_projects[]` 중 `updated_at` 이 가장 오래된 1~2개의 `name` 언급. P0 가 0건이면 "P0 프로젝트 없음".
3. (선택) 세 번째 문장: `recent_memos[]` 개수 + 가장 최근 메모의 `content` 첫 60자(60자 초과 시 "…"). 없으면 생략.
4. (조건부) 연체 문장: `overdue_schedules` / `stale_projects` 가 있으면 각각 개수 + 가장 오래된 항목 1건을 한 줄로.

출력 형태 예시 (그대로 베끼지 말고 실제 데이터 기반):
> 오늘(2026-04-23) 일정 2건 — 10:00 팀 싱크, 14:00 치과. P0 "Hearth v0.8 release" 가 4일째 업데이트 없음. 최근 24시간 메모 3개 (최신: "스킬 브레인스토밍 끝…"). 연체 일정 1건(4/20 문서 정리) 남아있습니다.

# Mutation phase

이 스킬은 읽기 전용입니다. `hearth` 의 변경 계열 명령 (`create`, `update`, `delete`, `undo`, `redo`, `import`) 을 절대 호출하지 마세요.
````

- [ ] **Step 2: Verify YAML frontmatter parses**

Run:

```bash
awk '/^---$/{c++; next} c==1' skills/hearth-today-brief/SKILL.md | head -5
```

Expected: prints the `name:` and `description:` block cleanly (no parsing errors visible, description continues across lines under `|`).

- [ ] **Step 3: Commit**

```bash
git add skills/hearth-today-brief/SKILL.md
git commit -m "feat(skills): hearth-today-brief — read-only KR briefing"
```

---

## Task 3: `hearth-project-scan` skill (folder → project registration)

**Files:**
- Create: `skills/hearth-project-scan/SKILL.md`

- [ ] **Step 1: Create the skill file**

Write this content to `skills/hearth-project-scan/SKILL.md`:

````markdown
---
name: hearth-project-scan
description: |
  "이 폴더 스캔", "scan this dir", "프로젝트 후보 찾아줘" — 지정한 디렉토리의 하위 폴더 중 아직 hearth 프로젝트로 등록되지 않은 것을 찾아내고, 사용자 승인 후에만 일괄 등록. Orchestrates `hearth project scan` + `hearth project create` with an explicit propose → approve → apply gate.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정 (순서대로): `$HEARTH_BIN` 이 실행 가능하면 그 값 → PATH 의 `hearth` → 실패 시 아래 문구로 중단:
   > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. `"$HEARTH" db path` 로 동작 확인. 실패면 `error`/`hint` 전달 후 중단.

# Trigger

- KR: "이 폴더 스캔", "프로젝트 후보", "이 디렉토리를 hearth 에 등록", "여기 폴더 정리"
- EN: "scan this dir", "register projects from folder", "hearth project scan"
- Recommended cadence: 수동 호출 (신규 워크스페이스를 처음 탐색할 때).

# Preconditions — 대상 디렉토리 확보

사용자 메시지 안에 절대 경로가 없으면 딱 한 번 질문하세요:

> "어느 폴더를 스캔할까요? 절대 경로로 알려주세요."

받은 경로를 `DIR` 로 씁니다. `DIR` 이 실제 디렉토리가 아니면 즉시 "폴더가 존재하지 않습니다: <DIR>" 로 중단.

# Read phase (순서 고정)

1. `"$HEARTH" project scan "$DIR"`
   - 응답: `data[]` = 각 `{path, name, already_registered}`. `ok == true` 확인.
   - 후보 필터: `already_registered == false` 인 항목만 이후 단계로 진행.
2. `"$HEARTH" project list`
   - 응답: `data[]` 의 `path` 집합을 구해 race-condition 이중 체크. 이미 등록된 경로가 후보에 섞여 있으면 제거.

필터링 후 후보가 0 개면 "`$DIR` 에서 새 프로젝트 후보 없음" 을 출력하고 종료. 이후 어떤 mutation 도 호출하지 마세요.

# Propose — 사용자에게 제시

후보를 **Markdown 표** 로 출력하세요:

| # | name | path | 제안 priority |
|---|------|------|---------------|
| 1 | <item.name> | <item.path> | P2 |
| 2 | <item.name> | <item.path> | P2 |

바로 다음에 한 번 묻습니다:

> "어떤 번호를 등록할까요? 예시:
> - `1,2` — 해당 번호만 기본 P2 로 등록
> - `all` — 모두 기본 P2 로 등록
> - `1 P0, 2 P1` — 번호별 priority 지정
> - `none` — 취소
> 기본값은 `none` (아무 것도 하지 않음) 입니다."

응답이 모호하면 다시 물어보세요 (예: 숫자가 후보 범위를 벗어남, priority 가 P0/P1/P2/P3 이외). 사용자가 `none` 을 고르면 여기서 종료.

# Mutation phase — 승인된 항목만 순차 등록

승인된 항목 각각에 대해 순서대로 (선택 priority, 기본 P2):

```
"$HEARTH" project create "<name>" --priority <P> --path "<path>"
```

- 각 호출 후 응답의 `ok == true` 확인. 실패면 즉시 중단하고 사용자에게:
  > "N / M 건 등록 완료. 실패 항목: <name> (error=<error 전문>, hint=<hint 전문>). 나머지는 취소됨."
- 모두 성공하면:
  > "M 건 등록 완료 (id: <id1>, <id2>, ...)."

# Close — 로그 + undo 안내

1. (선택) `"$HEARTH" log show --limit 10` 결과를 요약해 어떤 mutation 이 기록되었는지 한 줄로 언급해도 됩니다.
2. 마지막 줄로 반드시 출력:
   > "되돌리려면 `hearth undo M` 를 실행하세요 (M = 방금 등록한 건수). 일괄 취소됩니다."
````

- [ ] **Step 2: Verify the file exists and frontmatter parses**

Run:

```bash
test -f skills/hearth-project-scan/SKILL.md && awk '/^---$/{c++; next} c==1' skills/hearth-project-scan/SKILL.md
```

Expected: prints `name: hearth-project-scan` and the `description:` block.

- [ ] **Step 3: Commit**

```bash
git add skills/hearth-project-scan/SKILL.md
git commit -m "feat(skills): hearth-project-scan — folder→project registration with approval gate"
```

---

## Task 4: `hearth-memo-organize` skill (memo → project reassignment)

**Files:**
- Create: `skills/hearth-memo-organize/SKILL.md`

- [ ] **Step 1: Create the skill file**

Write this content to `skills/hearth-memo-organize/SKILL.md`:

````markdown
---
name: hearth-memo-organize
description: |
  "메모 정리", "메모 재분류", "organize my memos", "memo tidy" — hearth 메모 목록을 읽고, 내용이 특정 프로젝트에 명확히 속하는 경우에 한해 보수적으로 재연결을 제안, 승인 후 일괄 적용. Orchestrates `hearth memo list` + `hearth project list` → propose → `hearth memo update --project`.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정: `$HEARTH_BIN` → PATH 의 `hearth` → 실패 시:
   > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. `"$HEARTH" db path` 로 동작 확인. 실패면 `error`/`hint` 전달 후 중단.

# Trigger

- KR: "메모 정리", "메모 재분류", "메모 프로젝트 연결", "메모 쪽 정리"
- EN: "organize my memos", "memo tidy", "reclassify memos", "link memos to projects"
- Recommended cadence: 수동 (예: 주 1회).

# Read phase

1. `"$HEARTH" memo list`
   - 응답: `data[]` = `[{id, content, color, project_id, ...}]`. `ok == true` 확인.
   - (CLI 자체에는 `--limit` 플래그가 없습니다. 메모가 수백 개 이상이면 결과를 클라이언트 쪽에서 잘라 처리하세요.)
2. `"$HEARTH" project list`
   - 응답: `data[]` = 각 `{id, name, ...}`. `ok == true` 확인.
   - `id` → `name` 매핑 테이블을 메모리에 준비.

# Propose — 보수적 재연결 규칙

아래 **모든** 조건을 만족하는 메모만 제안하세요. 애매하면 건너뛰기.

1. 메모의 현재 상태가 다음 중 하나:
   - `project_id` 가 `null`
   - `project_id` 가 가리키는 프로젝트의 `name` 토큰이 메모 `content` 에 포함되어 있지 않음 (링크가 엉뚱하게 걸려 있는 경우).
2. 후보 프로젝트의 `name` 을 공백 기준으로 토큰화하고, 각 토큰이 2자 이상이며 메모 `content` (대소문자 무시) 에 포함되어야 함.
3. 위 조건을 만족하는 후보가 **정확히 1개** 일 때만 제안. 2개 이상 매칭되면 ambiguous 로 판단, 건너뛰기.

`--color` 재지정은 v1 범위 밖입니다 (프로젝트 색은 카테고리에서 파생된 hex 코드, 메모 색은 별도 팔레트 → 직접 매핑 불가). 이번 스킬은 `--project` 재연결만 처리합니다.

제안 표:

| memo.id | 내용(첫 40자) | 현재 project | 제안 project |
|---------|---------------|--------------|--------------|
| 12 | "Hearth CLI audit log 테스트…" | (none) | Hearth v0.8 release |
| 27 | "…" | 잘못된 프로젝트 | 올바른 프로젝트 |

그 뒤 한 번 묻습니다:

> "모두 적용할까요? 일부만 적용하려면 memo.id 를 쉼표로 주세요 (예: `12,27`). `all` 전체 적용, `none` 취소. 기본값은 `none`."

모호한 응답은 다시 물어보세요. 제안이 0 건이면 "명확히 재연결할 메모 없음" 으로 종료, 이후 mutation 호출 금지.

# Mutation phase — 승인 항목만 순차 적용

각 승인 항목에 대해:

```
"$HEARTH" memo update <memo.id> --project <project.id>
```

- 각 호출 후 `ok == true` 확인. 첫 실패에서 중단하고:
  > "N / M 건 적용 완료. 실패 항목: memo #<id> (error=<...>, hint=<...>). 나머지는 취소됨."
- 전부 성공하면:
  > "M 건 재연결 완료."

# Close — 로그 + undo 안내

1. (선택) `"$HEARTH" log show --limit 10` 을 호출해 최근 기록 요약.
2. 마지막 줄로 반드시:
   > "되돌리려면 `hearth undo M` 을 실행하세요 (M = 방금 적용한 건수). 일괄 취소됩니다."
````

- [ ] **Step 2: Verify file exists and frontmatter parses**

Run:

```bash
test -f skills/hearth-memo-organize/SKILL.md && awk '/^---$/{c++; next} c==1' skills/hearth-memo-organize/SKILL.md
```

Expected: prints `name: hearth-memo-organize` block.

- [ ] **Step 3: Commit**

```bash
git add skills/hearth-memo-organize/SKILL.md
git commit -m "feat(skills): hearth-memo-organize — conservative memo→project reassign with approval gate"
```

---

## Task 5: `scripts/install-skills.sh` (TDD)

**Files:**
- Create: `scripts/tests/test_install_skills.sh`
- Create: `scripts/install-skills.sh`

- [ ] **Step 1: Write the failing test**

Create `scripts/tests/test_install_skills.sh`:

```bash
#!/usr/bin/env bash
# Harness for scripts/install-skills.sh. Exits 0 on all-pass.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INSTALL="$REPO_ROOT/scripts/install-skills.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "PASS: $*"; }

[[ -x "$INSTALL" ]] || fail "install-skills.sh missing or not executable"

TMP="$(mktemp -d -t hearth-install-test.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

# 1. Missing --into must error with exit 64.
if "$INSTALL" >/dev/null 2>&1; then
  fail "expected non-zero exit when --into omitted"
fi
code=$("$INSTALL" >/dev/null 2>&1; echo $?)
[[ "$code" == "64" ]] || fail "expected exit 64 when --into omitted, got $code"
pass "missing --into exits 64"

# 2. Happy path: install links each skills/* as symlink into target.
TARGET="$TMP/target"
"$INSTALL" --into "$TARGET" >"$TMP/install.log" 2>&1 \
  || { cat "$TMP/install.log" >&2; fail "install failed"; }

for skill in "$REPO_ROOT/skills"/*/; do
  name="$(basename "$skill")"
  link="$TARGET/$name"
  [[ -L "$link" ]] || fail "$link is not a symlink"
  resolved="$(readlink "$link")"
  expected="$(cd "$skill" && pwd)"
  [[ "$resolved" == "$expected" ]] || fail "$link -> $resolved (expected $expected)"
done
pass "symlinks created for all skills"

# 3. Re-running install is idempotent (replaces symlinks, no error).
"$INSTALL" --into "$TARGET" >/dev/null 2>&1 || fail "second install errored"
pass "idempotent re-install"

# 4. Refuses to overwrite a non-symlink at the target.
# Remove one symlink and put a real file there.
first_skill="$(find "$TARGET" -mindepth 1 -maxdepth 1 -type l | head -n1)"
rm "$first_skill"
echo "real file" > "$first_skill"
if "$INSTALL" --into "$TARGET" >/dev/null 2>&1; then
  fail "expected failure when non-symlink exists at target"
fi
pass "refuses to overwrite non-symlink"
# Cleanup for next test
rm "$first_skill"

# 5. --remove only removes symlinks this script would create.
# Add an unrelated symlink inside target; verify it survives --remove.
ln -s /tmp "$TARGET/unrelated-link"
"$INSTALL" --into "$TARGET" >/dev/null 2>&1 || fail "reinstall before remove"
"$INSTALL" --into "$TARGET" --remove >/dev/null 2>&1 || fail "--remove errored"
for skill in "$REPO_ROOT/skills"/*/; do
  name="$(basename "$skill")"
  [[ ! -e "$TARGET/$name" ]] || fail "$TARGET/$name still present after --remove"
done
[[ -L "$TARGET/unrelated-link" ]] || fail "--remove deleted an unrelated symlink"
pass "--remove surgical"

echo
echo "ALL GOOD"
```

Then make it executable:

```bash
chmod +x scripts/tests/test_install_skills.sh
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```bash
./scripts/tests/test_install_skills.sh
```

Expected: FAIL with `install-skills.sh missing or not executable` (because Step 1 of this task only created the test, not the install script yet).

- [ ] **Step 3: Write the install script**

Create `scripts/install-skills.sh`:

```bash
#!/usr/bin/env bash
# Install or remove symlinks for hearth agent skills into a host's skills dir.
# Usage: install-skills.sh --into <path> [--remove]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILLS_DIR="$REPO_ROOT/skills"

INTO=""
REMOVE=0

usage() {
  cat <<EOF
Usage: $0 --into <path> [--remove]

  --into <path>    Target skills dir. No default — must be explicit.
  --remove         Remove only the symlinks this script would create.
  -h, --help       Show this help.

Hints (these are NOT defaults — always pass --into explicitly):
  Claude Code:  ~/.claude/skills   (\$CLAUDE_SKILLS_DIR if set)
  Codex:        ~/.codex/skills    (\$CODEX_SKILLS_DIR if set)
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --into)
      [[ $# -ge 2 ]] || { echo "--into needs a value" >&2; usage >&2; exit 64; }
      INTO="$2"; shift 2 ;;
    --remove) REMOVE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown argument: $1" >&2; usage >&2; exit 64 ;;
  esac
done

if [[ -z "$INTO" ]]; then
  usage >&2
  exit 64
fi

mkdir -p "$INTO"

count=0
for skill_path in "$SKILLS_DIR"/*/; do
  [[ -d "$skill_path" ]] || continue
  name="$(basename "$skill_path")"
  target="$INTO/$name"

  if [[ $REMOVE -eq 1 ]]; then
    if [[ -L "$target" ]]; then
      rm "$target"
      echo "removed: $target"
      count=$((count+1))
    else
      echo "skip (not a symlink we created): $target"
    fi
  else
    if [[ -e "$target" && ! -L "$target" ]]; then
      echo "refusing to overwrite non-symlink: $target" >&2
      exit 1
    fi
    abs_skill="$(cd "$skill_path" && pwd)"
    ln -snf "$abs_skill" "$target"
    echo "linked: $target -> $abs_skill"
    count=$((count+1))
  fi
done

echo
if [[ $REMOVE -eq 1 ]]; then
  echo "Done. Removed $count symlink(s) from $INTO"
else
  echo "Done. Installed $count skill(s) into $INTO"
  echo "Verify: ls -l \"$INTO\" | grep hearth-"
fi
```

Then make it executable:

```bash
chmod +x scripts/install-skills.sh
```

- [ ] **Step 4: Run the test to verify it passes**

Run:

```bash
./scripts/tests/test_install_skills.sh
```

Expected: all PASS lines print, final line `ALL GOOD`, exit code 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/install-skills.sh scripts/tests/test_install_skills.sh
git commit -m "feat(scripts): install-skills.sh + TDD harness (--into, --remove, no overwrite)"
```

---

## Task 6: `scripts/smoke-skills.sh` (regression)

**Files:**
- Create: `scripts/smoke-skills.sh`

This script seeds a throwaway DB, exercises every CLI recipe each skill relies on, and asserts `ok:true`. It IS the regression test — no meta-test needed.

- [ ] **Step 1: Write the smoke script**

Create `scripts/smoke-skills.sh`:

```bash
#!/usr/bin/env bash
# Regression test: exercise every CLI recipe the v1 hearth skills depend on.
# Requires: jq, built `hearth` binary (HEARTH_BIN or src-tauri/target/release/hearth).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

HEARTH="${HEARTH_BIN:-$REPO_ROOT/src-tauri/target/release/hearth}"
if [[ ! -x "$HEARTH" ]]; then
  echo "hearth binary not found at: $HEARTH" >&2
  echo "Build first: (cd src-tauri && cargo build --release -p hearth-cli)" >&2
  echo "Or export HEARTH_BIN=<path-to-hearth>" >&2
  exit 1
fi

command -v jq >/dev/null 2>&1 || { echo "jq is required" >&2; exit 1; }

TMP="$(mktemp -d -t hearth-smoke.XXXXXX)"
trap 'rm -rf "$TMP"' EXIT

export HEARTH_DB="$TMP/smoke.db"
SCANDIR="$TMP/workspace"
mkdir -p "$SCANDIR/alpha" "$SCANDIR/beta"

assert_ok() {
  local label="$1" out="$2"
  if ! echo "$out" | jq -e '.ok == true' >/dev/null 2>&1; then
    echo "FAIL: $label" >&2
    echo "$out" >&2
    exit 1
  fi
  echo "PASS: $label"
}

# Portable "tomorrow"/"yesterday" — BSD date on macOS vs GNU date on Linux.
date_rel() {
  if date -v+0d +%Y-%m-%d >/dev/null 2>&1; then
    date -v"$1"d +%Y-%m-%d
  else
    case "$1" in
      +1) date -d 'tomorrow' +%Y-%m-%d ;;
      -1) date -d 'yesterday' +%Y-%m-%d ;;
      *) date -d "$1 days" +%Y-%m-%d ;;
    esac
  fi
}

TODAY="$(date +%Y-%m-%d)"
YESTERDAY="$(date_rel -1)"

echo "== migrate =="
assert_ok "db migrate" "$("$HEARTH" db migrate)"

echo "== seed fixture data =="
P1=$("$HEARTH" project create "Alpha Dashboard" --priority P0 | jq '.data.id')
P2=$("$HEARTH" project create "Side Experiment" --priority P2 | jq '.data.id')
"$HEARTH" memo create "Alpha Dashboard sprint notes" >/dev/null
"$HEARTH" memo create "Side Experiment notebook"     >/dev/null
"$HEARTH" memo create "Unrelated thought"             >/dev/null
"$HEARTH" schedule create "$TODAY"     --time 10:00 --description "team sync"       >/dev/null
"$HEARTH" schedule create "$YESTERDAY"              --description "overdue cleanup" >/dev/null
echo "   seeded projects #$P1 (P0), #$P2 (P2), 3 memos, 2 schedules"

echo "== hearth-today-brief read recipes =="
assert_ok "today"   "$("$HEARTH" today)"
assert_ok "overdue" "$("$HEARTH" overdue)"

echo "== hearth-project-scan read + mutation =="
SCAN=$("$HEARTH" project scan "$SCANDIR")
assert_ok "project scan" "$SCAN"
UNREG=$(echo "$SCAN" | jq '[.data[] | select(.already_registered == false)] | length')
[[ "$UNREG" -eq 2 ]] || { echo "expected 2 unregistered, got $UNREG" >&2; echo "$SCAN" >&2; exit 1; }
echo "PASS: project scan found 2 unregistered candidates"

BEFORE=$("$HEARTH" project list | jq '.data | length')
for sub in alpha beta; do
  assert_ok "project create $sub" "$("$HEARTH" project create "$sub" --priority P2 --path "$SCANDIR/$sub")"
done
AFTER=$("$HEARTH" project list | jq '.data | length')
[[ $((BEFORE + 2)) -eq "$AFTER" ]] || { echo "expected +2 projects ($BEFORE -> $AFTER)" >&2; exit 1; }
echo "PASS: project count $BEFORE -> $AFTER"

echo "== hearth-memo-organize read + mutation =="
assert_ok "memo list" "$("$HEARTH" memo list)"
MID=$("$HEARTH" memo list | jq '.data[] | select(.content == "Alpha Dashboard sprint notes") | .id')
[[ -n "$MID" ]] || { echo "seeded memo not found" >&2; exit 1; }
assert_ok "memo update --project" "$("$HEARTH" memo update "$MID" --project "$P1")"
LINKED=$("$HEARTH" memo get "$MID" | jq '.data.project_id')
[[ "$LINKED" -eq "$P1" ]] || { echo "memo project_id expected $P1, got $LINKED" >&2; exit 1; }
echo "PASS: memo #$MID linked to project #$P1"

echo "== undo round-trip =="
assert_ok "undo" "$("$HEARTH" undo)"
LINKED2=$("$HEARTH" memo get "$MID" | jq '.data.project_id')
[[ "$LINKED2" != "$P1" ]] || { echo "undo did not revert memo project_id" >&2; exit 1; }
echo "PASS: undo reverted memo #$MID link (now project_id=$LINKED2)"

echo "== log show (close-phase recipe) =="
assert_ok "log show --limit 10" "$("$HEARTH" log show --limit 10)"

echo
echo "ALL GOOD"
```

Then make it executable:

```bash
chmod +x scripts/smoke-skills.sh
```

- [ ] **Step 2: Ensure the hearth binary is built**

Run:

```bash
cd src-tauri && cargo build --release -p hearth-cli
cd ..
ls -la src-tauri/target/release/hearth
```

Expected: binary exists and is executable.

- [ ] **Step 3: Run the smoke script end-to-end**

Run:

```bash
./scripts/smoke-skills.sh
```

Expected: every line prefixed `PASS:`, final line `ALL GOOD`, exit code 0.

If it fails, read the printed JSON; the skill files will need to match the actual CLI contract. Do not edit the plan — fix the underlying file (usually SKILL.md text or the smoke script itself).

- [ ] **Step 4: Commit**

```bash
git add scripts/smoke-skills.sh
git commit -m "feat(scripts): smoke-skills.sh — end-to-end CLI regression for v1 skills"
```

---

## Task 7: Spec deliverables checklist + CHANGELOG + top-level README link

**Files:**
- Modify: `docs/superpowers/specs/2026-04-23-hearth-skills-design.md` (section 8 checklist)
- Modify: `CHANGELOG.md` (under `[0.7.0]`)
- Modify: `README.md` (add a Skills pointer)

- [ ] **Step 1: Check off the deliverables in the spec**

Edit `docs/superpowers/specs/2026-04-23-hearth-skills-design.md`, section 8, flipping each `- [ ]` to `- [x]` for the 6 concrete deliverables (all now shipped by this plan):

Replace (exact old string):

```
- [ ] `skills/README.md` — prereq + install + troubleshooting
- [ ] `skills/hearth-today-brief/SKILL.md`
- [ ] `skills/hearth-project-scan/SKILL.md`
- [ ] `skills/hearth-memo-organize/SKILL.md`
- [ ] `scripts/install-skills.sh` (with `--into`, `--remove`, env hints)
- [ ] `scripts/smoke-skills.sh` (covers read + one mutation + undo per skill)
- [ ] This spec committed
- [ ] Implementation plan (next step, writing-plans skill)
```

With:

```
- [x] `skills/README.md` — prereq + install + troubleshooting
- [x] `skills/hearth-today-brief/SKILL.md`
- [x] `skills/hearth-project-scan/SKILL.md`
- [x] `skills/hearth-memo-organize/SKILL.md`
- [x] `scripts/install-skills.sh` (with `--into`, `--remove`, env hints)
- [x] `scripts/smoke-skills.sh` (covers read + one mutation + undo per skill)
- [x] This spec committed
- [x] Implementation plan (this file: `docs/superpowers/plans/2026-04-23-hearth-skills.md`)
```

- [ ] **Step 2: Add a CHANGELOG entry under `[0.7.0]`**

In `CHANGELOG.md`, under the existing `## [0.7.0] - (unreleased)` heading's `### Added` block, append (after the "Search powered by FTS5" bullet):

```
- **Agent Skills (v1)**: 3 skills callable from Claude Code, Codex, or any host that loads standard `SKILL.md`:
  - `hearth-today-brief` — read-only 한국어 브리핑 (오늘 일정 + P0 + 최근 메모 + 연체).
  - `hearth-project-scan` — 디렉토리 → hearth 프로젝트 등록. 사용자 승인 후에만 적용.
  - `hearth-memo-organize` — 메모 → 프로젝트 보수적 재연결. 승인 후에만 적용.
- **`scripts/install-skills.sh`**: manual install path — symlinks `skills/*` into a user-specified dir. Requires explicit `--into`; supports `--remove`.
- **`scripts/smoke-skills.sh`**: seeds a throwaway DB and exercises every CLI recipe each v1 skill depends on.
```

- [ ] **Step 3: Add a Skills pointer to top-level README.md**

The "Hearth CLI" section in `README.md` ends with a `### 안전성` subsection (the paragraph about `audit_log` / `undo` / `redo`) directly before `## Building from Source`. Insert a new `### Agent Skills` subsection between them.

Use Edit with:

old_string (exact, including the blank line before the next top-level heading):

````
모든 mutation 은 `audit_log` 에 기록됩니다. `hearth log undo` / `redo` 로 되돌릴 수 있고, `hearth log show` 로 히스토리 조회 가능. 앱·CLI 변경이 하나의 히스토리를 공유합니다.

## Building from Source
````

new_string:

````
모든 mutation 은 `audit_log` 에 기록됩니다. `hearth log undo` / `redo` 로 되돌릴 수 있고, `hearth log show` 로 히스토리 조회 가능. 앱·CLI 변경이 하나의 히스토리를 공유합니다.

### Agent Skills

CLI 은 `SKILL.md` 파일들이 얹히는 기반입니다. v1 에서 세 가지가 `skills/` 에 포함됩니다:

- `hearth-today-brief` — 오늘 브리핑 (read-only)
- `hearth-project-scan` — 디렉토리 → 프로젝트 등록 (mutation, 승인 필요)
- `hearth-memo-organize` — 메모 → 프로젝트 재연결 (mutation, 승인 필요)

에이전트 호스트에 설치:

```bash
./scripts/install-skills.sh --into ~/.claude/skills       # Claude Code
./scripts/install-skills.sh --into ~/.codex/skills        # Codex
```

자세한 prereq · install · troubleshooting 은 [`skills/README.md`](skills/README.md) 참고.

## Building from Source
````

If an earlier commit has already added an "Agent Skills" subsection, the Edit will fail on the anchor — skip Step 3 in that case and note it in Step 4's diff.

- [ ] **Step 4: Verify all three edits**

Run:

```bash
git diff --stat README.md CHANGELOG.md docs/superpowers/specs/2026-04-23-hearth-skills-design.md
```

Expected: all three files show modifications.

- [ ] **Step 5: Commit**

```bash
git add CHANGELOG.md README.md docs/superpowers/specs/2026-04-23-hearth-skills-design.md
git commit -m "docs: v1 agent skills — CHANGELOG, README pointer, spec checklist"
```

---

## Task 8: Final end-to-end verification

Everything ships — this is the sanity pass before handoff.

- [ ] **Step 1: Run the install harness**

```bash
./scripts/tests/test_install_skills.sh
```

Expected: `ALL GOOD`.

- [ ] **Step 2: Run the CLI regression**

```bash
./scripts/smoke-skills.sh
```

Expected: `ALL GOOD`.

- [ ] **Step 3: Dry-run an actual install into a scratch dir**

```bash
SCRATCH="$(mktemp -d)"
./scripts/install-skills.sh --into "$SCRATCH"
ls -l "$SCRATCH"
./scripts/install-skills.sh --into "$SCRATCH" --remove
ls -l "$SCRATCH"
rm -rf "$SCRATCH"
```

Expected: first `ls` shows 3 symlinks (`hearth-today-brief`, `hearth-project-scan`, `hearth-memo-organize`) pointing to the repo's `skills/<name>/` dirs. Second `ls` shows an empty dir.

- [ ] **Step 4: Confirm git tree is clean**

```bash
git status
git log --oneline -10
```

Expected: working tree clean (README.md/docs/hearth-cli-ko.md pre-existing deltas from earlier sessions are fine — they predate this plan). Last ~6 commits are the ones this plan created.

- [ ] **Step 5: Announce completion**

No commit here; just report: "Skills v1 shipped. Run `./scripts/smoke-skills.sh` to verify or `./scripts/install-skills.sh --into <dir>` to install."

---

## What this plan does NOT do (committed to follow-up sub-projects)

All from spec §9. These remain deferred; do not attempt in this plan:

1. Auto-deploy sub-project: Homebrew tap, skill registry publish, `hearth skills …` CLI subcommand.
2. Skills v2: `hearth-search-digest`, `hearth-overdue-triage`, `hearth-weekly-retro`, and whatever dogfooding surfaces.
3. Codex cron / CC hook scheduling templates (shipped examples).
4. Agent-level eval harness (skill-invocation accuracy).

---

## Appendix: quick reference for executors

**What the `hearth` binary expects:**
- `HEARTH_DB` env var for the DB file path (fallback is the app's default data dir).
- Stdout is strict JSON: `{"ok": true, "data": ...}` on success, `{"ok": false, "error": "...", "hint": "..."}` on failure.
- Exit codes: 0 success, 1 user error, 2 DB error, 64 usage.

**Building the binary** (required before smoke test):

```bash
cd src-tauri && cargo build --release -p hearth-cli
```

**File-touching discipline:**
- Tasks 1–4 only add new files under `skills/`.
- Task 5 only adds `scripts/install-skills.sh` + `scripts/tests/test_install_skills.sh`.
- Task 6 only adds `scripts/smoke-skills.sh`.
- Task 7 modifies exactly three existing files: `CHANGELOG.md`, `README.md`, and the spec. No Rust code, no JSX, no Cargo.toml.

Nothing in this plan touches production app code or the CLI itself. If a task asks you to edit `src-tauri/**`, stop — something went off-plan.
