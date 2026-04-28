# Hearth CLI 한글 설명서

`hearth` CLI는 Hearth 앱의 SQLite DB를 터미널에서 직접 읽고 쓰는 독립 실행형 도구입니다. 프로젝트, 메모, 일정, 카테고리, 검색, 통계, 감사 로그, 가져오기/내보내기를 모두 명령어로 다룰 수 있습니다. 실행 중인 Hearth 앱은 CLI가 DB를 바꾸면 0.5~1초 안에 변경 사항을 자동 반영합니다.

이 문서는 현재 구현된 `hearth-cli` 기준의 사용 설명서입니다.

## 빠른 시작

릴리즈 바이너리를 빌드합니다.

```bash
cd src-tauri
cargo build --release -p hearth-cli
```

바이너리는 다음 위치에 생성됩니다.

```bash
src-tauri/target/release/hearth
```

저장소 루트에서 실행한다면 아래처럼 호출할 수 있습니다.

```bash
./src-tauri/target/release/hearth --help
./src-tauri/target/release/hearth db path
./src-tauri/target/release/hearth project list
```

자주 쓴다면 셸 alias를 두면 편합니다.

```bash
alias hearth="$PWD/src-tauri/target/release/hearth"
```

임시 DB로 안전하게 시험하려면 `HEARTH_DB`를 지정합니다.

```bash
HEARTH_DB=/tmp/hearth-smoke.db hearth db migrate
HEARTH_DB=/tmp/hearth-smoke.db hearth project create "CLI 테스트" --priority P0
HEARTH_DB=/tmp/hearth-smoke.db hearth memo create "CLI에서 만든 메모" --color blue
HEARTH_DB=/tmp/hearth-smoke.db hearth today
```

## 기본 구조

Hearth는 Tauri 앱과 CLI가 같은 `hearth-core` 로직을 공유합니다.

```text
src-tauri/
├── core/  공통 도메인 로직, DB 마이그레이션, 검색, 감사 로그
├── app/   Tauri 데스크톱 앱
└── cli/   독립 실행형 hearth 바이너리
```

CLI는 앱 프로세스에 명령을 보내지 않습니다. 같은 SQLite DB를 열어서 `hearth-core` 함수를 직접 호출합니다. 그래서 앱이 꺼져 있어도 CLI만으로 데이터를 만들고 수정할 수 있고, 앱이 켜져 있으면 `PRAGMA data_version` 감지로 화면이 자동 갱신됩니다.

## 공통 옵션

모든 명령에서 다음 옵션을 사용할 수 있습니다.

| 옵션              | 설명                                                   |
| ----------------- | ------------------------------------------------------ |
| `--db <PATH>`     | 사용할 SQLite DB 경로를 직접 지정합니다.               |
| `-v`, `--verbose` | 디버그 로그를 stderr로 출력합니다.                     |
| `--pretty`        | 일부 list 명령을 사람이 읽기 쉬운 테이블로 출력합니다. |
| `-h`, `--help`    | 도움말을 출력합니다.                                   |
| `-V`, `--version` | CLI 버전을 출력합니다.                                 |

DB 경로 우선순위는 다음과 같습니다.

1. `--db <PATH>`
2. `HEARTH_DB` 환경변수
3. macOS 기본 앱 DB: `~/Library/Application Support/com.codewithgenie.hearth/data.db` (이전 `com.newturn2017.hearth/`는 첫 실행 시 자동 이관)

현재 CLI의 자동 기본 경로 해석은 macOS 기준입니다. Windows/Linux에서 테스트할 때는 `--db` 또는 `HEARTH_DB`를 명시하는 편이 안전합니다.

## 출력 규칙

기본 출력은 JSON envelope입니다.

```json
{"ok":true,"data":{...}}
```

처리된 사용자 오류는 stderr에 JSON으로 출력됩니다.

```json
{
  "ok": false,
  "error": "project 999 not found",
  "hint": "try 'hearth project list'"
}
```

예외가 하나 있습니다. `hearth export`를 `--out` 없이 실행하면 파이프에 바로 넘기기 좋도록 envelope 없이 원본 dump JSON을 stdout으로 출력합니다.

```bash
hearth export | jq .
```

`--pretty`는 현재 `project list`, `memo list`, `schedule list`에서 테이블 출력을 제공합니다. 자동화나 에이전트가 파싱할 때는 기본 JSON 출력을 쓰는 것이 안정적입니다.

## 명령 요약

