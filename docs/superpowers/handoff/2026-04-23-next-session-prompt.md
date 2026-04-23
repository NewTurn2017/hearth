# Next Session Prompt — Agent Skills sub-project

Paste the block below into a fresh Claude Code session started at the repo root (`/Users/genie/dev/tools/hearth`). The session should pick up where this one left off.

---

```
앞선 세션에서 Hearth CLI 구현을 완료했다 (branch `claude/pedantic-merkle-b0a5f1`, 31 commits, 88 tests). 이제 parent goal 의 두 번째 sub-project 인 **Agent Skills** 를 브레인스토밍 → 스펙 → 계획 → 구현으로 진행한다.

## 컨텍스트 먼저 읽기

1. **구현 완료 정리** — `docs/superpowers/handoff/2026-04-23-hearth-cli-complete.md`
   어떤 CLI 가 있고, 테스트 상태가 어떻고, 뭐가 deferred 됐는지 여기 다 있다.

2. **CLI 스펙** — `docs/superpowers/specs/2026-04-22-hearth-cli-design.md`

3. **CLI 계획** — `docs/superpowers/plans/2026-04-22-hearth-cli.md` (5.5k 줄, 실행 완료)

4. **부모 목표 맥락 (원문)**
   > 별도 워크트리를 폴더 내부에 두어 개발한다. 기능: cli 개발 및 전용 스킬(skills) 생성. 목적: cli를 통해 모든 기능을 통제할수 있음. db 자유 변경, 검색까지. cli 개발후 이것을 이용한 스킬 개발 (스킬을 통해 자유롭게 codex, claude code 와 같은 에이전트에서 마음데로 hearth 에 로컬 폴더 정리 및 메모 정리등이 가능해짐). 현재 모든 기능을 파악해서 적용가능한 다양한 아이디어 산출 필요함. 또한 최종 목표는 cli, skill, 선택 자동배포.

5. **이번 sub-project 범위**
   - CLI 는 완료 (`hearth` 바이너리 정상 동작)
   - 이번 세션의 출발점: CLI 를 감싸는 agent skills 세트를 브레인스토밍
   - 자동 배포 (Homebrew tap + skill registry) 는 그 다음 sub-project — 이 세션 범위 밖

## 진행 방식

- `superpowers:using-superpowers` 스킬부터 invoke
- 그 다음 `superpowers:brainstorming` 으로 Agent Skills sub-project 브레인스토밍 시작
- CLI 가 이미 존재한다는 전제 위에 논의 — 이미 만들어진 게 아니라 새로 만드는 식으로 접근하면 중복 질문 발생
- 스킬 아이디어는 `docs/superpowers/ideas-backlog.md` 참고 + CLI 의 composite 뷰 (`today`, `overdue`, `stats`, `search`, `log`) 에 대응되는 사용 시나리오 중심으로
- brainstorm 결과를 `docs/superpowers/specs/YYYY-MM-DD-hearth-skills-design.md` 에 저장
- plan 은 `docs/superpowers/plans/YYYY-MM-DD-hearth-skills.md`

## 참고 — 이 sub-project 에서 다룰 스킬 후보 (브레인스토밍 단계에서 함께 발굴/가감)

- `hearth-today-brief` — 매일 아침 `hearth today` 실행 → 자연어 요약으로 변환
- `hearth-memo-organize` — 메모 목록을 읽고 색상/프로젝트 재분류 제안, 승인 후 일괄 적용
- `hearth-project-scan` — 로컬 디렉토리를 `hearth project scan` 한 뒤 사용자가 보기 좋은 리포트 + 미등록 폴더 선택적 등록
- `hearth-search-digest` — 특정 키워드로 `hearth search` → 결과를 맥락 있게 요약
- `hearth-overdue-triage` — `hearth overdue` 로 방치 프로젝트/일정을 가져와 우선순위 리스트업 + 액션 제안
- `hearth-weekly-retro` — 지난 7일 audit_log (`hearth log --limit 500`) 를 읽고 "이번 주 한 일" 회고 메모 자동 생성

이것들은 시작 아이디어일 뿐. 브레인스토밍에서 사용자와 스킬 범위/트리거/MCP 통합 여부 등을 결정하자.

## 환경

- 메인 레포: `/Users/genie/dev/tools/hearth`
- 앞선 작업 브랜치: `claude/pedantic-merkle-b0a5f1` (아직 main 에 merge 안 됨)
- 이번 세션을 위한 새 워크트리를 만들지는 brainstorm 중 사용자와 결정 (superpowers:using-git-worktrees 스킬 참고)

## 바로 시작하는 첫 질문

앞선 세션의 CLI 완료 정리를 읽고, 사용자에게 첫 clarifying question 으로 던질 것:

> Skills 의 주된 사용자는 (a) 사용자 본인이 Claude Code 에서 직접 호출, (b) Codex/다른 agent 가 배경으로 주기적 실행, (c) 둘 다 — 어느 시나리오를 주로 노릴까요? 이게 스킬 UX 의 tone 을 결정합니다.

그 뒤로는 brainstorming 스킬의 표준 플로우 따라 진행.
```

---

## 새 세션 시작 전 체크리스트 (이번 세션에서 처리)

- [x] CLI 구현 완료 (commit `05efcd6` 까지)
- [x] 완료 정리 문서 생성 (`docs/superpowers/handoff/2026-04-23-hearth-cli-complete.md`)
- [x] 다음 세션 프롬프트 생성 (이 파일)
- [ ] 두 문서 commit + push (새 세션이 시작하기 전에 main 또는 같은 브랜치에서 pull 가능해야 함)

새 세션에서는 worktree 를 새로 만들지 기존 worktree 에서 계속할지 사용자와 결정 — 두 가지 모두 합리적.
- 새 worktree: clean isolation, 하지만 `target/` 재빌드 비용
- 기존 worktree 계속: 빌드 캐시 재사용, 하지만 CLI PR 완료 전에 Skills 이 얹히는 문제
