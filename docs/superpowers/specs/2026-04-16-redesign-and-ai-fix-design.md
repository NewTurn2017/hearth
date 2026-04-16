# Project Genie — 디자인 리워크 & AI 수정

**Date**: 2026-04-16
**Status**: Approved (brainstorming 완료, 구현 계획 수립 대기)
**Related**: `docs/superpowers/specs/2026-04-16-project-genie-design.md` (초기 v1 스펙)

## 배경 / 동기

v1 초기 구현(한 번의 스캐폴드)이 "빠르게 돌아가는 앱"은 만들었으나 다음 3가지 피드백이 들어왔다:

1. **UI 기본 디자인이 안 된 상태** — 토큰 시스템, 타이포 스케일, 공간/서피스 계층이 없고 Tailwind 기본값 위에 CSS vars 몇 개만 얹은 수준.
2. **AI 대화 기능이 실질적으로 동작하지 않음** — 서버 라이프사이클/툴 호출 감지/결과 피드백 모두 불안정.
3. **이모지가 UI 전반에 사용됨** — 일관성·접근성·렌더링 품질 문제. `lucide-react` 같은 라이너 아이콘으로 교체 필요.

본 스펙은 위 3개를 한 번에 해결하는 **전면 리디자인 + AI 백엔드 재작성** 을 정의한다. v1 스펙의 데이터 모델·Tauri 커맨드·DB 전략·Excel import 등은 유지한다.

## 스코프 요약

| 영역 | 조치 |
|------|------|
| 디자인 언어 | Notion 따뜻한 페이퍼 다크 톤으로 재구축 |
| 액센트 컬러 | Amber (`#d97706` primary, `#fbbf24` hover) |
| 아이콘 | 이모지 전량 제거 → `lucide-react` |
| AI 진입점 | 우측 드로어 → **⌘K 커맨드 바** |
| AI 확인 플로우 | **모든 액션에 확인 팝업** (테스트 단계, 안전 우선) |
| AI 라이프사이클 | Lazy 기동 + 앱 종료까지 유지 |
| AI 스코프 | 전체 CRUD + 요약/검색 (스냅샷 주입) |
| 컴포넌트 구조 | `src/ui/` (primitive) + `src/command/` (⌘K) 신규 |
| 기존 도메인 컴포넌트 | 전량 새 토큰/primitive 기반으로 리팩토링 |

## 1. 디자인 토큰

### 1.1 Color tokens

```
Surface (warm paper dark)
  --surface-0:  #141312   body bg
  --surface-1:  #1a1917   titlebar, sidebar
  --surface-2:  #221f19   cards, inputs
  --surface-3:  #2a2721   hover, active
  --border:     #2e2a23
  --border-strong: #3a362e

Text
  --text-hi:    #f4efcf   headings, brand
  --text:       #ebeadf   body
  --text-muted: #a7a496   secondary
  --text-dim:   #7a7668   labels

Brand (Amber)
  --brand:      #d97706   primary action
  --brand-hi:   #fbbf24   hover / focus ring
  --brand-soft: rgba(217,119,6,0.18)   chip bg

Semantic (priorities / status — v1 유지)
  --p0: #ef4444   --p1: #f97316   --p2: #eab308
  --p3: #3b82f6   --p4: #6b7280
  --success: #22c55e   --danger: #ef4444

Category (v1 유지)
  Active #22c55e  Side #f97316  Lab #a855f7
  Tools #6b7280   Lecture #3b82f6
```

### 1.2 Typography

SF Pro Text (macOS 기본) → Inter fallback. Mono 는 JetBrains Mono fallback.

| Token | Size | Line | Weight | Tracking | 용도 |
|-------|------|------|--------|----------|------|
| display | 22px | 1.2 | 600 | -0.015em | 페이지 타이틀 |
| heading | 15px | 1.3 | 600 | -0.005em | 섹션 |
| body | 13px | 1.45 | 400 | 0 | 기본 텍스트 |
| small | 12px | 1.4 | 400 | 0 | 보조 텍스트 |
| label | 10px | 1.4 | 600 | 0.06em UPPERCASE | 사이드바 라벨 |
| mono | 12px | 1.4 | 400 | 0 | 경로, 키보드 숏컷 |

### 1.3 Spacing / Radius / Shadow / Motion

