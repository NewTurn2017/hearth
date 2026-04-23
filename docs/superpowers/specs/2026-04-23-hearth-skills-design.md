# Hearth Agent Skills — Design Spec (v1)

**Date:** 2026-04-23
**Parent goal:** Agent-driven hearth control — CLI (done) → **Skills (this)** → Auto-deploy (next)
**Prereq:** `hearth` CLI from branch `claude/pedantic-merkle-b0a5f1` is installed on PATH (or `$HEARTH_BIN`)

## 1. Motivation

CLI is the universal substrate; Skills are the *agent-facing layer* that makes hearth operable from any agent host (Claude Code, Codex, 기타 Skill-tool 지원 에이전트) without the user typing CLI flags. A skill bundles: trigger description, a pinned sequence of CLI recipes, natural-language synthesis, and — for mutating skills — an explicit `propose → user approval → apply` gate.

## 2. Goals & Non-Goals

### Goals
- Ship **3 v1 skills** that cover distinct interaction patterns:
  1. **hearth-today-brief** — read-only briefing
  2. **hearth-project-scan** — read-heavy, write-on-approval folder-to-project registration
  3. **hearth-memo-organize** — read + propose reclassification + apply-on-approval
- Work **portably** across CC, Codex, and any agent that loads standard `SKILL.md` (YAML frontmatter + Markdown body).
- Keep each SKILL.md **self-contained** (no helper scripts, no external runtimes) so it survives any agent host.
- Provide a **manual install path** (`scripts/install-skills.sh`) usable today while auto-deploy is designed as a separate sub-project.
- Provide a **smoke test script** that exercises the CLI recipes each skill depends on.

### Non-Goals (deferred to later sub-projects — must ship eventually)
- Automated distribution (Homebrew tap, skill registry publish).
- A `hearth skills …` CLI subcommand.
- The other four candidate skills from the ideas list: `hearth-search-digest`, `hearth-overdue-triage`, `hearth-weekly-retro`, and any further expansions.
- Codex cron / CC hook auto-scheduling. The skill description may *mention* a recommended cadence; actual scheduling is the agent host's domain.
- Formal agent-level evals (skill-invocation accuracy studies).

## 3. Core Decisions

| Decision | Value | Why |
|---|---|---|
| Agent portability | Standard `SKILL.md` (name + description frontmatter, Markdown body); no CC-specific features | Codex + other hosts must load the same file |
| Orchestration depth | **Medium** — skill composes 2+ CLI calls, summarizes, proposes next actions | Thin wrappers don't justify existing; thick logic drifts across model strengths |
| CLI invocation | **Pinned recipes** — SKILL.md contains exact commands to run, not hints | Prevents the agent from inventing wrong flags |
| Mutation UX | **Always 2-step: propose → explicit user approval → apply** | Uniform contract across CC interactive and Codex background; builds trust in v1 |
| Binary discovery | `$HEARTH_BIN` env → `hearth` on PATH → abort with install hint | Simple, explicit, no fragile path scanning |
| Language | Korean primary in body + user-facing output; trigger description bilingual (KR + EN) | Matches repo tone; covers host discovery across languages |
| Error surfacing | CLI's `{"ok":false,"error","hint"}` forwarded verbatim to user | CLI already owns error wording; no retranslation drift |
| Install | Manual via `scripts/install-skills.sh --into <path>` requiring explicit target dir | Avoid silent writes into unverified system dirs |
| Tests | `scripts/smoke-skills.sh` runs each skill's CLI recipes on a fixture DB | Agent-level skill tests are out of scope for v1 |

## 4. Repository Layout

```
skills/
├── README.md                       # PATH prereq, install instructions, troubleshooting
├── hearth-today-brief/
│   └── SKILL.md
├── hearth-project-scan/
│   └── SKILL.md
└── hearth-memo-organize/
    └── SKILL.md
scripts/
├── install-skills.sh               # symlinks skills/* into user-specified dir
└── smoke-skills.sh                 # regression: exercises each skill's CLI recipes
docs/superpowers/
├── specs/2026-04-23-hearth-skills-design.md   # this file
└── plans/2026-04-23-hearth-skills.md          # (will be created by writing-plans skill)
```

All skill folders sit at repo root under `skills/`. GitHub tree view shows them as a first-class product artifact, and the flat `skills/<name>/SKILL.md` convention matches the broader agent-skill ecosystem.

## 5. SKILL.md Standard Template

Every skill follows this structure. Sections marked *(mutation-only)* are omitted for read-only skills.

