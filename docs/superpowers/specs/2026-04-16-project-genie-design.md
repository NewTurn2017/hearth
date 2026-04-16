# Project Genie - Design Spec

## Overview

로컬 Mac 데스크탑 앱. 개인 프로젝트 포트폴리오를 한눈에 관리하고, 일정 확인, 메모 작성, 프로젝트 폴더 빠른 접근을 제공한다.

**핵심 원칙:** 빠른 속도 > 확실한 저장 > 심플한 UX > 깔끔한 디자인

## Tech Stack

| 영역 | 선택 | 이유 |
|------|------|------|
| Framework | Tauri v2 | 가벼움, 네이티브 성능 |
| Frontend | React + TypeScript | DnD/캘린더 생태계 |
| Bundler | Vite | 빠른 빌드 |
| DB | SQLite via rusqlite | Rust 네이티브, 단일 파일 |
| 스타일 | Tailwind CSS | 빠른 스타일링, 다크테마 |
| DnD | @dnd-kit | 경량, 고성능 |
| 캘린더 | react-big-calendar | 심플 월간 뷰 |
| Excel 파싱 | calamine (Rust crate) | xlsx → SQLite import |

## Data Model

### projects

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | auto increment |
| priority | TEXT | P0, P1, P2, P3, P4 |
| number | INTEGER | 원본 No. (nullable) |
| name | TEXT NOT NULL | 프로젝트명 |
| category | TEXT | Active, Side, Lab, Tools, Lecture |
| path | TEXT | 로컬 경로 (nullable) |
| evaluation | TEXT | 평가/메모 |
| sort_order | INTEGER | 드래그 정렬용 |
| created_at | TEXT | ISO 8601 |
| updated_at | TEXT | ISO 8601 |

### schedules

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | auto increment |
| date | TEXT NOT NULL | 날짜 (YYYY-MM-DD) |
| time | TEXT | 시간 (nullable) |
| location | TEXT | 장소 |
| description | TEXT | 내용 |
| notes | TEXT | 비고 |
| created_at | TEXT | ISO 8601 |
| updated_at | TEXT | ISO 8601 |

### memos

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | auto increment |
| content | TEXT NOT NULL | 메모 내용 |
| color | TEXT DEFAULT 'yellow' | yellow, pink, blue, green, purple |
| project_id | INTEGER | FK → projects.id (nullable) |
| sort_order | INTEGER | 정렬/위치 |
| created_at | TEXT | ISO 8601 |
| updated_at | TEXT | ISO 8601 |

### clients

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | auto increment |
| company_name | TEXT | 회사명 |
| ceo | TEXT | 대표 |
| phone | TEXT | 연락처 |
| fax | TEXT | 팩스 |
| email | TEXT | 이메일 |
| offices | TEXT | JSON (여러 사무실) |
| project_desc | TEXT | 프로젝트 설명 |
| status | TEXT | 현재 상태 |
| created_at | TEXT | ISO 8601 |
| updated_at | TEXT | ISO 8601 |

## UI Layout

### 전체 구조

- **상단 탭 바**: 프로젝트(📋) / 캘린더(📅) / 메모보드(📌)
- **좌측 사이드바**: 우선순위 필터 (P0~P4) + 카테고리 필터 (Active/Side/Lab/Tools/Lecture)
- **메인 영역**: 탭에 따른 콘텐츠

### 프로젝트 탭

- 우선순위 그룹별로 프로젝트 카드 나열
- 각 카드: 프로젝트명, 카테고리 배지, 평가 텍스트
- `≡` 드래그 핸들로 같은 그룹 내 순서 변경
- 액션 버튼: ▶ Ghostty 열기, 📁 Finder 열기
- 인라인 편집: 프로젝트명, 평가, 우선순위 클릭하여 즉시 수정
- 사이드바 필터로 우선순위/카테고리 토글

### 캘린더 탭

- react-big-calendar 월간 뷰
- 일정 클릭 → 상세 보기/편집
- 새 일정 추가 버튼
- 일정 CRUD (생성/읽기/수정/삭제)

### 메모보드 탭

- 포스트잇 스타일 카드 그리드
- 색상 선택 (5가지 파스텔 컬러)
- 프로젝트 연결 가능 (드롭다운 선택)
- 드래그로 순서 변경
- 새 메모 추가: + 카드 클릭
- 인라인 편집

## Design Direction

- **다크 테마** 기본
- 포스트잇 메모: 파스텔 컬러 + box-shadow로 떠있는 느낌
- 우선순위 색상: 🔴 P0 red, 🟠 P1 orange, 🟡 P2 yellow, 🔵 P3 blue, ⚪ P4 gray
- 카테고리 배지: 각 카테고리별 고유 색상
- 깔끔하고 미니멀한 UI, 불필요한 장식 없음
- 트랜지션: 드래그/드롭, 탭 전환 시 부드러운 애니메이션

## 저장 전략

- **즉시 저장**: 모든 변경 즉시 SQLite 반영. 별도 저장 버튼 없음.
- **WAL 모드**: SQLite Write-Ahead Logging으로 읽기/쓰기 동시 성능.
- **DB 위치**: `~/Library/Application Support/project-genie/data.db`

## 성능 최적화

- Rust 백엔드에서 SQLite 직접 처리 → IPC 오버헤드 최소화
- 프로젝트 ~50개 수준 → 가상 스크롤 불필요, 단순 DOM 렌더링
- 드래그 시 `sort_order`만 부분 업데이트
- Tauri IPC는 JSON 직렬화 → 필요한 필드만 전송

## 외부 연동

### Ghostty 열기
```
open -a Ghostty <project_path>
```
Rust `std::process::Command`로 실행. 경로 존재 여부 사전 체크.

### Finder 열기
```
open <project_path>
```

### Excel Import
- 첫 실행 시 DB 비어있으면 import 안내
- 메뉴에서 수동 import 가능
- `calamine` crate로 xlsx 파싱 → SQLite insert
- 기존 데이터 있으면 덮어쓰기 전 확인 다이얼로그

## Tauri Commands (Rust → Frontend IPC)

```
// Projects
get_projects(filter?) → Project[]
update_project(id, fields) → Project
reorder_projects(priority, order[]) → void

// Schedules
get_schedules(month?) → Schedule[]
create_schedule(data) → Schedule
update_schedule(id, fields) → Schedule
delete_schedule(id) → void

// Memos
get_memos() → Memo[]
create_memo(data) → Memo
update_memo(id, fields) → Memo
delete_memo(id) → void
reorder_memos(order[]) → void

// Actions
open_in_ghostty(path) → Result
open_in_finder(path) → Result
import_excel(file_path) → ImportResult

// Clients
get_clients() → Client[]
```

## Scope Out (v1에서 제외)

- 클라우드 동기화
- 다중 사용자
- 알림/리마인더
- 프로젝트 상세 페이지 (별도 뷰)
- Git 상태 연동
- 검색 기능 (v1은 필터로 충분)
