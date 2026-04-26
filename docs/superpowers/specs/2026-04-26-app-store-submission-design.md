# Hearth 1.0 — App Store Connect 제출 설계 (D)

- **작성일**: 2026-04-26 (Sprint Day 0)
- **서브프로젝트**: D (출시 sprint 8개 중 네 번째)
- **선행**: A (MAS 적합성), B (IAP + 라이선스), E (랜딩 lite)
- **일정**: Sprint Day 14-15 압축 실행 (메타 입력 + 빌드 업로드 + Submit) → Day 17-20 심사 → Day 21 (2026-05-17) Manual Release
- **출시 채널**: Mac App Store 단독

## 1. Scope

D는 "스펙·코드를 만든다"보다 **"App Store Connect에 정확한 데이터를 정확한 순서로 입력하고, 심사 통과 후 마케팅과 동기화하여 출시한다"**가 본질. 다른 서브프로젝트(A·B·E·F) 산출물에 의존성이 많고, 자체 산출물은 주로 메타데이터·심사 문서·빌드 업로드 자동화.

본 스펙은 다음을 결정·정의한다:
- App Store Connect 메타데이터 패키지 (이름/subtitle/description/키워드/카테고리 등)
- 5장 정적 스크린샷 + App Preview 30초 영상 제작 파이프라인
- App Privacy 신고 답변 + 개인정보처리방침 본문
- App Review Notes 본문
- Day 14-15 분 단위 제출 워크플로
- Day 21 Manual Release 의식

## 2. 확정된 5대 결정 (사용자 승인)

| # | 결정 | 선택 | 근거 요지 |
|---|------|------|----------|
| Q1 | 카테고리 | **Primary: Productivity / Secondary: 없음** | Things/Bear/Notion 직접 경쟁 풀, Secondary는 정체성 흐림 방지 |
| Q2 | 키워드 전략 | **하이브리드 (하이볼륨 2-3 + 롱테일 5-7)** | 100% 롱테일 = 노출 0, 100% 하이볼륨 = ranking 0 |
| Q3 | 스크린샷 | **5장 + UI + 카피 오버레이 (B형)** + App Preview 영상 (HF) | 첫 3장이 결정적, 영상은 전환율 +15-25% |
| Q4 | App Privacy | **Data Not Collected** | 자체 서버 0, 마케팅 가치 + GDPR/CCPA 자동 충족 |
| Q5 | Review Notes + Release | **풀 노트 + Manual Release** | Reject 사이클 절감 + 마케팅 동기화 통제권 |
| Q6 | 핵심 차별점 부각 | **`hearth` skill + hearth-cli + 외부 DB write auto-refresh = "AI agent driveable workspace"** | 이미 1.0에 출시 완료된 인프라(see commit 9ff38f3, `src-tauri/cli/`, `skills/hearth/SKILL.md`). 무명 1.0의 가장 강한 차별점이라 description·subtitle·screenshot·App Preview 모두 이 시그널 중심으로 재설계 |

추가 결정:
- 도메인: `hearth.codewithgenie.com` (Privacy/Support/Marketing URL 모두 이 서브도메인 하위)
- 지원 이메일: `genie@codewithgenie.com`
- Copyright: `© 2026 위드지니` (개인 권리자: 장재현)
- 별도 약관 없음 (Apple Standard EULA 의존)
- 출시 시각: 2026-05-17 09:00 KST

## 3. 의존성 그래프

```
A (MAS 적합성) ─────┐
  ├ App ID         │
  ├ Provisioning   │
  └ Distribution   ├──┐
                   │  │
B (IAP + 라이선스) ──┤  │
  ├ Product ID    │  ├──> D (제출)
  └ Sandbox       │  │      ├ App Store Connect 데이터 입력
                  │  │      ├ Build 업로드
E (랜딩 lite) ─────┘  │      ├ Review submission
  ├ /privacy URL    │      └ Manual release on launch day
  └ /support URL    │
                    │
F (가격·포지셔닝) ───┘
  └ subtitle 카피
```

**핵심 함의:**
- D는 단독 진행 불가
- E의 `/privacy` 와 `/support` 는 Day 14 메타 입력 시점에 200 OK 필수 → **E lite의 Day 13 우선 처리 권장**
- B의 코드는 ASC 등록 자체엔 불필요. Product ID 문자열(`io.hearth.app.pro`)만 필요. B 머지(M2)는 D submit과 독립 일정
- F(가격·포지셔닝)는 D 안에서 흡수 — 별도 sprint day 없이 D 작업 중 결정

## 4. 메타데이터 패키지

### 4.1 텍스트 필드 (App Store Connect 입력값)

