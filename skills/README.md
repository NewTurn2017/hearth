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