| 명령                             | 용도                                                        |
| -------------------------------- | ----------------------------------------------------------- |
| `hearth db path`                 | 실제로 열 DB 경로를 확인합니다.                             |
| `hearth db migrate`              | DB를 열고 마이그레이션을 적용합니다.                        |
| `hearth db vacuum`               | `VACUUM`과 `PRAGMA integrity_check`를 실행합니다.           |
| `hearth project ...`             | 프로젝트 목록, 생성, 조회, 수정, 삭제, 폴더 스캔, 경로 연결 |
| `hearth memo ...`                | 메모 목록, 생성, 조회, 수정, 삭제                           |
| `hearth schedule ...`            | 일정 목록, 생성, 조회, 수정, 삭제                           |
| `hearth category ...`            | 카테고리 목록, 생성, 이름 변경, 수정, 삭제                  |
| `hearth search <QUERY>`          | 프로젝트, 메모, 일정 전체 FTS5 검색                         |
| `hearth today`                   | 오늘 일정, P0 프로젝트, 최근 메모 요약                      |
| `hearth overdue`                 | 지난 일정과 오래 방치된 프로젝트 요약                       |
| `hearth stats`                   | 전체 개수와 우선순위, 카테고리, 색상 분포                   |
| `hearth log show`                | 감사 로그 조회                                              |
| `hearth log undo`, `hearth undo` | 최근 변경 되돌리기                                          |
| `hearth log redo`, `hearth redo` | 되돌린 변경 다시 적용                                       |
| `hearth export`                  | JSON 또는 SQLite 파일로 내보내기                            |
| `hearth import`                  | JSON export 파일 가져오기                                   |

## 프로젝트

프로젝트 목록을 봅니다.

```bash
hearth project list
hearth --pretty project list
hearth project list --priority P0,P1
hearth project list --category Active,Tools
```

프로젝트를 생성합니다. `--priority` 기본값은 `P2`입니다.

```bash
hearth project create "영상 자동화 파이프라인" --priority P0 --category Lab
hearth project create "문서 정리" --path "/Users/genie/dev/docs" --evaluation "다음 릴리즈 전 정리"
```

프로젝트를 조회, 수정, 삭제합니다.

```bash
hearth project get 1
hearth project update 1 --name "영상 자동화 MVP" --priority P1
hearth project delete 1
```

폴더를 스캔해서 프로젝트 후보를 찾습니다.

```bash
hearth project scan "/Users/genie/dev" --depth 2
```

기존 프로젝트에 실제 파일시스템 경로를 연결합니다. 경로가 존재하는 디렉터리인지 확인한 뒤 저장합니다.

```bash
hearth project link-path 3 "/Users/genie/dev/tools/hearth"
```

## 메모

메모 목록을 봅니다.

```bash
hearth memo list
hearth --pretty memo list
```

메모를 생성합니다. 색상 기본값은 `yellow`입니다.

```bash
hearth memo create "릴리즈 전에 CLI 문서 확인"
hearth memo create "앱 화면에서 바로 반영되는지 확인" --color blue
hearth memo create "이 메모는 프로젝트 3에 연결" --project 3
```

메모를 수정하거나 프로젝트 연결을 바꿉니다.

```bash
hearth memo update 5 --content "문서 예시까지 확인 완료"
hearth memo update 5 --color green
hearth memo update 5 --project 3
hearth memo update 5 --detach
```

메모를 조회하거나 삭제합니다.

```bash
hearth memo get 5
hearth memo delete 5
```

## 일정

일정 목록을 봅니다.

```bash
hearth schedule list
hearth --pretty schedule list
hearth schedule list --month 2026-05
hearth schedule list --from 2026-05-01 --to 2026-05-31
```

일정을 생성합니다. 날짜는 positional argument입니다.

```bash
hearth schedule create 2026-05-01 --time 09:00 --description "월간 계획"
hearth schedule create 2026-05-02 --location "온라인" --notes "자료 먼저 준비"
hearth schedule create 2026-05-03 --time 14:00 --description "데모" --remind-5min --remind-start
```

일정을 수정합니다.

```bash
hearth schedule update 7 --date 2026-05-04 --time 10:30
hearth schedule update 7 --description "데모 리허설" --location "회의실"
hearth schedule update 7 --remind-5min false --remind-start true
```

일정을 조회하거나 삭제합니다.

```bash
hearth schedule get 7
hearth schedule delete 7
```

## 카테고리

카테고리를 관리합니다.

```bash
hearth category list
hearth category create Research --color "#6aa4ff"
hearth category rename Research Lab
hearth category update 2 --color "#5dd39e" --sort-order 10
hearth category delete 2
```

카테고리 이름 변경은 해당 카테고리를 쓰는 모든 프로젝트에 트랜잭션으로 반영됩니다. 사용 중인 카테고리는 삭제할 수 없고, 먼저 프로젝트의 카테고리를 다른 값으로 바꿔야 합니다.

## 검색과 요약 뷰

전체 검색은 프로젝트, 메모, 일정에 걸쳐 FTS5로 검색합니다.