| 필드 | 한도 | en-US | ko-KR |
|------|------|-------|-------|
| App Name | 30자 | `Hearth` | `Hearth` |
| Subtitle | 30자 | `Local-first AI agent workspace` (30자) | `로컬 AI 에이전트 워크스페이스` (14자) |
| Promotional Text | 170자 | (출시 후 자유 변경, 1.0은 동일) | (동일) |
| Description | 4000자 | §4.2 본문 | §4.2 본문 |
| Keywords | 100자 | §4.3 단일 라인 | §4.3 단일 라인 |
| Support URL | URL | `https://hearth.codewithgenie.com/support` | (동일) |
| Marketing URL (옵션) | URL | `https://hearth.codewithgenie.com` | (동일) |
| Privacy Policy URL | URL 필수 | `https://hearth.codewithgenie.com/privacy` | (동일) |
| Copyright | 한 줄 | `© 2026 위드지니` | (동일) |
| Age Rating | 설문 | 4+ (모든 항목 None/No) | (동일) |
| What's New (1.0) | 자유 | `Hearth 1.0 첫 출시. Mac App Store 단독 채널, 단일 비소모성 IAP 라이선스.` | (동일) |

### 4.2 Description 본문

**en-US (`docs/superpowers/app-store/description-en-US.md`):**

```
Hearth is a local-first personal workspace for projects, memos, and schedules.
All your data lives in a SQLite file on your Mac — never on a remote server.

Press ⌘K to open the AI command palette: "Add a dentist appointment for
Wednesday at 3pm" or "Find that note about React 19 hooks." Hearth's AI
turns natural language into actual changes via tool calling.

Press ⌃⇧H from any app to capture a thought in one second. The Quick Capture
overlay floats above whatever you're doing — Enter to save, Esc to cancel.

— Unified workspace: projects, memos, and schedules in one place
— ⌘K command palette: AI handles natural language requests
— ⌃⇧H Quick Capture: save anything from any app
— ⌘F instant search across everything
— Privacy-first: all data stays on your Mac
— Optional AI: works fully without an API key

DRIVE HEARTH FROM YOUR AI AGENT
Hearth ships with a `hearth` skill for Claude Code and Codex. Tell your AI
agent "summarize today's PR work into a new project with a review meeting
tomorrow at 3pm" — your agent calls the bundled `hearth` CLI, the database
updates, and Hearth's open tabs refresh in real time. Your AI workspace,
finally drivable by AI.

Buy Hearth Pro to unlock unlimited use after a 14-day evaluation period.
Family Sharing supported. Restore your purchase on any of your Macs.
```

의도적 카피 선택:
- "**14-day evaluation period**" — Q5(R1) 정책 준수, "free trial" 회피
- "Buy Hearth Pro to unlock unlimited use" — IAP 명시 (Apple 가이드라인 권장)
- "Family Sharing supported" — Q3 ON 결정 마케팅 활용
- "Optional AI: works fully without an API key" — 무명 1.0 신뢰 시그널 + AI 키 없는 사용자 진입 장벽 제거

**ko-KR (`docs/superpowers/app-store/description-ko-KR.md`):**

```
Hearth는 프로젝트·메모·일정을 한 곳에서 관리하는 로컬 퍼스트 데스크톱 앱입니다.
모든 데이터는 Mac의 SQLite 파일에 저장되며, 외부 서버로 전송되지 않습니다.

⌘K 한 번이면 AI 커맨드 팔레트가 열립니다. "수요일 오후 3시에 치과 예약 추가해줘"
또는 "React 19 훅 관련 메모 찾아줘" — Hearth AI가 자연어를 실제 작업으로 바꿉니다.

다른 앱에서도 ⌃⇧H 한 번이면 Quick Capture 오버레이가 떠서 1초 메모.

— 통합 워크스페이스: 프로젝트·메모·일정 한 곳에서
— ⌘K 커맨드 팔레트: AI가 자연어 요청 처리
— ⌃⇧H Quick Capture: 어디서든 즉시 저장
— ⌘F 전체 검색: 모든 데이터 즉시 찾기
— 프라이버시 우선: 모든 데이터는 내 Mac에
— AI는 선택: 키 없이도 모든 기능 정상 동작

AI 에이전트가 직접 조작하는 워크스페이스
Hearth는 Claude Code와 Codex용 `hearth` 스킬을 함께 제공합니다. AI 에이전트에게
"오늘 작업한 PR 내용 새 프로젝트로 정리하고 내일 오후 3시 리뷰 회의 잡아줘"라고
말하면, 에이전트가 내장 `hearth` CLI를 호출해 DB를 업데이트하고, 열려 있는 Hearth
탭이 실시간으로 새로고침됩니다. AI 워크스페이스를 진짜로 AI가 조작합니다.

Hearth Pro 구매로 14일 사용 기간 후에도 무제한 이용. 가족 공유 지원, 구매한 모든 Mac에서 복원 가능.
```

### 4.3 Keywords (100자 단일 라인)

**중요한 규칙:**
- 100자 한도, 쉼표로 구분, 공백 없이
- 앱 이름·subtitle·카테고리는 이미 인덱싱 → 중복 제외
- Subtitle "Local-first AI agent workspace"가 `local`, `first`, `ai`, `agent`, `workspace`를 인덱싱하므로 키워드에서 제외
- 단어 토큰화 — `brain` 단독으로 "second brain" 검색에 매칭됨

