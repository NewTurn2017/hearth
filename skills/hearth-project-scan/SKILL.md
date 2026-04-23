---
name: hearth-project-scan
description: |
  "이 폴더 스캔", "scan this dir", "프로젝트 후보 찾아줘" — 지정한 디렉토리의 하위 폴더 중 아직 hearth 프로젝트로 등록되지 않은 것을 찾아내고, 사용자 승인 후에만 일괄 등록. Orchestrates `hearth project scan` + `hearth project create` with an explicit propose → approve → apply gate.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정 (순서대로): `$HEARTH_BIN` 이 실행 가능하면 그 값 → PATH 의 `hearth` → 실패 시 아래 문구로 중단:
   > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. 동작 확인: `"$HEARTH" db path` 를 실행. exit code 0 이 아니거나 `"ok": false` 면 stdout 의 `error`/`hint` 필드를 그대로 사용자에게 전달한 뒤 중단.

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