```bash
hearth search "agent"
hearth search "영상 자동화" --scope project,memo
hearth search "회의" --scope schedule --limit 5
```

단일 단어 검색은 prefix 매칭을 사용합니다. 예를 들어 `agent`는 `agent*` 형태로 검색되어 `agents` 같은 단어도 잡습니다. 여러 단어 검색은 phrase 검색으로 처리됩니다.

요약 뷰는 자주 필요한 상태를 한 번에 보여줍니다.

```bash
hearth today
hearth overdue
hearth stats
```

## 감사 로그, undo, redo

프로젝트, 메모, 일정 변경은 `audit_log`에 기록됩니다. CLI에서 만든 변경은 `source=cli`로 남습니다.

```bash
hearth log show
hearth log show --limit 50
hearth log show --source cli
hearth log show --table projects
hearth log show --include-undone
```

최근 변경을 되돌립니다.

```bash
hearth log undo
hearth log undo 3
hearth undo
hearth undo 3
```

되돌린 변경을 다시 적용합니다.

```bash
hearth log redo
hearth log redo 3
hearth redo
hearth redo 3
```

주의할 점은 현재 앱의 Undo 토스트가 아직 `audit_log` 기반 undo와 완전히 연결되어 있지는 않다는 점입니다. CLI undo/redo는 CLI 감사 로그 엔진을 사용하며, 앱은 DB 변경을 화면에 반영합니다.

## 내보내기와 가져오기

JSON으로 내보냅니다.

```bash
hearth export --format json --out hearth-dump.json
hearth export --include-audit --out hearth-dump-with-audit.json
```

`--out`을 생략하면 stdout으로 원본 JSON이 나옵니다.

```bash
hearth export > hearth-dump.json
```

SQLite DB 파일을 그대로 복사합니다. 이 경우 `--out`이 필요합니다.

```bash
hearth export --format sqlite --out hearth-copy.db
```

가져오기는 JSON export 파일만 지원합니다.

```bash
hearth import hearth-dump.json --dry-run
hearth import hearth-dump.json --merge
```

기존 데이터를 모두 교체하려면 `--replace --yes`가 필요합니다. 교체 전 현재 DB 옆에 `pre-import-<timestamp>.db` 백업이 만들어집니다.

```bash
hearth import hearth-dump.json --replace --yes
```

## 앱과 함께 쓰기

앱을 켜둔 상태에서 CLI로 데이터를 바꾸면 UI가 자동으로 갱신됩니다.

```bash
npm run tauri dev
```

다른 터미널에서 같은 DB를 대상으로 CLI를 실행합니다.

```bash
./src-tauri/target/release/hearth memo create "앱에서 바로 보이는 CLI 메모" --color blue
```

앱은 500ms 간격으로 DB의 `PRAGMA data_version`을 확인하고, 변경이 감지되면 `projects:changed`, `memos:changed`, `schedules:changed`, `categories:changed` 이벤트를 발생시킵니다. React 훅들이 이 이벤트를 받아 데이터를 다시 읽습니다.

## 에이전트 자동화에서의 권장 사용법

자동화나 AI 에이전트가 CLI를 쓸 때는 JSON 기본 출력을 유지하는 편이 좋습니다.

```bash
hearth project list | jq '.data[] | {id, name, priority}'
```

테스트나 실험은 실제 앱 DB가 아니라 임시 DB에서 먼저 실행합니다.

```bash
HEARTH_DB=/tmp/hearth-agent-test.db hearth db migrate
HEARTH_DB=/tmp/hearth-agent-test.db hearth project create "agent smoke" --priority P0
HEARTH_DB=/tmp/hearth-agent-test.db hearth search "agent"
```

실제 DB를 바꾸기 전에 현재 DB 경로를 확인합니다.

```bash
hearth db path
```

파괴적인 작업 전에는 export를 남깁니다.

```bash
hearth export --format sqlite --out "backup-before-agent.db"
hearth import new-dump.json --dry-run
```

## 현재 범위와 제한

- `clients`는 core에서 읽기 전용 목록 함수만 있으며 CLI subcommand는 아직 없습니다.
- CLI AI 프록시인 `hearth ai "..."` 명령은 의도적으로 포함하지 않았습니다.
- 검색 FTS5는 external-content 모드가 아니라 standalone FTS5 테이블을 씁니다. 현재 검색 기능에는 기능상 차이가 없고, 짧은 텍스트 필드만 중복 저장됩니다.
- Windows/Linux 빌드는 아직 검증하지 않았습니다. CLI 자체는 순수 Rust와 번들 SQLite 기반이지만, 기본 DB 경로 해석은 현재 macOS 중심입니다.
- 앱의 기존 Undo 토스트와 CLI `audit_log` undo/redo는 후속 작업에서 하나의 undo stack으로 통합할 수 있습니다.