**en-US (`docs/superpowers/app-store/keywords-en-US.txt`):**
```
todo,notes,memo,tasks,calendar,planner,brain,palette,capture,sticky,unified,daily,cli,automation
```
글자 수: 96/100. (subtitle이 `local`/`first`/`ai`/`agent`/`workspace`를 흡수했으므로 그 자리에 `cli`/`automation` 등 핵심 차별점 키워드 배치)

**ko-KR (`docs/superpowers/app-store/keywords-ko-KR.txt`):**
```
할일,메모,일정,캘린더,투두,노트,세컨드브레인,명령팔레트,빠른메모,퀵캡처,시간관리,일정관리,데일리플래너,스티키메모,자동화,프로젝트관리,개발자도구,바이브코딩
```
글자 수: 86/100. (subtitle이 `로컬`/`AI`/`에이전트`/`워크스페이스`를 흡수했으므로 그 자리에 `자동화`/`개발자도구`/`바이브코딩` 등 핵심 차별점 키워드 배치)

**subtitle 중복 회피 (en + ko 공통)**: subtitle이 인덱싱하는 단어들(`local`, `first`, `ai`, `agent`, `workspace` / `로컬`, `AI`, `에이전트`, `워크스페이스`)은 키워드에서 의도적 제외하여 100자 효율 극대화.

### 4.4 In-App Purchase 메타

| 필드 | 값 |
|------|-----|
| Type | Non-Consumable |
| Reference Name | `Hearth Pro` |
| Product ID | `io.hearth.app.pro` |
| Price | USD $14.99 (Tier 15)<br>* 가격대는 환율 변동·App Store tier 시스템 변경에 따라 정확 수치는 입력 시점 확인 |
| Family Sharing | ON |
| Display Name (en) | `Hearth Pro` |
| Display Name (ko) | `Hearth Pro` (영문 그대로 음차) |
| Description (en) | `Unlock unlimited use of Hearth after the 14-day evaluation period.` |
| Description (ko) | `Hearth 14일 사용 기간 후에도 무제한으로 이용하세요.` |
| Review Screenshot | B 작업으로 생성된 PaywallModal 캡처 (1024×768 이상) |
| Review Notes | `This is a one-time purchase that unlocks the app beyond the 14-day evaluation period. Test with the sandbox account provided in the app review notes.` |

### 4.5 산출물 파일 구조

```
docs/superpowers/app-store/
├── README.md                      # 입력 순서·체크리스트
├── metadata-en-US.md              # 위 표 + en 본문 종합
├── metadata-ko-KR.md              # 동일 ko 버전
├── description-en-US.md           # 4000자 description (4.2)
├── description-ko-KR.md           # 동일 ko
├── keywords-en-US.txt             # 100자 단일 라인 (4.3)
├── keywords-ko-KR.txt             # 동일 ko
├── iap-product.md                 # IAP 메타 (4.4)
├── privacy-policy.md              # §6 본문
├── review-notes.md                # §7 본문
├── what-is-new-1.0.md             # release notes
└── screenshots/                   # §5 산출물
    ├── template.fig               # Figma 템플릿
    ├── screenshot-en-US-01-unified-workspace.png
    ├── screenshot-en-US-02-ai-palette.png
    ├── ... (총 10장)
    └── app-preview/
        ├── composition.html       # HyperFrames source
        └── app-preview-30s.mp4    # 렌더 결과
```

## 5. 시각 자산 제작 파이프라인

### 5.1 정적 스크린샷 5장 (Figma)

**산출물 사양:**
- 해상도: 2880 × 1800 px (Retina 16:10), PNG
- 장수: 5장 × 2 locale = **총 10장**
- 파일명: `screenshot-{locale}-{nn}-{slug}.png`
- 저장: `docs/superpowers/app-store/screenshots/`

**5컷 컨셉:**

| # | 카피 (ko / en) | UI 내용 | 핵심 가치 |
|---|---------------|---------|----------|
| 1 | "프로젝트·메모·일정을 한 곳에서" / "Projects, memos, schedules — one place" | 메인 화면 (탭 3개 visible, 더미 데이터 풍성) | 통합 workspace |
| 2 | "⌘K 한 번이면 AI가 다 한다" / "⌘K — your AI command palette" | 커맨드 팔레트 열린 상태 + AI 응답 in progress | AI 차별점 |
| 3 | "⌃⇧H 어디서든 1초 메모" / "⌃⇧H — capture from anywhere" | Quick Capture 오버레이 + 다른 앱 위에 떠있는 모습 | Quick Capture |
| 4 | "AI 에이전트가 내 워크스페이스를 조작한다" / "Your AI agent drives your workspace" | **분할 컷**. 좌: Claude Code 터미널에서 "오늘 PR 정리해서 새 프로젝트로 묶어줘" 입력 → `hearth` skill이 hearth-cli 호출하는 로그. 우: Hearth 앱 화면에서 새 프로젝트가 실시간으로 추가되는 모습 (앰버 글로우로 새 카드 강조) | **AI agent driveable workspace 차별점** (Q6 결정 핵심) |
| 5 | "⌘F 즉시 검색·바로 이동" / "⌘F — find anything, jump there" | 검색 결과 + 앰버 글로우 펄스 highlight | 전체 검색 UX |