```
Spacing: 2 / 4 / 6 / 8 / 10 / 12 / 16 / 20 / 24 / 32
Radius:  sm 6 · md 8 · lg 10 · xl 14 (modal/dialog)
Shadow:
  e1: 0 1px 2px rgba(0,0,0,.3)        cards
  e2: 0 4px 12px rgba(0,0,0,.35)      dropdown/popover
  e3: 0 20px 40px rgba(0,0,0,.5)      ⌘K, modal
Motion:
  duration: 120ms hover, 180ms layout, 220ms modal enter
  easing: cubic-bezier(.2,.8,.2,1)
```

### 1.4 Icons

- **라이브러리**: `lucide-react`
- **Stroke**: 1.75 일괄 (기본 2 보다 약간 얇음)
- **Size 토큰**: 14 (inline), 16 (버튼), 18 (헤더/탭)
- **공용 래퍼**: `src/ui/Icon.tsx` 가 size/color 표준화

### 1.5 Tailwind 4 바인딩

`src/App.css` 에 `@theme { --color-surface-0: ...; --radius-lg: ...; }` 등으로 바인딩하여 `bg-surface-0`, `rounded-lg`, `shadow-e2` 등 유틸 자동 생성.

## 2. 컴포넌트 아키텍처

### 2.1 디렉토리 구조

```
src/
├── ui/                     # 신규: Primitive (shadcn-lite, 각 100~150 LOC)
│   ├── Button.tsx          # variant: primary | secondary | ghost | danger
│   ├── Input.tsx           # 텍스트/search 공용
│   ├── Dialog.tsx          # 모달, 포커스 트랩, ESC 핸들
│   ├── Popover.tsx
│   ├── Tooltip.tsx
│   ├── Badge.tsx           # priority / category pill
│   ├── Kbd.tsx             # ⌘K, ⏎ 키 힌트
│   ├── Icon.tsx            # lucide-react 래퍼
│   ├── Toast.tsx           # 성공/실패 + Undo
│   └── EmptyState.tsx      # 공용 빈 상태 뷰
├── command/                # 신규: ⌘K 커맨드 바
│   ├── CommandPalette.tsx  # 루트, 단축키, 백드롭
│   ├── CommandInput.tsx
│   ├── CommandResults.tsx
│   ├── CommandEmpty.tsx
│   ├── useCommandState.ts
│   └── dispatch.ts         # Mode 판단/실행 파이프
├── components/             # 기존 도메인 (전량 리팩토링)
│   ├── Layout.tsx          # AI Panel 제거, CommandPalette 마운트
│   ├── TopBar.tsx          # TabBar 리네임
│   ├── Sidebar.tsx
│   ├── ProjectCard.tsx
│   ├── ProjectList.tsx
│   ├── MemoCard.tsx
│   ├── MemoBoard.tsx
│   ├── CalendarView.tsx
│   └── ScheduleModal.tsx   # Dialog primitive 위로
├── hooks/                  # 기존 유지 + useAi 재작성
└── lib/
    ├── cn.ts               # clsx + tailwind-merge
    └── shortcuts.ts        # 전역 단축키 (⌘K)
```

### 2.2 Primitive 구현 원칙

- **Radix 미도입** — 번들 최소화. 순수 React + Tailwind.
- **variant API 통일** — `variant`, `size`, `className` 3개 prop 표준.
- **포커스 링** — `focus-visible:ring-2 ring-[--brand-hi] ring-offset-2 ring-offset-[--surface-0]` 공통.
- **아이콘 허용** — Button 에 `leftIcon` / `rightIcon` prop 만 허용, children 은 텍스트.

### 2.3 신규 의존성

```json
"lucide-react": "^0.475",
"clsx": "^2.1",
"tailwind-merge": "^3.0",
"cmdk": "^1.0"
```

### 2.4 삭제될 기존 파일

- `src/components/AiPanel.tsx`
- `src/components/ChatMessage.tsx`

(`hooks/useAi.ts` 는 동일 경로에 완전히 재작성)

## 3. ⌘K 커맨드 바 동작 모델

### 3.1 진입 / 종료

- **⌘K** — 어디서나 열림. 이미 열려있으면 입력 포커스만 재설정.
- **ESC** — 닫힘. 내부 확인 Dialog 가 열려있으면 Dialog 먼저 닫힘.
- 백드롭 클릭 → 닫힘.

### 3.2 2-모드 자동 전환

