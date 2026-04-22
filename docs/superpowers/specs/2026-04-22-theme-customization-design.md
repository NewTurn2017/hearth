# 테마 커스터마이징 — 디자인 문서

**날짜:** 2026-04-22
**버전 타깃:** 0.7.0 (잠정)
**상태:** 설계 승인 대기 → 플래닝

## 목적

현재 Hearth는 "Warm Paper Dark" 단일 테마(앰버 강조색) 고정이다. 이 문서는 사용자가 **프리셋 10개(다크 5 + 라이트 5)** 중 하나를 고르거나, **커스텀 강조색**으로 자기 테마를 만들 수 있도록 하는 기능의 설계안을 정의한다.

**목적이 아닌 것:**

- 시스템 다크/라이트 자동 추종 (명시적으로 제외)
- 표면/텍스트까지 사용자가 수작업 조정하는 완전 커스텀 (복잡도 대비 가치 낮음)
- 프리셋별 그림자·반경·타이포 변경 (앱 성격을 흔들어 제외)
- 사용자 정의 "내 테마 저장/공유" (스코프 외)

## 요구사항 요약

1. **프리셋 10개** — 현재 Warm Paper를 기본값으로 유지하고 9개 신규 프리셋 추가
2. **커스텀** — 다크/라이트 베이스 모드 선택 + 강조색 HEX 하나 지정
3. **단일 선택** — 시스템 테마 무시, 사용자가 고른 하나만 활성
4. **영속** — 재시작해도 유지, 메인 창과 QuickCapture 모두 동기화
5. **즉시 반영** — 프리셋 클릭 시 앱 전체가 즉시 바뀜 (FOUC 없음)

## 프리셋 카탈로그

### 다크 5개

| ID | 이름 | 베이스 표면 | 브랜드 | 컨셉 |
|---|---|---|---|---|
| `warm-paper` | Warm Paper | `#141312` (웜 블랙) | `#d97706` 앰버 | 현재 기본, 종이/촛불 |
| `midnight` | Midnight | `#0f1420` (딥 네이비) | `#3b82f6` 블루 | 집중, 차분 |
| `forest` | Forest | `#0f1612` (딥 그린) | `#10b981` 에메랄드 | 자연, 안정 |
| `plum` | Plum | `#1a1320` (딥 퍼플) | `#a855f7` 바이올렛 | 개성, 야간 |
| `carbon` | Carbon | `#111111` (뉴트럴 블랙) | `#f97316` 오렌지 | 미니멀, 선명 |

### 라이트 5개

| ID | 이름 | 베이스 표면 | 브랜드 | 컨셉 |
|---|---|---|---|---|
| `cream` | Cream | `#fdf8ef` (웜 크림) | `#b45309` 앰버딥 | Warm Paper 주광판 |
| `linen` | Linen | `#fafaf7` (오프화이트) | `#1d4ed8` 네이비 | 업무용 뉴트럴 |
| `mint` | Mint | `#f4faf6` (쿨 화이트) | `#059669` 그린 | 산뜻 |
| `blush` | Blush | `#fdf5f6` (핑크 화이트) | `#be185d` 로즈 | 따뜻 |
| `arctic` | Arctic | `#f4f7fb` (블루 화이트) | `#0ea5e9` 시안 | 차가운 집중 |

각 프리셋의 전체 11개 토큰(`--color-surface-0..3`, `--color-border`, `--color-border-strong`, `--color-text-hi/text/muted/dim`, `--color-brand/brand-hi/brand-soft`) 구체 값은 `src/theme/presets.ts`에 정의한다. 정의 단계에서 WCAG AA 대비(본문 텍스트 vs surface-0/1)를 수작업 검증한다.

## 핵심 설계

### 1. Swap 메커니즘 — `[data-theme]` CSS + 커스텀만 JS 주입

**근거:** 현재 앱의 모든 UI가 `var(--color-*)`만 참조한다 — CSS 변수 값을 바꾸면 리액트 리렌더 없이 즉시 반영된다.

- `src/App.css`의 기본 `@theme` 블록은 Warm Paper 값 그대로 유지 → **Warm Paper = "no data-theme" 기본값**
- `src/theme/theme.css`에 나머지 9개 프리셋을 `[data-theme="midnight"] { --color-surface-0: ...; ... }` 형태로 추가
- React가 `document.documentElement.setAttribute("data-theme", id)` 호출
- 커스텀: `data-theme="custom"` + 런타임 주입 `<style id="hearth-custom-theme">:root[data-theme="custom"] { ... }</style>`

Priority, category, semantic 토큰(`--color-p0..p4`, `--color-cat-*`, `--color-success/danger`)과 `--radius-*`, `--shadow-*`, `--ease-*`, 타이포 변수는 **모든 프리셋에서 전역 공통** — 의미 매핑이므로 테마 독립.

### 2. 커스텀 파생 규칙

**입력**
```ts
{ baseMode: "light" | "dark", brandHex: "#rrggbb" }
```