**컷 4 제작 노트**: 좌측 터미널은 macOS Terminal.app (다크 테마, 폰트는 SF Mono 또는 JetBrains Mono), 우측 Hearth 앱은 동일 다크 테마. 두 윈도우 사이에 화살표/연결선으로 "CC → Hearth 실시간 반영" 흐름 시각화. 로컬 퍼스트 시그널(이전 컷 4의 핵심)은 subtitle "Local-first AI agent workspace" + description bullet "Privacy-first: all data stays on your Mac"으로 흡수.

**디자인 가이드:**
- 배경: 다크 단색 `#1a1614` (deep gray)
- 카피 영역(상단 30%): Pretendard Bold or SF Pro Display Bold, 120-140pt, 색상 `#f5e6c8` (warm cream)
- UI 영역(하단 70%): 실제 스크린샷 (mockup frame 없이 raw window)
- 액센트 색상: `#d97706` (amber)

**제작 4단계 (Day 14 안에 완결, 총 ~5h):**

1. **Figma 템플릿** (1h) — 2880×1800 프레임 1개, 카피 레이어 + 스크린샷 슬롯
2. **더미 데이터 시드 스크립트** (1h) — `scripts/seed-screenshots.ts`. 깨끗한 SQLite에 프로젝트 5개·메모 ~12개·일정 ~8개 (en/ko 분리). 개인정보·실제 이메일·저작권 위반 콘텐츠 0
3. **dev 빌드 캡처** (2h) — 더미 데이터 시드 후 Cmd+Shift+4+Spacebar로 윈도우 캡처 (alpha 보존). en + ko 각 5컷
4. **Figma 합성 + Export** (1h) — 카피 텍스트 layer 10조합 export, 파일명 규칙 적용

**검수 체크리스트 (업로드 전):**
- [ ] 모든 컷 2880×1800 정확히 (다른 해상도 reject)
- [ ] 첫 3장 = 메인화면 / ⌘K AI / ⌃⇧H Quick Capture (검색 결과 카드 노출용 핵심)
- [ ] 카피 텍스트에 오타 없음 (en + ko 각 5개)
- [ ] 스크린샷 안에 실제 개인정보·외부 저작권 콘텐츠 0
- [ ] Hearth UI 안 모든 데이터가 더미 (실 사용 데이터 누출 0)
- [ ] 다크 모드 일관성 (라이트 모드 캡처 X)
- [ ] 윈도우 그림자 자연스러움 (alpha 보존)
- [ ] AI 컷의 응답 텍스트가 hand-crafted dummy ("OpenAI" 로고·copyrighted text 회피)

### 5.2 App Preview 영상 30초 (HyperFrames)

**사양:**
- 길이: 30초 (Apple 한도 15-30초)
- 해상도: 1920×1080 (또는 1920×1200), H.264 mp4
- Poster Frame: 영상 시작점

**컴포지션 (6컷 흐름, 30초 안에 압축):**

```
[0:00-0:04]  Hearth 로고 페이드인 + 카피 "Local-first AI agent workspace"
[0:04-0:09]  메인 화면 (탭 3개 데이터 풍성)
             카피: "Projects, memos, schedules — one place"
[0:09-0:14]  ⌘K 팔레트 → 사용자 타이핑 → AI 응답 → 실제 일정 추가
             카피: "⌘K — AI command palette"
[0:14-0:18]  ⌃⇧H Quick Capture 오버레이 → Enter → 메모 저장
             카피: "⌃⇧H — capture from anywhere"
[0:18-0:25]  ⭐ HERO MOMENT — Claude Code 터미널 ↔ Hearth 앱 분할 화면.
             좌: CC에 "오늘 PR 정리해서 프로젝트로 묶어줘" 입력 → `hearth` skill 작동
             우: Hearth 앱에서 새 프로젝트·메모·일정이 한 줄씩 실시간으로 추가
             카피: "Your AI agent drives your workspace"
             (이 컷에 7초 — 가장 차별화된 모먼트라 시청자가 한 번 더 보게 시간 줌)
[0:25-0:28]  ⌘F 검색 → 결과 카드 → 클릭 → 앰버 글로우 펄스
             카피: "⌘F — find anything"
[0:28-0:30]  CTA 카피 "Buy Hearth Pro $14.99" + Hearth 로고
```

**제작 도구**: HyperFrames (HTML 애니메이션 → mp4 렌더)
- Source: `docs/superpowers/app-store/screenshots/app-preview/composition.html`
- 렌더 명령: `hyperframes render` (hyperframes-cli)
- 산출물: `app-preview-30s.mp4`

**HF 컴포지션 작성 시 주의:**
- 정적 스크린샷 5장을 그대로 base 이미지로 활용 → 카피 동기화 + scene transition
- 각 컷에서 UI 일부(커서·키 입력·하이라이트)를 GSAP으로 미세 애니메이션
- 음성·BGM 미사용 (Apple App Preview는 무성도 OK, 무성으로 다국어 부담 제거)
- 자막 inline 렌더 (별도 SRT 불필요)

### 5.3 시각 자산 제작 일정

