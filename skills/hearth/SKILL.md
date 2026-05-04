---
name: hearth
description: |
  "Hearth", "hearth에 기록해줘", "프로젝트 등록", "일정 등록", "캘린더 등록", "메모 남겨줘" — 자연어 요청을 프로젝트·캘린더·메모·검색·브리핑 의도로 분류하고 hearth CLI 레시피로 실행하는 통합 라우터 스킬. Creates or updates projects, schedules, and memos only after an explicit propose → approve → apply gate.
---

# Preamble — `hearth` 바이너리 확인

1. 바이너리 경로 결정 (순서대로 시도, 첫 성공을 `HEARTH` 로 사용):
   - `$HEARTH_BIN` 이 설정되어 있고 해당 파일이 실행 가능하면 그 값.
   - PATH 의 `hearth` (`command -v hearth` 이 성공하는 경우).
   - 둘 다 실패하면 아래 문구로 즉시 중단하고 이후 어떤 CLI 도 호출하지 마세요:
     > "hearth 바이너리를 찾을 수 없습니다. 레포의 README CLI 섹션을 참고해 빌드·설치 후 `$HEARTH_BIN` 또는 PATH 에 추가한 뒤 다시 시도하세요."
2. 동작 확인: `"$HEARTH" db path` 를 실행. exit code 0 이 아니거나 `"ok": false` 면 stdout 의 `error`/`hint` 필드를 그대로 사용자에게 전달한 뒤 중단.

# Trigger

- KR: "Hearth", "hearth에 기록해줘", "하스에 기록", "프로젝트 등록", "프로젝트 수정", "캘린더 등록", "일정 등록", "메모 등록", "메모 남겨줘", "검색해줘", "오늘 뭐해"
- EN: "hearth", "add this to Hearth", "create project", "update project", "add calendar event", "add schedule", "create memo", "search Hearth", "today brief"
- Use this skill as the default Hearth entrypoint when the user asks for project, schedule/calendar, memo, search, today, overdue, or light organization work and did not explicitly request a narrower Hearth skill.

# Intent routing

사용자 요청을 아래 의도 중 하나로 분류하세요. 둘 이상이 섞여 있으면 한 번의 제안에 여러 작업을 묶되, 각 작업의 CLI 명령을 분리해서 보여주세요.

- `project.create`
  - Read phase: `"$HEARTH" project list`; 필요하면 `"$HEARTH" search "<name>" --scope project --limit 5`
  - Mutation recipe: `"$HEARTH" project create "<name>" --priority <P0-P4> [--category "<category>"] [--path "<path>"] [--evaluation "<memo>"]`
- `project.update`
  - Read phase: `"$HEARTH" project list`; 후보가 여러 개면 `"$HEARTH" search "<query>" --scope project --limit 10`; 선택한 대상은 `"$HEARTH" project get <id>`
  - Mutation recipe: `"$HEARTH" project update <id> [--name "<name>"] [--priority <P0-P4>] [--category "<category>"] [--path "<path>"] [--evaluation "<memo>"]`
- `schedule.create`
  - Read phase: 날짜가 상대 표현이면 현재 날짜 기준으로 YYYY-MM-DD 로 해석. 기존 중복 가능성은 `"$HEARTH" schedule list --from <date> --to <date>` 로 확인
  - Mutation recipe: `"$HEARTH" schedule create <YYYY-MM-DD> [--time HH:MM] [--description "<title>"] [--location "<place>"] [--notes "<notes>"] [--remind-5min] [--remind-start]`
- `schedule.update`
  - Read phase: `"$HEARTH" schedule list --from <start> --to <end>` 또는 `"$HEARTH" search "<query>" --scope schedule --limit 10`; 선택한 대상은 `"$HEARTH" schedule get <id>`
  - Mutation recipe: `"$HEARTH" schedule update <id> [--date YYYY-MM-DD] [--time HH:MM] [--description "<title>"] [--location "<place>"] [--notes "<notes>"] [--remind-5min true|false] [--remind-start true|false]`
- `memo.create`
  - Read phase: 프로젝트 연결 가능성이 있으면 `"$HEARTH" project list`; 특정 프로젝트명이 있으면 `"$HEARTH" search "<project>" --scope project --limit 5`
  - Mutation recipe: `"$HEARTH" memo create "<content>" [--color yellow|blue|green|pink|purple] [--project <project.id>]`
- `memo.update`
  - Read phase: `"$HEARTH" memo list` 또는 `"$HEARTH" search "<query>" --scope memo --limit 10`; 선택한 대상은 `"$HEARTH" memo get <id>`
  - Mutation recipe: `"$HEARTH" memo update <id> [--content "<content>"] [--color yellow|blue|green|pink|purple] [--project <project.id> | --detach]`