**출력 토큰**
- `--color-brand` = 입력 HEX
- `--color-brand-hi` = 입력 HEX의 HSL에서 `L += 10` (clamp 45~75%)
- `--color-brand-soft` = 입력 HEX의 RGB에 alpha 0.18 적용 (`rgba(r,g,b,0.18)`)
- **나머지 8개 토큰 (surface 4 + border 2 + text 4)** = 베이스 모드의 **중립 프리셋 복사**
  - `baseMode === "dark"` → Carbon의 surface/border/text 토큰
  - `baseMode === "light"` → Linen의 surface/border/text 토큰

이렇게 하면 사용자는 강조색 하나만 고르면 되고 표면/텍스트는 검증된 중립 팔레트를 그대로 쓴다. 파생 로직은 `src/theme/derive.ts`로 분리 → 순수 함수로 테스트.

### 3. 상태 & 저장

**TypeScript 타입**
```ts
export type PresetId =
  | "warm-paper" | "midnight" | "forest" | "plum" | "carbon"
  | "cream" | "linen" | "mint" | "blush" | "arctic";

export type ThemeSetting =
  | { kind: "preset"; id: PresetId }
  | { kind: "custom"; baseMode: "light" | "dark"; brandHex: string };

export const DEFAULT_THEME: ThemeSetting = { kind: "preset", id: "warm-paper" };
```

**저장**
- **서버 측:** Tauri 백엔드 기존 settings 스토어에 새 커맨드 2개 추가 — `get_theme()` / `set_theme(theme: ThemeSetting)`. 직렬화는 JSON 한 행.
- **클라이언트 측 캐시:** `localStorage["hearth.theme"]` — FOUC 방지 용도. Rust 백엔드 응답 전에 이 값으로 먼저 페인트.
- 시작 순서:
  1. `main.tsx` 최상단에서 localStorage 읽음 → 있으면 즉시 `applyTheme(cached)` (React 마운트 전)
  2. React 마운트 후 `api.getTheme()` 호출
  3. 응답이 캐시와 다르면 `applyTheme(fresh)` + localStorage 갱신

### 4. 적용 API — `applyTheme`

`src/theme/applyTheme.ts`에 순수 DOM 함수 노출 (React 없이도 호출 가능).

```ts
export function applyTheme(theme: ThemeSetting): void
```

동작:
1. `theme.kind === "preset"` →
   - `html.setAttribute("data-theme", theme.id)`
   - 기존 `<style id="hearth-custom-theme">`가 있으면 제거
2. `theme.kind === "custom"` →
   - 파생 → 11개 토큰 값 계산
   - `<style id="hearth-custom-theme">:root[data-theme="custom"] { --color-...: ...; }</style>`를 `<head>`에 upsert
   - `html.setAttribute("data-theme", "custom")`

### 5. ThemeContext (React)

`src/theme/ThemeContext.tsx`

```ts
{
  theme: ThemeSetting,
  setTheme: (next: ThemeSetting) => Promise<void>,  // applyTheme + persist + emit
}
```

`setTheme` 내부:
1. `applyTheme(next)` (즉시)
2. `localStorage.setItem("hearth.theme", JSON.stringify(next))`
3. `api.setTheme(next)` (Tauri → SQLite)
4. Tauri event `theme-changed` emit (페이로드: `ThemeSetting`)

`App.tsx`를 `<ThemeProvider>`로 래핑.

### 6. QuickCapture 윈도우 동기화

`src/windows/QuickCapture.tsx`의 부팅 시퀀스:
1. localStorage에서 테마 캐시 즉시 읽음 → `applyTheme(cached)`
2. 마운트 후 `api.getTheme()` fetch → 다르면 재적용
3. `listen("theme-changed", (e) => applyTheme(e.payload))` 구독

메인창에서 테마를 바꿀 때 QuickCapture가 이미 떠있으면 **Tauri 이벤트 한 번**으로 반영된다.

### 7. Settings UI — 새 "테마" 탭

**SettingsDialog 탭 배열 변경:**
```ts
const TABS = [
  { key: "general", label: "일반" },
  { key: "theme",   label: "테마" },   // 신규
  { key: "ai",      label: "AI" },
  { key: "backup",  label: "백업/가져오기" },
  { key: "categories", label: "카테고리" },
];
```

**`SettingsThemeSection` 구조 (위에서 아래로):**

1. **다크 섹션** — 5개 카드 그리드 (2~3열, 반응형)
   - 카드 = 스와치 4개 (surface-0, surface-2, brand, text-hi) + 이름 + 선택 시 체크 아이콘
   - 클릭 즉시 `setTheme({ kind: "preset", id })`
2. **라이트 섹션** — 동일 구조, 5개
3. **커스텀 섹션**
   - 라디오: `◉ 다크  ○ 라이트` (베이스 모드)
   - HTML `<input type="color">` + 옆에 HEX 텍스트 입력(`#rrggbb` 유효성 검사)
   - 색 변경은 **디바운스 300ms로 라이브 미리보기**, "저장" 버튼 누르면 persist
   - "프리셋으로 되돌리기" 링크 → 이전 프리셋(없으면 `warm-paper`)으로 복귀

**선택 상태 표시:** 현재 활성 테마 카드/커스텀 섹션 상단에 `--color-brand` 보더 + 체크 아이콘.