| 작업 | 시간 | 위치 |
|------|------|------|
| DB 백업 (`~/.hearth-backup-pre-screenshots-20260426/`) | 5min | **이미 완료 (Day 0)** |
| 더미 데이터 시드 스크립트 | 1h | Day 14 오전 |
| Figma 템플릿 | 1h | Day 14 오전 |
| dev 빌드 캡처 (en + ko) | 2h | Day 14 오후 |
| 정적 스크린샷 합성 + export | 1h | Day 14 오후 |
| HF 컴포지션 작성 | 2h | Day 14 오후 |
| HF mp4 렌더 | 30min | Day 14 오후 |
| **합계** | **~7.5h** | **Day 14 안에 끝** |

## 6. App Privacy + 개인정보처리방침

### 6.1 App Store Connect Privacy 신고

| 질문 | 답변 |
|------|------|
| Do you or your third-party partners collect data from this app? | **No** |
| (그 외 모든 follow-up) | 자동 N/A |

**결과**: 앱 페이지에 라벨 "**Data Not Collected**" 노출.

### 6.2 Privacy Policy 본문 (en)

`docs/superpowers/app-store/privacy-policy.md` (en + ko 1페이지 동시 노출):

```markdown
# Privacy Policy

Last updated: 2026-05-17

## 1. Summary

Hearth does not collect any personal data. All your data — projects, memos,
schedules, settings — is stored in a SQLite database on your Mac. Nothing
is sent to a Hearth server, because there is no Hearth server.

## 2. Data we DO NOT collect

- We do not collect analytics, telemetry, or usage statistics.
- We do not collect crash reports.
- We do not collect your IP address, device ID, or any identifier.
- We do not have user accounts. There is no sign-up.

## 3. Data that stays on your Mac

All app data is stored at:
`~/Library/Application Support/com.newturn2017.hearth/data.db`

You can inspect, back up, copy, or delete this file at any time.

## 4. Optional integrations

If you choose to enable optional integrations, those services receive data
according to their own privacy policies. Hearth never reads or stores the
data exchanged with these services beyond what is required to display the
result back to you.

- **OpenAI** (when you provide your own API key in Settings):
  Hearth sends your command palette prompts to OpenAI's API.
  See https://openai.com/policies/privacy-policy
- **External calendar providers** (Google Calendar, etc., if you connect):
  Hearth reads/writes events via OAuth. Tokens are stored in macOS Keychain.

You can disconnect these integrations at any time in Settings → Integrations.
Disconnecting works even after your trial period ends.

## 5. Payments

Purchases are processed by Apple via the Mac App Store. Hearth never sees
your payment information. License verification happens entirely on your
device using Apple StoreKit 2; no purchase data is transmitted to a Hearth
server.

## 6. Family Sharing

If you purchase Hearth Pro, your purchase is shared with members of your
Family Sharing group, per Apple's standard policy. Hearth does not see who
those members are.

## 7. Children

Hearth is rated 4+. We do not knowingly collect data from anyone, including
children, because we do not collect data at all.

## 8. Changes to this policy

If we ever change this policy, we will update the date at the top and post
a notice in the app's release notes.

## 9. Contact

Questions about this policy: genie@codewithgenie.com
```

**ko 본문**: 위 영문의 1:1 한국어 번역, 동일 9개 섹션. (E 랜딩 빌드 시 본 markdown을 import하여 두 언어 동시 렌더)

### 6.3 앱 내 표기 (필수)

Settings 추가 작업 (B의 Settings → 라이선스 섹션 옆에 배치):
- "개인정보처리방침" 링크 → `https://hearth.codewithgenie.com/privacy` (외부 브라우저)
- "오픈소스 라이선스" 링크 → 앱 내 라이선스 화면 (npm + cargo 의존성 list)
- 버전 표시 + 빌드 번호

**구현 비용**: ~30분 작업. D 스펙 안에서 처리 (별도 서브프로젝트 없이).

**추가 (Q6 결정)**: Settings → Integrations 섹션에 `hearth` skill 안내 카드 추가:
- 카피: "Drive Hearth from your AI agent. Install the `hearth` skill for Claude Code or Codex."
- "View installation instructions" 버튼 → 외부 브라우저로 hearth-cli OSS repo README 열기
- 이미 설치된 경우 (`~/.claude/skills/hearth` 또는 `~/.codex/skills/hearth` 심링크 존재) → "Installed ✓" 배지
- 구현 비용: ~1h. D 스코프 안에서 처리 가능

### 6.4 GDPR / CCPA / 개인정보보호법 대응

- **GDPR**: Data Not Collected → controller/processor 의무 거의 없음. Privacy Policy의 "we do not collect" 명시로 사실상 충족
- **CCPA**: 동일 논리. "Do Not Sell My Personal Information" 토글 불필요
- **국내(개인정보보호법)**: ko 버전에 "수집하는 개인정보 항목: 없음" 명시로 충족

### 6.5 1.0 이후 정책 안전장치 (영구 박제)

향후 어떤 사유로든(텔레메트리 추가 등) "Data Not Collected" 라벨이 깨지는 변경을 가하려면:
1. 사전 메이저 버전 업데이트(1.x → 2.0)에서만 허용
2. 사용자 opt-in toggle 필수 (기본값 off)
3. 출시 1주일 전 in-app 공지 + privacy policy 업데이트