**Mode 1 — 로컬 퀵액션 (AI 미호출)**
- 입력이 비었거나 `/` 로 시작.
- 가능한 액션: `새 프로젝트`, `새 일정`, `새 메모`, `백업 생성`, `Excel 가져오기` 등.
- fuzzy 매칭 (`cmdk` 기본 제공).

**Mode 2 — AI 질의**
- `?` 프리픽스 or ⇧⏎ or Mode 1 매칭 결과 없음.
- 300ms debounce 후 `ai_chat` 호출.

### 3.3 결과 렌더링

```
┌────────────────────────────────────────┐
│  ◎  [입력 필드]          ESC  ⌘K      │
├────────────────────────────────────────┤
│  {reply 자연어 답변}                    │
│  ─────────────────                      │
│  제안 액션                              │
│   ■ 라벨           ⏎ 실행 / ⇥ 포커스  │
│   ■ 라벨                                │
└────────────────────────────────────────┘
```

- AI 응답의 `actions` 배열 각 항목을 하단 리스트로 렌더링
- 상단은 `reply` 자연어 답변
- `type: 'navigation'` / `'info'` → ⏎ 즉시 실행
- `type: 'mutation'` → ⏎ 시 **중첩 확인 Dialog** 표시

### 3.4 확인 Dialog

```
┌────────────────────────────────────────┐
│  확인                                    │
│  ─────────────────                       │
│  Genie Redesign 의 우선순위를            │
│  P0 → P1 로 변경합니다.                  │
│                                          │
│  [취소]              [실행 ⏎]            │
└────────────────────────────────────────┘
```

- 기본 포커스: **실행** 버튼 (⏎ 연속 입력 편의)
- ESC / 취소 → Dialog 만 닫힘, 팔레트는 유지
- 사용자 검증 단계에서는 모든 `mutation`에 적용. 추후 "destructive-only" 모드 스위치 추가 가능 (v2 스코프).

### 3.5 실행 후 피드백

- **성공** → 우측 하단 Toast: `"{라벨} 완료"` + [Undo] 5초 타이머
- **실패** → Toast 에러, 팔레트 열린 상태 유지, 입력 보존

### 3.6 키보드

| 키 | 동작 |
|----|------|
| ⌘K | 열기 / 포커스 |
| ESC | 닫기 (Dialog 열려있으면 Dialog 먼저) |
| ↑ / ↓ | 결과 항목 이동 |
| ⏎ | 선택 항목 실행 |
| ⇧⏎ | Mode 2 강제 진입 (로컬 매칭 있을 때도 AI 호출) |
| Tab / ⇧Tab | 포커스 순회 |

## 4. AI 백엔드 재설계

### 4.1 현재 문제 (삭제 대상)

1. `useEffect` cleanup 에서 `stopAiServer()` 호출 → 패널 닫자마자 서버 사망.
2. Tool call 파싱이 `{` 시작 라인 1개만 탐지 → 다중 액션/자연어 혼재 시 실패.
3. 스트리밍 없음 → 로딩 중 사용자 피드백 약함.
4. 툴 실행 결과가 다음 턴 컨텍스트에 미주입.
5. `pkill -f mlx_lm.server` → 동일 호스트의 다른 MLX 인스턴스까지 종료.

### 4.2 Rust 라이프사이클

```rust
// cmd_ai.rs (재작성)
pub enum AiState {
    Idle,
    Starting { pid: u32, spawned_at: Instant },
    Running  { pid: u32, port: u16 },
    Failed   { error: String, last_try_at: Instant },
}

pub struct AiManager {
    pub state: Mutex<AiState>,
    pub script_path: Mutex<String>,
}
```

- 첫 ⌘K AI 호출 시점에 `start_ai_server` 트리거 → `Starting` 상태, 백그라운드에서 `/v1/models` 폴링 → `Running` 또는 `Failed`.
- `Running` 상태면 `ai_chat` 즉시 호출.
- `tauri::WindowEvent::Destroyed` 에서 저장된 `pid` 로 **지정 kill** (`pkill` 사용 안 함).
- 탭 닫기 / 커맨드 바 종료 시 **서버 유지**.

### 4.3 구조화 출력 강제

mlx_lm.server 가 지원하는 `response_format: { type: "json_schema", json_schema: {...} }` 사용.

