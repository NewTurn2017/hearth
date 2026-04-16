```
██╗  ██╗███████╗ █████╗ ██████╗ ████████╗██╗  ██╗
██║  ██║██╔════╝██╔══██╗██╔══██╗╚══██╔══╝██║  ██║
███████║█████╗  ███████║██████╔╝   ██║   ███████║
██╔══██║██╔══╝  ██╔══██║██╔══██╗   ██║   ██╔══██║
██║  ██║███████╗██║  ██║██║  ██║   ██║   ██║  ██║
╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝
```

**Local-first personal workspace for projects, memos, and schedules — driven by an AI command palette.**

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri)](https://tauri.app)
[![React](https://img.shields.io/badge/React-19-61DAFB?logo=react)](https://react.dev)
[![Rust](https://img.shields.io/badge/Rust-edition%202021-000000?logo=rust)](https://rust-lang.org)
[![License](https://img.shields.io/github/license/NewTurn2017/hearth)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)](https://tauri.app)

---

Hearth는 개인 프로젝트 · 스티키 메모 · 일정을 한 곳에서 관리하는 **로컬 퍼스트 데스크톱 앱**입니다. 모든 데이터는 로컬 SQLite에 저장되고, `⌘K` 커맨드 팔레트에서 **AI에게 자연어로 지시**하면 tool-calling으로 실제 항목을 만들고 고치고 찾아줍니다. AI 백엔드는 **로컬 MLX**(오프라인) 또는 **OpenAI** 중 선택할 수 있습니다.

## Features

- **명령 팔레트** (`⌘K`) — 프로젝트 생성 · 메모 추가 · 탭 전환 · AI 대화가 전부 한 입력창에서
- **AI 도우미** — "내일 오후 3시에 치과 예약 추가해줘" 같은 한국어 지시를 그대로 실행
- **변경 확인 다이얼로그** — 생성 · 수정 · 삭제 전에 항상 확인 모달 + 실행 후 Undo 토스트
- **프로젝트 보드** — 우선순위 `P0~P4`, 카테고리(Active/Side/Lab/Tools/Lecture), 경로, 평가 메모
- **스티키 메모** — 5색, 특정 프로젝트에 붙이기 · 떼기
- **일정 캘린더** — `react-big-calendar` 기반 월/주/일 뷰, 드래그로 이동
- **듀얼 AI 백엔드** — 로컬 MLX(`localhost:18080`) 또는 OpenAI `gpt-5.4-mini` 런타임 전환
- **오프라인 가능** — MLX 백엔드 쓰면 인터넷 없이 전부 동작
- **로컬 저장** — 모든 데이터는 SQLite 한 파일(`~/Library/Application Support/com.genie.project-genie/`)에

## Screenshots

> _스크린샷 추가 예정._

## Installation

릴리즈 바이너리는 아직 없습니다. 지금은 [Building from Source](#building-from-source) 섹션을 참고해 직접 빌드해 주세요.

**시스템 요구사항**

| 항목 | 최소 사양 |
|------|----------|
| OS | macOS 13+, Windows 10+, Linux (glibc 2.31+) |
| 메모리 | 4 GB (MLX 백엔드 쓰면 16 GB 권장) |
| 저장 공간 | 150 MB (MLX 모델 별도, ~3 GB) |
| 기타 | Rust 1.75+, Node.js 20+, npm |

## Usage

### 명령 팔레트

```
⌘K      (macOS)
Ctrl+K  (Windows / Linux)
```

- **일반 모드** — 프로젝트 검색, 탭 전환, "new project"·"new memo" 같은 빠른 동작
- **AI 모드** — 맨 앞에 `>` 또는 자연어로 요청 (`"프로젝트 목록 보여줘"`, `"P0만 필터"`)
- **ESC** — 팔레트 닫기

### AI 예시

| 입력 | 동작 |
|---|---|
| `프로젝트 목록 보여줘` | `list_projects` 호출 후 리스트 응답 |
| `P0 우선순위만 필터` | 프로젝트 뷰로 이동 + 필터 적용 |
| `내일 오후 3시 치과 예약 추가` | 확인 모달 → 일정 생성 |
| `'WebApp' 프로젝트 만들어줘` | 확인 모달 → 프로젝트 생성 |
| `달력 탭으로 가줘` | 캘린더 뷰로 전환 |
| `안녕` | 도구 호출 없이 대화 응답 |

변경을 일으키는 요청은 **항상 확인 다이얼로그**가 뜨고, 실행 후 상단 토스트에서 `Undo` 가능합니다.

## AI Setup

Hearth는 설정 다이얼로그(`⌘,`)에서 AI 백엔드를 고릅니다.

### 로컬 MLX (오프라인, Apple Silicon 권장)

```bash
# huggingface-cli 설치 (pipx 권장)
brew install pipx
pipx ensurepath
pipx install "huggingface_hub[cli]"

# 기본 모델 (~3 GB, Apple Silicon 최적화)
huggingface-cli download mlx-community/gemma-4-e4b-it-4bit

# 로컬 서버 기동
mlx_lm.server --model mlx-community/gemma-4-e4b-it-4bit --port 18080
```

Hearth 설정에서 provider를 **`mlx`** 로 두면 `http://127.0.0.1:18080` 에 자동으로 붙습니다. 한 번 모델을 캐시해두면 이후 인터넷 없이 전부 동작합니다.

### OpenAI (클라우드, 모든 플랫폼)

1. [platform.openai.com/api-keys](https://platform.openai.com/api-keys) 에서 API 키 발급
2. Hearth 설정(`⌘,`) → AI → provider **`openai`** 선택
3. API 키 붙여넣기

모델은 `gpt-5.4-mini` 하드코딩 (도구 호출 정확도 + 단가 균형). 키는 로컬 SQLite의 settings 테이블에 평문으로 저장되니 공용 기기 사용은 피해주세요.

## Data Storage

| 항목 | 경로 (macOS) |
|---|---|
| DB 파일 | `~/Library/Application Support/com.genie.project-genie/hearth.db` |
| AI 설정 | 같은 DB의 `settings` 테이블 |
| 백업 수동 내보내기 | 명령 팔레트 → `Backup DB` |

Windows/Linux의 경우 Tauri가 OS 표준 데이터 디렉토리를 씁니다 (`%APPDATA%`, `$XDG_DATA_HOME` 등).

## Testing

Rust 통합 테스트 스위트가 **OpenAI tool-calling 정확도**를 실제 호출로 검증합니다. 기본 `cargo test`에선 제외(`#[ignore]`)돼 있어 API 호출·비용이 발생하지 않습니다.

```bash
cd src-tauri
OPENAI_API_KEY=sk-... cargo test --test tool_calling_integration \
    -- --ignored --test-threads=1 --nocapture
```

- 12개 시나리오 — 단순 선택, 인자 추출, Mutation 분류, No-tool 잡담, 모호성 방어
- 1회 수행 약 21초 · 약 $0.02

상세 스펙: `docs/superpowers/specs/2026-04-16-tool-calling-integration-tests-design.md`

## Building from Source

```bash
# 의존성
brew install rust node
npm install -g @tauri-apps/cli

# 클론 + 프론트엔드 설치
git clone https://github.com/NewTurn2017/hearth.git
cd hearth
npm install

# 개발 모드 (HMR + Rust 자동 리로드)
npm run tauri dev

# 프로덕션 번들 (DMG/MSI/AppImage)
npm run tauri build
```

빌드 결과물은 `src-tauri/target/release/bundle/` 에 생성됩니다.

## Architecture

```
┌───────────────────────────────────────────────┐
│  React 19 · Tailwind 4 · cmdk command palette │
└──────────────┬────────────────────────────────┘
               │ Tauri IPC (invoke)
┌──────────────┴────────────────────────────────┐
│  Rust core: rusqlite + reqwest + tokio        │
│  ├── cmd_ai.rs      tool-calling agent loop   │
│  ├── ai_tools.rs    13-tool registry          │
│  ├── cmd_projects   memos / schedules / ...   │
│  └── db.rs          SQLite schema + migration │
└──────────────┬────────────────────────────────┘
               │ HTTP (chat completions)
      ┌────────┴────────┐
      ▼                 ▼
  Local MLX         OpenAI
  (offline)      (gpt-5.4-mini)
```

**Tool-calling loop** (`cmd_ai.rs`)

1. 모델이 `ai_tools::specs()` 로 선언된 도구를 보고 `tool_calls` 생성
2. Rust 측이 도구를 세 종류로 분류해 다르게 처리:
   - `Read` — 즉시 실행, 결과를 모델에 피드백
   - `Mutation` — 루프 일시정지, UI가 확인 모달 표시 → 승인 시 재개
   - `ClientIntent` — 프론트엔드가 실행 (필터 · 탭 전환)
3. 최대 8스텝 안에 최종 응답 도달

## Contributing

이슈와 PR 환영합니다. 버그 제보는 재현 절차 · OS · 백엔드(MLX/OpenAI) · 콘솔 로그를 같이 남겨 주세요.

## License

MIT © 2026 NewTurn2017