이 안전장치는 본 스펙의 정책으로 박제하여 미래 의사결정을 묶음. 무명 1.0의 "Data Not Collected" 시그널은 가장 강한 신뢰 자산.

## 7. App Review Notes 본문 (확정)

`docs/superpowers/app-store/review-notes.md`:

```
=== Review Notes for Hearth 1.0 ===

Thank you for reviewing Hearth.

1. SANDBOX TESTER ACCOUNT (for In-App Purchase testing)
   Email:    [Day 15 오전 발급된 sandbox tester]
   Password: [발급된 비밀번호]
   Region:   United States

   To test purchases:
   - macOS System Settings → Apple ID → Media & Purchases → Sandbox Account
   - Sign in with the credentials above

2. APP OVERVIEW
   Hearth is a local-first personal workspace combining projects, memos,
   and schedules with an AI command palette. All user data is stored in
   a local SQLite database (~/Library/Application Support/com.newturn2017.hearth/).
   No user data is transmitted to any Hearth server (we operate no server).

3. KEY FLOWS TO TEST
   a) Trial entry:
      - Launch app → 14-day evaluation starts automatically
      - Trial countdown visible in top-right corner

   b) Purchase Hearth Pro:
      - Settings → License → "Buy Hearth Pro" button
      - Sandbox payment dialog appears → Confirm
      - Status changes to "Purchased" → all features unlock permanently

   c) Restore Purchase:
      - Settings → License → "Restore Purchase" button
      - For testing on a fresh install or a different Mac

   d) Read-only mode (when trial expires):
      - In sandbox builds, Settings → Debug → "Force trial expiry" expires
        the trial without waiting 14 days
      - All read paths remain functional; mutation UI is disabled
      - Banner at top with "Buy Hearth Pro" CTA

   e) Quick Capture: Press ⌃⇧H from any app

4. PRE-EMPTIVE COMPLIANCE NOTES
   - Trial copy uses "14-day evaluation period" not "free trial" per
     guideline 3.1.2(b) for non-consumable IAP.
   - Read-only mode preserves data access (export/copy/disconnect
     always work) per data accessibility expectations.
   - Restore Purchase button is permanently visible in Settings.
   - Family Sharing entitlement is honored (ownership_type=FAMILY_SHARED
     branch tested with Sandbox family group).
   - Paywall opens only on user mutation attempt, never automatically.

5. OPTIONAL FEATURES (not required for review)
   - OpenAI integration: requires user-provided API key. Skip for review;
     core app functions fully without it.
   - Google Calendar OAuth: requires user account. Skip for review.
   - Bundled `hearth` skill for Claude Code / Codex CLI agents:
     installable via `./scripts/install-skills.sh` from the open-source
     `hearth-cli` repository. Allows external AI agents to drive the app
     via a CLI surface. Database changes appear in the running app in
     real time. Not exercised by the review build (the CLI binary is
     distributed separately as OSS, not bundled inside the MAS package).
   All optional integrations are opt-in via Settings → Integrations.

6. PRIVACY
   This app collects no data. Privacy Label answer: "Data Not Collected".
   Privacy Policy: https://hearth.codewithgenie.com/privacy

7. CONTACT
   Developer: 장재현 / 위드지니
   Email: genie@codewithgenie.com
   Response time: within 24 hours (KST timezone)
```

**주의:**
- "Force trial expiry" 디버그 메뉴는 sandbox 빌드 한정 (production에는 비표시). B의 D11/D12 작업 중 추가 권장 — 미구현 시 Review Notes에서 해당 줄 제거하고 "trial naturally expires after 14 days, please advance system clock or use the test transaction simulator" 안내로 대체

## 8. Day-by-day 워크플로

### Day 14 — 메타데이터 입력 + 빌드 업로드

**오전 (3-4h):**

```
[09:00] App Store Connect 진입
[09:15] My Apps → "+" → New App
        - Platform: macOS
        - Name: Hearth
        - Primary Language: English (U.S.)
        - Bundle ID: com.newturn2017.hearth (드롭다운에서 A 등록한 것 선택)
        - SKU: hearth-1-mas
        - User Access: Full Access

[09:30] App Information 탭
        - Subtitle: §4.1 표 그대로
        - Category Primary: Productivity / Secondary: 없음
        - Content Rights: No
        - Age Rating: 4+ (모든 항목 None/No)

[10:00] Pricing and Availability 탭
        - Price: USD $14.99 (Tier 15)
        - Availability: All territories
        - Pre-orders: Off

[10:30] In-App Purchases 탭
        - §4.4 표 입력
        - Status: "Ready to Submit"

[11:00] Version 1.0 (좌측 사이드바)
        - Description: docs/superpowers/app-store/description-en-US.md 복붙
        - Keywords: docs/superpowers/app-store/keywords-en-US.txt 복붙
        - Support URL: https://hearth.codewithgenie.com/support
        - Marketing URL: https://hearth.codewithgenie.com
        - Privacy Policy URL: https://hearth.codewithgenie.com/privacy
        - Copyright: © 2026 위드지니
        - What's New: docs/superpowers/app-store/what-is-new-1.0.md

[11:30] Localization 추가 (Korean)
        - 위 모든 텍스트 필드 ko 버전 복붙

[12:00] App Privacy 섹션
        - "Data Not Collected" 단일 답변
```