```markdown
---
name: <kebab-case-skill-name>
description: |
  <KR 한 줄 트리거 설명> / <EN one-line trigger description>.
  Orchestrates <which CLI commands> to <what outcome>.
---

# Preamble — `hearth` 바이너리 확인
1. Resolve binary path: prefer `$HEARTH_BIN`; else `hearth` on PATH; else abort with:
   "hearth 바이너리를 찾을 수 없습니다. https://github.com/.../hearth 참조해 설치 후 다시 시도하세요."
2. Confirm CLI is usable: `"$HEARTH" db path` (exit 0 ⇒ continue; non-zero ⇒ surface `error`/`hint`).

# Trigger
- KR: <예시 문구 3~5개>
- EN: <3–5 example phrases>
- Recommended cadence (if any): e.g. "daily 07:00 via Codex cron"

# Read phase
(ordered CLI recipes — exact commands and what to extract)
1. `"$HEARTH" <subcmd> [flags]` → parse `data.<field>`
2. `"$HEARTH" <subcmd> [flags]` → parse `data.<field>`

# Synthesis
한국어 N~M문장으로 아래를 요약:
- <항목 1>
- <항목 2>
(예시 출력 한 블록 포함)

# Mutation phase  *(mutation-only)*
1. Build a human-readable plan from the read-phase data.
2. Present the plan + "진행할까요?" question. Do NOT run mutations yet.
3. Wait for explicit user approval. Anything ambiguous ⇒ ask again.
4. On approval, run mutation CLI commands one at a time:
   - Check each exit code / `ok` field.
   - On the first failure: stop, report which succeeded and which remain, and surface the failing CLI's `error`/`hint`.
5. Close with `"$HEARTH" log --limit <N>` and the line:
   "되돌리려면 `hearth undo` 를 실행하세요."
```

### Frontmatter notes
- `name` must be kebab-case and prefixed `hearth-` (collision avoidance in flat skill dirs).
- `description` is the sole field the host matches against — it must carry KR + EN triggers and say *what CLI flow it runs* (agents rank descriptions by specificity).

## 6. Per-Skill Specification

### 6.1 `hearth-today-brief` — read-only briefing

| Field | Value |
|---|---|
| Triggers | "오늘 뭐해", "오늘 브리핑", "agenda", "today brief", "morning rundown" |
| Read recipes | 1. `"$HEARTH" today` → `data.{schedules, p0_projects, recent_memos}`<br>2. `"$HEARTH" overdue` → `data.{past_schedules, stale_projects}` (only mentioned in output if non-empty) |
| Synthesis | 3–5 KR sentences: 오늘 일정 summary, P0 중 최근 터치 안 된 것, overdue가 있으면 별도 한 줄 |
| Mutation | None |
| Failure modes | DB missing → surface hint; no data at all → "오늘 일정/메모 없음" + suggestion to add one |

**Example output shape** (illustrative):
> 오늘 일정 2건 — 10:00 팀 싱크, 14:00 치과. P0 프로젝트 3개 중 "Hearth v0.8 release"가 4일째 터치 안 되고 있어요. 지난주 방치된 일정 1건(4/20 문서 정리)도 남아있습니다.

### 6.2 `hearth-project-scan` — folder → project registration

| Field | Value |
|---|---|
| Triggers | "이 폴더 스캔해서 프로젝트로", "scan this dir", "프로젝트 후보 찾아줘" |
| Preconditions | User provides a directory path; if missing, ask "어느 폴더를 스캔할까요?" once. |
| Read recipes | 1. `"$HEARTH" project scan <DIR>` → `data.candidates[]` (each has `{name, path, has_git, size_mb}`)<br>2. `"$HEARTH" project list` → dedupe against existing `data.projects[].linked_path` |
| Propose | Table of *unregistered* candidates with suggested `name` + default `priority=P2`. Ask user which to register (default: none — user must pick explicitly). |
| Mutation | For each chosen candidate: `"$HEARTH" project create "<name>" --priority <P> --linked-path "<path>"`. Verify `ok:true` per call; stop on first failure. |
| Close | `"$HEARTH" log --limit <N>` + undo hint. |

### 6.3 `hearth-memo-organize` — reclassify memos

| Field | Value |
|---|---|
| Triggers | "메모 정리해줘", "메모 재분류", "organize my memos", "memo tidy" |
| Read recipes | 1. `"$HEARTH" memo list --limit 200` → `data.memos[]` (each has `{id, content, color, project_id}`)<br>2. `"$HEARTH" project list` → project names + colors for mapping reference |
| Propose | Build a reassignment table: for memos whose content clearly belongs to an existing project (keyword match against project name/aliases) or whose color contradicts the linked project's color, suggest a change. Skip ambiguous cases by default. |
| Mutation | For each confirmed reassignment: `"$HEARTH" memo update <id> [--color <c>] [--project <id>]`. Per-call ok check; stop on first failure. |
| Close | `"$HEARTH" log --limit <N>` + undo hint. |