- `project.scan`
  - Read phase: `"$HEARTH" project scan "<dir>"` 후 `already_registered == false` 인 후보만 남기고 `"$HEARTH" project list` 로 중복 경로를 재확인
  - Mutation recipe: 승인된 후보마다 `"$HEARTH" project create "<name>" --priority <P0-P4> --path "<path>"`
- `memo.organize`
  - Read phase: `"$HEARTH" memo list` + `"$HEARTH" project list`
  - Mutation recipe: 명확히 하나의 프로젝트에만 매칭되는 메모에 한해 `"$HEARTH" memo update <memo.id> --project <project.id>`
- `search`
  - Read-only recipe: `"$HEARTH" search "<query>" [--scope project,memo,schedule] [--limit N]`
- `today`
  - Read-only recipe: `"$HEARTH" today` + 필요하면 `"$HEARTH" overdue`

# Clarification rules

1. 날짜는 반드시 `YYYY-MM-DD` 로 확정한 뒤 실행합니다. "오늘", "내일", "다음 주 월요일" 같은 상대 표현은 현재 세션 날짜와 타임존 기준으로 계산하고, 제안에 계산 결과를 표시하세요.
2. 시간이 없으면 일정은 종일 일정으로 등록합니다. 사용자가 시간 맥락을 암시했지만 불명확하면 딱 한 번 물어보세요.
3. 프로젝트/메모/일정 수정에서 대상 후보가 1개로 좁혀지지 않으면 mutation 하지 말고 후보 목록을 보여준 뒤 id 선택을 요청하세요.
4. 프로젝트를 생성하거나 priority 를 변경할 때, 사용자 메시지에 priority 가 명시되어 있지 않으면 Propose 직전에 반드시 아래 표를 보여주고 P0~P4 중 하나를 선택받으세요. 사용자가 같은 턴에서 "기본값으로", "알아서", "임의로" 같은 표현으로 명시적으로 위임한 경우에만 `P2` 를 기본값으로 사용할 수 있습니다. 메모 color 가 없으면 `yellow` 를 기본값으로 씁니다.

   | 값 | 의미 |
   | --- | --- |
   | P0 | 긴급 |
   | P1 | 높음 |
   | P2 | 중간 |
   | P3 | 낮음 |
   | P4 | 참고용 |
5. 사용자 요청에 프로젝트명이 포함된 메모라도, 매칭되는 프로젝트가 정확히 1개가 아닐 때는 `--project` 없이 메모를 만들거나 연결 여부를 질문하세요. 프로젝트를 추측해서 연결하지 마세요.
6. 사용자 메시지에 "바로", "그냥 해", "기록해"가 있어도 변경 계열 명령은 아래 Propose 단계를 거칩니다. 단, 사용자가 같은 대화 턴에서 특정 명령과 인자를 명확히 승인한 경우에는 추가 질문 없이 적용할 수 있습니다.

# Propose — 변경 전 사용자 확인

변경 계열 의도(`create`, `update`, `delete`, `link-path`, `import`, `undo`, `redo`)를 실행하기 전에는 다음 형식으로 제안하세요.

```text
이렇게 Hearth에 반영하겠습니다.
1. <짧은 설명>
   명령: hearth ...

진행할까요? `진행`, `수정`, `취소` 중 하나로 답해주세요.
```

- 여러 작업이면 번호별로 명령을 모두 표시합니다.
- `수정`이면 사용자의 수정 내용을 반영해 Propose 를 다시 합니다.
- `취소` 또는 모호한 침묵이면 어떤 mutation 도 호출하지 않습니다.

# Mutation phase — 승인 후 실행

1. 제안한 명령을 순서대로 실행합니다.
2. 각 호출 후 응답의 `ok == true` 확인. 첫 실패에서 중단하고 stdout 의 `error`/`hint` 를 그대로 전달하세요.
3. 모두 성공하면 생성/수정된 id 와 핵심 필드를 한 줄로 요약합니다.
4. 변경 건수가 1건 이상이면 마지막 줄로 반드시 출력:
   > "되돌리려면 `hearth undo M` 를 실행하세요 (M = 방금 반영한 변경 건수)."

# Read-only responses

- `search`: 결과를 프로젝트/메모/일정별로 묶어 최대 10개까지만 요약합니다. id, 제목/내용 앞부분, 날짜 또는 priority 를 포함하세요.
- `today`: 한국어 3~5문장 브리핑을 작성하세요. `"$HEARTH" today` 를 먼저 실행하고, 연체 확인이 필요하면 `"$HEARTH" overdue` 를 추가 실행합니다.
- 사용자가 "수정하지 말고", "조회만", "설명만"이라고 말하면 절대 mutation 명령을 호출하지 마세요.

# Single-skill policy

Hearth는 하나의 agent skill 로 노출됩니다. 별도의 `hearth-today-brief`, `hearth-project-scan`, `hearth-memo-organize` 스킬을 찾거나 호출하지 말고, 위 intent routing 안에서 직접 처리하세요.