**오후 (3-4h):**

```
[13:00] 스크린샷 + App Preview 업로드
        - Mac App Store screenshots → 5장 × 2 locale = 10장
        - 순서: 1) main view, 2) ⌘K AI, 3) ⌃⇧H Quick Capture, 4) Local SQLite, 5) ⌘F search
        - App Preview: app-preview-30s.mp4

[14:00] 빌드 업로드 (B의 build-mas.sh + upload-mas.sh)
        $ ./scripts/build-mas.sh
        $ ./scripts/upload-mas.sh
        Apple 빌드 처리 30min-2h 대기 (이메일 알림)

[15:00-17:00] 빌드 처리 대기 활용:
        - Review Notes 본문 최종 작성 (§7)
        - Sandbox Tester 계정 발급 (Users and Access → Sandbox)
        - test_log.md (B 산출물) 최종 정리
        - 메타 입력 검수 1회

[17:30] 빌드 처리 완료 → Version 1.0 → "Build" 섹션에서 1.0(빌드번호) 선택
```

### Day 15 — Review Notes + Submit

**오전 (1-2h):**

```
[09:00] App Review Information 섹션
        - Sign-In required: No
        - Contact: 장재현 / [전화] / genie@codewithgenie.com
        - Notes: §7 본문 복붙
        - Attachment: 일반적으로 불필요 (App Preview 있음)

[09:30] Version Release 섹션
        - "Manually release this version" 선택

[09:45] 최종 검수 sweep:
        - 모든 메타 필드 채워짐
        - Privacy Policy URL 클릭 → 정상 오픈
        - Support URL 클릭 → 정상 오픈
        - 스크린샷 첫 3장 의도된 순서
        - IAP product Status = "Ready to Submit"

[10:00] "Submit for Review" 버튼 클릭
        Status: Waiting for Review

[10:05] 출시 보류 모드 → Day 17-20 심사 윈도우 시작
```

### Day 16-20 — 심사 대기 + 우발 대응

심사 평균 24-48h, 최대 7일. 대기 중 다른 sprint 작업 진행 (E·F·G).

**Reject 대응 매트릭스:**

| Reject 사유 | 대응 |
|------------|------|
| Metadata reject (텍스트만 변경 필요) | Resolution Center에서 답변 + 즉시 재제출, 재심사 6-24h |
| Build reject (코드 수정 필요) | 코드 수정 → 새 빌드 업로드 → 재제출, 재심사 24-48h |
| Guideline 3.1.2(b) "trial" 문구 challenge | Review Notes §4 답변 인용 + 카피 위치 명시 |
| IAP product missing | Day 14 [10:30] 단계 누락 점검, 재등록 |
| Privacy Policy URL 404 | E 랜딩 status 확인 + 임시로 raw GitHub Pages URL 교체 |
| 데이터 export 부재 reject | B에서 검증된 export 경로 데모 영상 첨부 답변 |

### Day 21 (2026-05-17) — Manual Release 의식

**07:00 KST**: 심사 통과 알림 수신 가정

**Pre-launch 체크리스트 (07:00-09:00):**
- [ ] E 랜딩 `hearth.codewithgenie.com` 200 OK + 결제 버튼 → App Store 링크 정상
- [ ] E 랜딩 `/privacy` `/support` 200 OK
- [ ] G 캠페인 게시물 준비됨 (HN Show / Twitter / Reddit r/macapps / Korean tech Slack)
- [ ] 스크린샷·App Preview 정상 노출 확인 (App Store Connect 미리보기)
- [ ] B의 정상 결제 흐름 1회 sandbox 재검증 (출시 빌드와 동일 코드)
- [ ] 지원 이메일 받은편지함 모니터링 준비

**09:00 KST**: App Store Connect → Version 1.0 → "Release this version" 클릭

**09:00-12:00**: Apple CDN 전 세계 전파 (1-3시간)

**12:00 (예상)**: 전 세계 App Store에 Hearth 1.0 노출 → G 캠페인 트리거 시작

## 9. 의존성 매트릭스 (Day 14 시작 직전 사전 조건)

| 의존 | 충족 시점 | 상태 (2026-04-26 기준) |
|------|----------|------------------------|
| A: App ID + 인증서 + Provisioning Profile | A의 Day 1-7 완료 | 스펙 작성됨, 검토 대기 |
| B: Product ID 확정값 + 빌드 가능한 코드 | M2 머지 = ASC 셋업 후 = Day 14 후반 또는 Day 15 | T1-T33 완료 (worktree feat/iap-license HEAD b964b35), T34/T35/T39/T41 ASC 의존 미완 |
| E: /privacy + /support + / 모두 200 OK | E lite 의 Day 13 완료 권장 | 스펙 미작성 |
| F: subtitle/description 카피 확정 | D 안에 흡수 | 본 스펙에서 결정 ✓ |
| 지원 이메일 genie@codewithgenie.com | **이미 보유 ✓** | ✓ |
| 도메인 hearth.codewithgenie.com | **이미 보유 ✓** | ✓ |
| Apple Developer Program 가입 | Day 0 이전 | ✓ |
| Sandbox Tester 계정 | Day 14 오후 또는 Day 15 오전 | TBD |