**스키마**:
```json
{
  "type": "object",
  "required": ["reply", "actions"],
  "properties": {
    "reply":   { "type": "string" },
    "actions": {
      "type": "array",
      "items": {
        "type": "object",
        "required": ["type", "label"],
        "properties": {
          "type":    { "enum": ["mutation","navigation","info"] },
          "label":   { "type": "string" },
          "command": { "enum": [
            "create_project","update_project","delete_project",
            "create_schedule","update_schedule","delete_schedule",
            "create_memo","update_memo","delete_memo",
            "set_filter","focus_project"
          ]},
          "args":    { "type": "object" }
        }
      }
    }
  }
}
```

**폴백**: `response_format` 미지원 시 시스템 프롬프트로 strict JSON 강제 + `serde_json::from_str` 실패하면 1회 재시도 ("JSON 형식이 틀렸습니다. 스키마에 맞춰 다시 답하세요").

### 4.4 시스템 프롬프트

```
너는 Project Genie 의 AI 어시스턴트다. 한국어로 답한다.
사용자 요청에 JSON 으로 응답한다. "reply" 는 자연어, "actions" 는 수행할 액션 배열 (없으면 빈 배열).

현재 상태:
- 프로젝트 N개 (P0 a, P1 b, P2 c, P3 d, P4 e)
- 이번 달 일정 M개
- 메모 K개

[프로젝트 목록]
[P0] Genie Redesign (Active)
[P0] Claude Agent SDK (Lab)
... (최대 50개)

[이번 달 일정]
2026-04-17 15:00 팀 미팅 @ 강남
... (최대 30개)

[최근 메모 10개]
- 오늘 읽을 좋은 시 한편 메모해보기
...

사용 가능한 command:
  create_project(name, priority, category?, path?)
  update_project(id, fields)
  delete_project(id)
  create_schedule(date, time?, location?, description?, notes?)
  update_schedule(id, fields)
  delete_schedule(id)
  create_memo(content, color?, project_id?)
  update_memo(id, fields)
  delete_memo(id)
  set_filter(priorities?, categories?)
  focus_project(id)

규칙:
1) 생성/수정/삭제 command (create_*, update_*, delete_*) 는 모두 type: mutation — 실행은 사용자가 UI 에서 확인한다.
2) set_filter, focus_project 는 type: navigation — 확인 없이 즉시 실행.
3) 단순 조회/요약은 reply 에만 서술, actions 는 빈 배열.
4) 존재하지 않는 프로젝트/일정/메모 는 추측하지 않고 사용자에게 되물어본다.
```

### 4.5 스트리밍

**Phase 1**: non-streaming. 커맨드 바에 스피너 + 경과 시간 표시(예: "AI 생각 중 · 3s").
**Phase 2** (이번 스펙 밖): SSE 스트리밍으로 `reply` 타이핑 효과.

### 4.6 Rust 커맨드 시그니처 (변경/신규)

```
start_ai_server()    → { state: "starting"|"running"|"failed", port, error? }
ai_server_status()   → 동일
stop_ai_server()     → () : 앱 종료 훅에서만 호출됨
ai_chat(messages)    → { reply: string, actions: Action[] }
```

### 4.7 프론트엔드 파이프

```
useAi.ts (재작성):
  sendQuery(text):
    1. checkStatus → 필요 시 startServer 기다림
    2. messages = [system(once), ...history, {role:user, content:text}]
    3. response = await aiChat(messages)
    4. append {role:assistant, content:JSON.stringify(response)} to history
    5. return { reply, actions }

CommandPalette:
  userSelectAction(action):
    if action.type === 'mutation':
      showConfirmDialog(action) → on confirm → executeAction(action)
    else:
      executeAction(action)

executeAction(action):
  switch(action.command):
    'create_project' → api.createProject(...)
    ...
  onSuccess: toast.success + undoQueue.push(inverseAction)
  onError:   toast.error
```

## 5. 마이그레이션 순서 & 테스트

### 5.1 Phase 0 — 토큰 기반 (1 commit)

1. `npm i lucide-react clsx tailwind-merge cmdk`
2. `src/lib/cn.ts` 추가
3. `src/App.css` 재작성 (신규 토큰 전체 교체, `@theme` 바인딩)
4. 기본 bg/텍스트 스모크 테스트

### 5.2 Phase 1 — UI Primitives (1 commit)

- `src/ui/` 전체 추가: Button, Input, Dialog, Popover, Tooltip, Badge, Kbd, Icon, Toast, EmptyState
- `App.tsx` 임시 페이지에서 각 primitive 스모크 검증 → 검증 후 제거

