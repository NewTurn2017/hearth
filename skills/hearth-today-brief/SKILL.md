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
2. 두 번째 문장: `p0_projects[]` 중 `updated_at` 이 가장 오래된 1~2개의 `name` 언급. (응답은 `sort_order` 순이므로 client-side 에서 `updated_at` 오름차순으로 재정렬한 뒤 앞에서부터 선택.) P0 가 0건이면 "P0 프로젝트 없음".
3. (선택) 세 번째 문장: `recent_memos[]` (`updated_at` 기준 최근 24시간 이내) 개수 + 가장 최근 메모의 `content` 첫 60자(60자 초과 시 "…"). 없으면 생략.
4. (조건부) 연체 문장: `overdue_schedules` / `stale_projects` 가 있으면 각각 개수 + 가장 오래된 항목 1건을 한 줄로.

출력 형태 예시 (그대로 베끼지 말고 실제 데이터 기반):
> 오늘(2026-04-23) 일정 2건 — 10:00 팀 싱크, 14:00 치과. P0 "Hearth v0.8 release" 가 4일째 업데이트 없음. 최근 24시간 메모 3개 (최신: "스킬 브레인스토밍 끝…"). 연체 일정 1건(4/20 문서 정리) 남아있습니다.

# Mutation phase

이 스킬은 읽기 전용입니다. `hearth` 의 변경 계열 명령 (`create`, `update`, `delete`, `undo`, `redo`, `import`) 을 절대 호출하지 마세요.