⚠️ **B 머지 전략 (M2)**: B의 라이선스 게이트는 출시 후 되돌리기 어려움 → ASC 셋업 후 T34/T35/T39/T41 통과 후 머지. 머지 시점 = Day 15 후반 또는 Day 16 초반 권장. main 브랜치는 D submit 시점까지 docs-only 상태 유지.

## 10. 검수 체크리스트 (Day 15 [09:45] 단계 입력)

- [ ] 메타데이터: en-US + ko-KR 모든 필드 채워짐
- [ ] Subtitle: 30자 한도 초과 없음
- [ ] Description: 4000자 한도 초과 없음
- [ ] Keywords: 100자 한도 초과 없음, 공백 없음
- [ ] Support / Marketing / Privacy URL 3개 모두 클릭 → 정상 오픈
- [ ] App Privacy: "Data Not Collected" 답변
- [ ] IAP product `io.hearth.app.pro` Status = "Ready to Submit"
- [ ] Family Sharing ON
- [ ] Price USD $14.99 (Tier 15)
- [ ] 스크린샷 5장 × 2 locale 업로드 (총 10장), 모두 2880×1800
- [ ] App Preview 30초 mp4 업로드
- [ ] Build 선택됨 (1.0 빌드번호 가시)
- [ ] App Review Information: Contact + Notes 채워짐
- [ ] Sandbox Tester 계정 정보 Notes에 포함
- [ ] Version Release: "Manually release this version" 선택
- [ ] Copyright: `© 2026 위드지니`
- [ ] Age Rating 4+
- [ ] Category: Primary Productivity / Secondary 없음

## 11. Out of Scope (1.x)

- TestFlight 베타 — 1.0은 직접 production 빌드 제출. 베타 풀은 1.1부터
- Phased release(7일 점진 노출) — 1.0은 100% 즉시. 무명이라 천천히 풀 의미 없음
- 다국어 메타 추가(en/ko 외) — 1.0은 2개 locale 고정. 일본어/중국어 등은 1.1+
- macOS Universal Purchase(iOS 동반 구매) — Hearth는 Mac 전용
- iPad/iPhone 버전 — 별도 product 등록 필요, 1.0 스코프 외
- Pre-order — 무명 1.0엔 의미 없음
- 자체 결제 (App Store 외부) — MAS-only 채널 결정(D5)

## 12. 영향 파일 인벤토리

**신규 생성:**
- `docs/superpowers/app-store/` 디렉토리 전체 (구조는 §4.5)
- `scripts/seed-screenshots.ts` (시드 스크립트)

**수정:**
- `src/Settings.tsx` — 개인정보처리방침/오픈소스 라이선스/버전 정보 섹션 추가 (~30분 작업)
- `src/Settings.tsx` — Integrations 섹션에 `hearth` skill 카드 추가 (~1h, Q6 반영). 카드 본문: "Drive Hearth from your AI agent. Install the `hearth` skill for Claude Code or Codex." + 설치 안내 외부 링크 + 심링크 존재 시 "Installed ✓" 배지
- `package.json` 또는 `Cargo.toml` — 변경 없음 (D는 메타·문서·빌드 산출물 위주)

**B 의존 작업:**
- 빌드 스크립트 `scripts/build-mas.sh` + `scripts/upload-mas.sh` — A 스펙에 정의됨, B 작업으로 검증
- Sandbox 디버그 메뉴 "Force trial expiry" — B의 D11/D12 작업 중 추가 권장. 미구현 시 Review Notes 카피 조정

## 13. 사용자 액션 항목 (외부 작업, Claude Code로 수행 불가)

D는 본질적으로 사용자(권리자)의 외부 작업이 많음. 본 스펙은 그 외부 작업의 입력값·순서를 명세하는 것이 1차 목적.

| Day | 작업 | 소요 |
|-----|------|------|
| Day 14 [09:00] | Apple Developer 로그인 → ASC 진입 | 5min |
| Day 14 [10:30] | IAP product 등록 (sandbox 사용 위해 빌드 업로드 전 가능) | 20min |
| Day 14 [13:00] | 스크린샷 10장 + App Preview mp4 업로드 | 30min |
| Day 14 [14:00] | 빌드 업로드 (`./scripts/build-mas.sh && ./scripts/upload-mas.sh`) | 5min + 빌드 시간 |
| Day 14 [17:30] | Sandbox Tester 계정 발급 | 5min |
| Day 15 [09:00] | Review Notes 입력 + Submit | 30min |
| Day 21 [09:00] | Manual Release 버튼 클릭 | 1min |

**총 사용자 외부 작업 시간**: ~2-3h (Day 14-15에 분산)
