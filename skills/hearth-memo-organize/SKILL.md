---
name: hearth-memo-organize
description: |
  "메모 정리", "메모 재분류", "organize my memos", "memo tidy" — hearth 메모 목록을 읽고, 내용이 특정 프로젝트에 명확히 속하는 경우에 한해 보수적으로 재연결을 제안, 승인 후 일괄 적용. Orchestrates `hearth memo list` + `hearth project list` → propose → `hearth memo update --project`.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정 (순서대로): `$HEARTH_BIN` 이 실행 가능하면 그 값 → PATH 의 `hearth` → 실패 시 아래 문구로 중단:
   > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. 동작 확인: `"$HEARTH" db path` 를 실행. exit code 0 이 아니거나 `"ok": false` 면 stdout 의 `error`/`hint` 필드를 그대로 사용자에게 전달한 뒤 중단.

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