### 8. 기존 하드코딩 값 처리

- `App.css`의 **find-highlight 노란 글로우** (`rgba(251, 191, 36, ...)`) — 하드코딩된 앰버. 이번 스코프에서는 **그대로 유지** (별도 후속 작업). 스펙에 기록.
- `App.css`의 **react-big-calendar 오버라이드** — `--color-*` 기반이라 자동 반영 예상. 10개 프리셋 순회 수동 검증 체크리스트에 포함.

## 파일 변경 요약

**신규 (9개)**
- `src/theme/presets.ts` — 10개 프리셋의 11개 토큰 값
- `src/theme/derive.ts` — HEX → brand-hi/soft 파생 + 커스텀 전체 토큰 합성
- `src/theme/applyTheme.ts` — DOM에 data-theme/style 적용 (순수)
- `src/theme/ThemeContext.tsx` — React context + 연동 로직
- `src/theme/theme.css` — 9개 `[data-theme="..."]` 블록 (warm-paper 제외)
- `src/theme/__tests__/derive.test.ts`
- `src/theme/__tests__/applyTheme.test.ts`
- `src/components/SettingsThemeSection.tsx`
- `src/components/__tests__/SettingsThemeSection.test.tsx`

**수정**
- `src/App.css` — `@import "./theme/theme.css"` 추가 (@theme 블록은 그대로)
- `src/main.tsx` — React 마운트 전 localStorage 캐시로 `applyTheme` pre-paint
- `src/App.tsx` — `<ThemeProvider>` 래핑
- `src/components/SettingsDialog.tsx` — 탭 배열에 "테마" 추가 + 섹션 마운트
- `src/api.ts` — `getTheme()` / `setTheme()` 함수 export
- `src/windows/QuickCapture.tsx` — pre-paint + `theme-changed` 이벤트 구독
- `src-tauri/src/` — `get_theme` / `set_theme` 커맨드 + settings 스토어 한 필드 (기존 구조 재사용)
- `src-tauri/tauri.conf.json` (필요 시 capability — 기존 settings와 동일)

## 상태 흐름도

```
[앱 부팅]
  main.tsx → read localStorage → applyTheme(cached or DEFAULT)
  → React mount → <ThemeProvider>
  → api.getTheme() → if differs: setTheme(fresh) → DOM 갱신

[사용자가 프리셋 카드 클릭]
  SettingsThemeSection → setTheme({ kind: "preset", id })
  → applyTheme 즉시 반영
  → api.setTheme 백엔드 저장
  → emit "theme-changed"
  → QuickCapture(있다면) 이벤트 수신 → applyTheme

[사용자가 커스텀 색 변경]
  color input onChange (디바운스 300ms) → applyTheme({ kind: "custom", ... })
  → "저장" 클릭 시에만 api.setTheme + emit
```

## 테스트 전략

- **`derive.test.ts`** — HEX → HSL 변환, brand-hi lightness clamp, brand-soft alpha, 경계값(#000000, #ffffff, 잘못된 HEX)
- **`applyTheme.test.ts`** (jsdom) — preset 적용 시 `data-theme` 속성 세팅, 커스텀 적용 시 `<style id="hearth-custom-theme">` 주입, preset ↔ custom 전환 시 style 태그 제거
- **`SettingsThemeSection.test.tsx`** — 카드 클릭 → context.setTheme 호출, 커스텀 HEX 유효성 검사, 라이브 미리보기 디바운스, "되돌리기" 동작
- **수동 검증 체크리스트** (플랜에 포함):
  - 10개 프리셋 순회 — 모든 화면(메모 보드·매트릭스·프로젝트·캘린더·Quick Capture·모든 다이얼로그·팔레트)에서 가독성/대비 확인
  - 커스텀 테마 3~5종(기존 브랜드와 동떨어진 색) — 문제 없는지 확인
  - QuickCapture 오픈 중 테마 바꿔서 동기화 확인
  - 앱 재시작 시 선택한 테마 복원 확인

## 리스크 & 미해결

- **react-big-calendar 일부 셀 색이 고정일 수 있음** — 순회 검증에서 발견 시 플랜에서 개별 대응
- **WCAG AA 대비** — 프리셋은 정의 단계에서 수작업 검증, 커스텀은 "베이스 모드 분리"로 최소 보장(완벽 방지는 아님). 필요하면 후속 작업에서 "대비 경고" 추가
- **find-highlight 노란 글로우 하드코딩** — 이번 스코프 외, 별도 이슈
- **메모 노트의 배경색(포스트잇)** — 카드 배경이 브랜드색과 독립된 고정 팔레트일 가능성. 순회 검증에서 확인 필요

## 성공 기준

- 설정 → 테마 탭에서 10개 프리셋 중 하나 또는 커스텀 강조색 선택
- 선택 즉시 메인 앱·Quick Capture 모두 반영 (FOUC 없음, 리로드 없음)
- 재시작 후에도 선택 유지
- 10개 프리셋 전부에서 텍스트 가독성 OK (수동 검증)
- 단위·컴포넌트 테스트 그린, 기존 테스트 회귀 없음