### 5.3 Phase 2 — 도메인 컴포넌트 리팩토링 (2~3 commits)

| 파일 | 변경 |
|------|------|
| TopBar (TabBar 리네임) | lucide 아이콘, 이모지 제거, 밀도 조정 |
| Sidebar | 라벨·간격·pill 토큰 적용 |
| ProjectCard / ProjectList | `▶📁✕≡` → `Play/FolderOpen/X/GripVertical`, hover 상태 통일 |
| MemoCard / MemoBoard | `⠿` → `GripVertical`, 색상 토큰 재적용 |
| CalendarView | react-big-calendar 테마 CSS 재작성 (amber/새 surface) |
| ScheduleModal | Dialog primitive 기반 재작성 |
| EmptyState 적용 | "프로젝트 없음" 등 일관 처리 |

### 5.4 Phase 3 — Command Palette (1 commit)

- `src/command/*` 구현
- `src/lib/shortcuts.ts` — 전역 ⌘K 등록
- Mode 1 (로컬 퀵액션) 먼저 동작 확인
- 확인 Dialog 플로우
- Toast + Undo 큐

### 5.5 Phase 4 — AI 백엔드 재작성 (2 commits)

- Rust `cmd_ai.rs` 재작성 (상태기계, pid kill, response_format)
- TypeScript `useAi.ts` 재작성
- Mode 2 + 확인 Dialog 연결
- messages 히스토리 관리

### 5.6 Phase 5 — 정리 (1 commit)

- `AiPanel.tsx`, `ChatMessage.tsx` 삭제
- dead import / unused CSS 제거
- 이모지 grep 0건 확인

### 5.7 수동 검증 체크리스트

- [ ] `rg '[\x{1F300}-\x{1FAFF}]' src` → 0건
- [ ] TopBar 아이콘 lucide
- [ ] ⌘K 어디서나 열림, ESC 닫힘
- [ ] `/` → 로컬 퀵액션만
- [ ] `?` / 자연어 → AI 경로
- [ ] mutation → 확인 Dialog
- [ ] 실행 후 리스트 즉시 갱신 + Toast
- [ ] Undo → 원복
- [ ] AI 서버 start → 로딩 표시 → 응답
- [ ] 팔레트 닫아도 서버 유지
- [ ] 앱 종료 시 `mlx_lm.server` 프로세스 정리

### 5.8 자동 테스트

현재 리포에 테스트 인프라 없음. 이 스펙 범위:
- Rust `cmd_ai` JSON 스키마 파싱 유닛 테스트 1~2 개 (스키마 성공/폴백 재시도)

프론트엔드 테스트 인프라는 v2 작업.

### 5.9 롤백

- Phase 별 단일 commit → 해당 commit revert 가능
- DB 스키마 변경 없음 → 데이터 롤백 불필요

## 6. Scope Out

- 프론트엔드 단위 테스트 인프라
- AI 스트리밍 응답 (SSE)
- AI 대화 히스토리 영구 저장
- 다중 AI 모델 선택 UI
- MLX 스크립트 경로/모델 선택 설정 화면 (하드코딩 유지)
- "destructive-only" 확인 모드 스위치 (v2)

## 7. 결정 요약 (Decisions log)

| # | 결정 | 근거 |
|---|------|------|
| 1 | Notion 따뜻한 페이퍼 다크 | 포스트잇 메모와 시각적 연속성, 장시간 사용 편안함 |
| 2 | Amber 액센트 (#d97706) | 베이지 톤과 조화, 보라 대비 포근함 |
| 3 | ⌘K 커맨드 바 (드로어 아님) | 공간 절약, 빠른 자연어 명령, Raycast 식 UX |
| 4 | 모든 mutation 확인 팝업 | 테스트 단계 안전성 우선. 추후 완화 가능 |
| 5 | Lazy + 앱 종료까지 유지 | 첫 기동 대기 1회, 이후 즉시 응답 |
| 6 | 전체 CRUD 스코프 | 실사용 테스트 목적 — 제한 없이 열어둠 |
| 7 | 전면 재구축 (접근 A) | 토큰만 바꾸면 피드백 1번 미충족 |
| 8 | lucide-react + cmdk + clsx + tailwind-merge | 경량, 생태계 표준, 번들 영향 최소 |