### 6.4 Shared output style
- Korean primary; English allowed inside code blocks/quoted CLI output.
- No emoji in skill body or user output (repo convention).
- Tables (Markdown) for multi-item proposals; prose for briefings.
- Always surface CLI's `error` + `hint` fields verbatim on failure.

## 7. Install & Runtime Contract

### Prereq
`hearth` binary built from this branch's workspace (see handoff doc). Must be either on PATH or pointed at by `$HEARTH_BIN`. Skills do NOT bundle the binary.

### `scripts/install-skills.sh`
Responsibilities:
1. Requires an explicit `--into <path>` argument (no silent defaulting into `~/.claude/...`).
2. Creates the target dir if missing; refuses to overwrite non-symlink files already there (errors with a hint).
3. For each `skills/<name>/`, creates a symlink at `<path>/<name>` pointing at the repo's folder (absolute path).
4. Supports `--remove` to delete only those symlinks it would create (no wildcard removal).
5. Prints a summary at the end: which skills linked, where, how to verify.

Environment overrides (detected if `--into` not given — but script still requires confirmation):
- `CLAUDE_SKILLS_DIR` — suggested default for CC.
- `CODEX_SKILLS_DIR` — suggested default for Codex.

Explicit `--into` always wins over env. Env presence only changes the suggested value in the error message when `--into` is missing.

### `scripts/smoke-skills.sh`
1. `export HEARTH_DB=$(mktemp -d)/smoke.db`
2. `"$HEARTH" db migrate`
3. Seed: 3 projects (one P0), 5 memos with mixed colors, 2 schedules (one past → overdue).
4. Execute each skill's read-phase CLI recipes; assert exit 0 and `ok:true`.
5. Execute project-scan mutation against a tmp dir containing 2 fake project folders; assert 2 new `project create` succeed; then `project list | jq '.data | length'` equals initial + 2.
6. Execute memo-organize mutation against a planned reassignment of 1 memo; verify `memo get` reflects change.
7. `hearth undo` once; verify the last mutation reverted.
8. Cleanup tmp dirs. Exit 0 on full pass.

### Failure handling in skills
- `hearth` not found → skill aborts in Preamble with install hint, no CLI calls attempted.
- `hearth db path` exit non-zero → surface CLI's error + hint; do not proceed.
- Mutation batch: stop on first failure; tell user "N of M applied, remainder canceled"; show undo hint.

## 8. v1 Deliverables Checklist

- [ ] `skills/README.md` — prereq + install + troubleshooting
- [ ] `skills/hearth-today-brief/SKILL.md`
- [ ] `skills/hearth-project-scan/SKILL.md`
- [ ] `skills/hearth-memo-organize/SKILL.md`
- [ ] `scripts/install-skills.sh` (with `--into`, `--remove`, env hints)
- [ ] `scripts/smoke-skills.sh` (covers read + one mutation + undo per skill)
- [ ] This spec committed
- [ ] Implementation plan (next step, writing-plans skill)

## 9. Out-of-Scope Note (explicit)

These items **are needed eventually** and are committed to follow-up sub-projects:

1. **Auto-deploy sub-project**
   - Homebrew tap for `hearth` binary
   - Skill registry publish (CC plugin marketplace + Codex skill registry once they exist/stabilize)
   - `hearth skills list|install|uninstall` CLI subcommand
2. **Skills v2 expansion**
   - `hearth-search-digest`
   - `hearth-overdue-triage`
   - `hearth-weekly-retro`
   - Any new skills surfaced by v1 dogfooding (skill ideas will live in `docs/superpowers/ideas-backlog.md`).
3. **Codex cron / CC hook scheduling templates** — shipped examples (e.g. daily 07:00 today-brief).
4. **Agent-level eval harness** — automated measurement of skill-invocation accuracy across hosts.

## 10. Open Questions (none blocking v1)

- Does `hearth project scan` currently emit `has_git` and `size_mb` fields on candidates? If only `{name, path}` exists, the skill simply shows `name` + `path` — no design change needed; plan will pin the exact field list after checking `hearth-core/src/scan.rs`.
- Color-based reassignment heuristic in `memo-organize` is subjective; plan will define a conservative rule (e.g. only propose when project has an explicit color AND memo color differs AND memo content contains the project's name token) and leave broader taste-based suggestions for v2.
