# E · Landing lite — Design Spec

**Sprint:** Hearth 1.0 Mac App Store launch
**Day:** 1 (2026-04-27) of 21
**Launch target:** 2026-05-17
**Live deadline for this sub-project:** Day 13 (2026-05-09) — required by D §메타 입력 단계
**Domain:** `hearth.codewithgenie.com`

---

## 1. Purpose

App Store Connect 제출 단계에서 `Privacy Policy URL`과 `Marketing/Support URL`이 200 OK로 응답해야 한다. 동시에 1.0 핵심 차별점인 "AI 에이전트가 직접 조작 가능한 로컬 워크스페이스"(D §Q6)를 외부 진입점에서도 일관되게 노출한다. 한 번에 두 목적을 충족하는 5페이지 정적 사이트를 만든다.

본 스펙의 범위는 **D §7 review-notes에서 약속한 R9 아키텍처(외부 CLI = 1st-party companion tool, 별도 배포)의 실제 배포 인프라까지 포함**한다 — Homebrew tap, R2 바이너리 호스팅, 사이트 install 페이지가 모두 같은 약속의 구현이다.

## 2. Identity & tone

- **톤:** Quiet local-first. 차분한 색조(off-white `#fafaf7`, ink `#1a1814`), 큰 타이포그래피, 단일 컬럼 중심, 시각 노이즈 최소화.
- **메시지 우선순위:**
  1. "내 데이터, 내 컴퓨터" 안정감 (로컬 first-party 데이터)
  2. 개인 생산성 도구로서의 핵심 기능(Projects · Memos · Schedule)
  3. AI 에이전트 조작 가능 차별점 (CC↔Hearth) — 히어로 아래 두 번째 섹션에서 부연
- **대비:** App Preview 영상은 어두운 톤(D §4.5)이라 시각적 무게가 강하다. 사이트의 차분함과 영상의 임팩트가 의도된 대비를 이룬다.

## 3. Routes

10 정적 빌드(en × ko × 5):

| Path | Purpose | 콘텐츠 출처 |
|---|---|---|
| `/` | 히어로 + 차별점 섹션 + CTA | 신규 카피 (en/ko) |
| `/install/cli` | Homebrew install 명령 + 서명/notarize 안내 + 시스템 요구사항 | 신규 카피 |
| `/install/skill` | `hearth-cli install-skill` 안내 | 신규 카피 |
| `/privacy` | 프라이버시 정책 9개 섹션 | `docs/superpowers/app-store/privacy-policy.md` 1:1 재사용 |
| `/support` | 지원 이메일 + FAQ 5-6개 | 신규 작성 (FAQ 항목 §3.5) |

**Locale routing:**
- 디렉토리 기반 — `/en/...`, `/ko/...`. 베어 `/` 접근 시 `Accept-Language` 헤더 기반 1회 리다이렉트, 쿠키로 사용자 선택 기억(이후엔 강제 리다이렉트 없음).
- `<link rel="alternate" hreflang="en">`, `hreflang="ko"`, `hreflang="x-default"` 모든 페이지에 삽입.
- 기본 언어: **ko** (사용자 모국어). en은 동등 우선순위로 작성하되 카피 검수는 ko 우선.

### 3.1 `/` (홈) — 섹션 구성

1. **Hero** — 헤드라인 1줄 + 서브타이틀 1줄 + 2 CTA(Mac App Store, Install CLI). 배경 off-white, 단일 컬럼 정렬.
2. **Drive it from Claude Code** — 차별점 시연 섹션. 짧은 코드 블록(skills/hearth + hearth-cli 호출 예) + 우측 또는 아래 영역에 App Preview 영상 임베드. 톤은 어둡게 전환되어 영상과 자연 정렬.
3. **What's in Hearth** — Projects · Memos · Schedule 3-카드 미니 설명. 스크린샷 또는 아이콘 1개씩.
4. **Local-first, by design** — 데이터 위치(`~/Library/Application Support/com.newturn2017.hearth/data.db`), 비전송 정책, 백업·CLI 자유 접근 한 줄.
5. **CTA repeat + footer** — Mac App Store + Install CLI 반복. 푸터: Privacy / Support / GitHub(공개되면 추후) / © 2026.

### 3.2 `/install/cli`

- 헤드라인: "Install hearth-cli"
- 단계 1: `brew tap codewithgenie/hearth && brew install hearth-cli`
- 단계 2: 첫 실행 시 macOS Gatekeeper 통과 확인 (Developer ID 서명 + notarized → 자동 통과 예상)
- 시스템 요구사항: macOS 11+ (Hearth 앱과 동일)
- 제거: `brew uninstall hearth-cli && brew untap codewithgenie/hearth`
- "Hearth 앱이 먼저 설치되어 있어야 합니다" 명시 + Mac App Store 링크
- 검증 명령: `hearth-cli --version`

### 3.3 `/install/skill`

- 헤드라인: "Add the Hearth skill to Claude Code"
- 단계: `hearth-cli install-skill` 단일 명령
- 무엇이 일어나는지(투명성):
  - `~/.claude/skills/hearth/` 디렉토리 생성
  - skill.md + 보조 파일 압축 해제
  - 다음 Claude Code 세션부터 자동 인식
- 제거: `hearth-cli uninstall-skill` 또는 `rm -rf ~/.claude/skills/hearth`
- 사용 예 1줄: 클릭하지 말고 그냥 Claude Code에서 자연어로 "Hearth에 메모 남겨줘" 같은 식으로 호출.

### 3.4 `/privacy`

- D 메타에서 추출한 9개 섹션을 그대로 렌더. en/ko 페이지 각각 별도 본문(이미 D 메타에 1:1 번역 존재).
- 마지막 업데이트 일자(`Last updated`) 자동 표기 — 빌드 시점 또는 Markdown frontmatter `updated` 필드.
- 변경 시 ASC 메타와 동기화 의무가 있다는 운영 노트(스펙 §6 참조).

### 3.5 `/support`

- 헤드라인: "How can we help?"
- Primary contact: `support@codewithgenie.com` (요구 시 `hyuni2020@gmail.com`로 fallback — D 메타 review-notes와 정합)
- 응답 시간 안내: 영업일 기준 48시간 내 (보수적 약속)
- FAQ 6개 항목:
  1. **설치 후 첫 실행에서 NSOpenPanel이 나타나는데 왜요?** (A 스펙 §4-3 답변 — security-scoped bookmark 동의)
  2. **구매 영수증 복원은 어떻게 하나요?** (Settings > License > Restore Purchase, B 스펙 참조)
  3. **DB 파일은 어디에 있나요? 백업 가능한가요?** (`~/Library/Application Support/com.newturn2017.hearth/data.db`, Time Machine 자동 포함)
  4. **CLI/skill이 작동하지 않을 때 체크리스트** (앱 실행 여부, 권한, 최신 버전)
  5. **macOS 업그레이드 후 데이터가 사라졌어요** (Time Machine 복원 절차)
  6. **환불 정책** (Apple 표준 — App Store에서 직접 신청)

## 4. Tech stack

- **Framework:** Astro 5.x (현 안정 버전) — 정적 빌드 + Markdown content collections + i18n 라우팅 빌트인.
- **Styling:** Tailwind v4 또는 Astro scoped CSS (선택은 구현 단계에서). 둘 다 5페이지 규모에 부담 없음.
- **JS surface:** 최소 — locale switcher 1개 컴포넌트만. 나머지는 정적 HTML.
- **Markdown:** content collections로 `/privacy`, `/support` FAQ 본문, 그리고 추후 changelog(1.1+) 모두 동일 메커니즘으로 관리.
- **이미지:** Astro `<Image>` 컴포넌트로 자동 변환 + 사이즈 최적화. 히어로 영역 스크린샷, App Preview 영상 포스터.
- **영상 임베드:** App Preview는 자체 호스팅(R2)된 mp4 + `<video>` 태그 + 포스터 이미지 + lazy load.
- **Analytics:** **없음** (1.0 한정). 1.1+에서 Plausible 또는 Cloudflare Web Analytics 결정.

## 5. Repos & deployment

### 5.1 Repo structure

| Repo | Visibility | 역할 |
|---|---|---|
| `hearth` | **Private** | 메인 모노레포 — 앱(`src/`, `src-tauri/`) + 랜딩(`web/`) + CLI(`cli/` 또는 별도 폴더) |
| `homebrew-hearth` | **Public** | Homebrew tap. 안에는 formula `.rb` 하나만. CI가 자동 PR로 버전 업데이트. |

### 5.2 Site deployment (Cloudflare Pages)

- GitHub App 인증으로 private `hearth` 레포 연결 (OAuth 1회).
- 빌드 설정:
  - Root directory: `web/`
  - Build command: `pnpm install && pnpm build`
  - Output directory: `web/dist`
  - Node version: `20`
- 커스텀 도메인: `hearth.codewithgenie.com`
  - Hostinger DNS에 `CNAME hearth → <project>.pages.dev` 추가
  - Cloudflare Pages 측 도메인 검증(자동 SSL 발급)
- PR마다 preview URL 자동 발급 (`<branch>.<project>.pages.dev`).

### 5.3 CLI binary distribution

- **호스팅:** Cloudflare R2 버킷 `hearth-releases`.
  - 객체 키 패턴: `cli/hearth-cli-{version}-{arch}.tar.gz` (예: `cli/hearth-cli-1.0.0-arm64.tar.gz`, `cli/hearth-cli-1.0.0-x86_64.tar.gz`).
  - 공개 access (read-only). 별도 도메인 `dl.codewithgenie.com` 또는 R2 기본 도메인 사용.
- **서명:** Developer ID Application 인증서로 코드 서명 → `xcrun notarytool submit` 통과 → `xcrun stapler staple`. 결과물은 `.tar.gz`로 압축.
- **CI 워크플로** (메인 레포 `.github/workflows/release-cli.yml`):
  1. tag push 또는 수동 dispatch (`v1.0.0` 등)
  2. cargo build --release (arm64 + x86_64) 또는 universal binary 1개
  3. codesign + notarize + staple
  4. SHA256 계산
  5. R2 upload (rclone 또는 aws-cli with R2 endpoint)
  6. `homebrew-hearth` 레포에 formula 업데이트 PR 자동 생성 (`gh pr create`)
- **Skill 임베드:** CLI 바이너리에 `skills/hearth/` 디렉토리를 `include_dir!` 매크로 또는 동등 방식으로 컴파일타임 포함. `hearth-cli install-skill` 시 `~/.claude/skills/hearth/`로 복사.

### 5.4 Homebrew tap (`homebrew-hearth`)

- 단일 파일: `Formula/hearth-cli.rb`
- Formula 골격 (예시):

```ruby
class HearthCli < Formula
  desc "CLI companion for Hearth — drive your local workspace from Claude Code"
  homepage "https://hearth.codewithgenie.com"
  version "1.0.0"
  on_arm do
    url "https://dl.codewithgenie.com/cli/hearth-cli-1.0.0-arm64.tar.gz"
    sha256 "<sha>"
  end
  on_intel do
    url "https://dl.codewithgenie.com/cli/hearth-cli-1.0.0-x86_64.tar.gz"
    sha256 "<sha>"
  end
  def install
    bin.install "hearth-cli"
  end
  test do
    system "#{bin}/hearth-cli", "--version"
  end
end
```

- License: 메타데이터만 공개되므로 formula 자체에 라이선스 명시 불필요. 단, 레포 README에 "이 tap은 Hearth(상업 macOS 앱)의 CLI 동반자를 배포합니다. 소스는 비공개입니다." 한 줄.

## 6. 페이지별 핵심 카피 (시드)

본 섹션은 카피 작성 전용이며 구현 단계에서 카피라이터 또는 사용자가 다듬는다. 검수 우선순위는 ko > en.

### 6.1 `/` (ko)
- **Hero headline:** "로컬에 사는 AI 에이전트 워크스페이스"
- **Hero subtitle:** "Projects · Memos · Schedule, in one calm place."
- **Hero CTA:** [Mac App Store에서 받기] [CLI 설치하기]
- **Section 2 headline:** "Claude Code에서 직접 조작하세요"
- **Section 2 body:** "Hearth는 명령형 인터페이스를 제공합니다. 자연어로 시키면 `skills/hearth`가 `hearth-cli`를 호출하고, 열려 있는 Hearth는 즉시 새로고침됩니다. 컨텍스트 전환 없이, 에이전트가 당신의 워크스페이스를 손에 쥐듯 사용합니다."
- **Section 3 headline:** "한 곳에 담는 일상"
- **Section 4 headline:** "처음부터 끝까지 로컬"
- **Section 4 body:** "데이터는 당신 Mac 안에만 있습니다. Hearth는 어떤 서버로도 전송하지 않으며, SQLite 파일 경로를 공개합니다. Time Machine으로 백업되고, CLI로 직접 읽고 쓸 수 있습니다."

### 6.2 `/` (en)
- **Hero headline:** "A local-first workspace your AI agent can drive."
- **Hero subtitle:** "Projects · Memos · Schedule, in one calm place."
- **Hero CTA:** [Get it on the Mac App Store] [Install CLI]
- **Section 2 headline:** "Drive it from Claude Code."
- **Section 2 body:** "Hearth ships with a real interface for agents. Say what you want; the `skills/hearth` skill dispatches `hearth-cli`; the open Hearth window refreshes in real time. No context-switching — your agent uses your workspace the way you do."
- **Section 3 headline:** "Everything in one calm place."
- **Section 4 headline:** "Local-first, all the way."
- **Section 4 body:** "Your data lives on your Mac. Hearth never transmits to any server. The SQLite path is public; Time Machine backs it up; the CLI reads and writes it directly."

### 6.3 카피 검수 책임
- ko: founder 직접 검수 (1.0 sprint Day 9-10)
- en: founder 1차 + 가능 시 외부 검수 1회 (Day 11-12, 옵션). 실패 시 founder 단독 통과.

## 7. Day-by-day timeline (E sub-project only)

| Day | Date | 작업 |
|---|---|---|
| Day 1 | 2026-04-27 | 본 스펙 확정 + 커밋 |
| Day 2 | 2026-04-28 | Astro 프로젝트 scaffold(`web/`), Cloudflare Pages 연결, `<project>.pages.dev` 200 확인 |
| Day 3 | 2026-04-29 | Hostinger DNS CNAME 추가 → 커스텀 도메인 검증, SSL 발급 확인 |
| Day 4 | 2026-04-30 | i18n 라우팅 + locale switcher + 베이스 레이아웃 |
| Day 5-6 | 05-01 ~ 05-02 | 모든 페이지 골격 + 임시 카피로 라우트 200 OK |
| Day 7 | 2026-05-03 | privacy 페이지 1:1 재사용 + support FAQ 작성 |
| Day 8 | 2026-05-04 | 히어로 + 차별점 섹션 디자인/카피 1차 |
| Day 9-10 | 05-05 ~ 05-06 | ko 카피 정밀 검수 + 디자인 폴리시 + 영상 임베드 (App Preview 1차 컷이 나오면) |
| Day 11-12 | 05-07 ~ 05-08 | en 카피 검수 + Lighthouse 점검 + a11y 점검 |
| Day 13 | **2026-05-09** | **Live deadline.** 모든 라우트 200, 도메인·SSL 확인, ASC 메타 입력 단계로 인계 |

## 8. CLI/Tap 빌드 인프라 timeline

본 스펙의 일부지만 사이트와 별도 트랙으로 진행 가능:

| Day | 작업 |
|---|---|
| Day 6 | `homebrew-hearth` public 레포 생성, 빈 formula skeleton 머지 |
| Day 7 | R2 버킷 생성, 더미 1.0.0-rc1 바이너리 업로드, `brew install` smoke test |
| Day 10 | 실제 1.0 CLI 빌드 + 서명 + notarize 1차 워크플로 검증 |
| Day 14 | 1.0 GA 빌드 → R2 업로드 → tap formula 자동 PR → 머지 → `brew install` 최종 확인 |

(Day 14는 D 스펙의 ASC 빌드 업로드와 같은 날.)

## 9. Done-of-Definition (Day 13)

- [ ] `https://hearth.codewithgenie.com/` → 200 OK
- [ ] `https://hearth.codewithgenie.com/privacy` → 200 OK (en + ko 양쪽)
- [ ] `https://hearth.codewithgenie.com/support` → 200 OK (en + ko 양쪽)
- [ ] `https://hearth.codewithgenie.com/install/cli` → 200 OK
- [ ] `https://hearth.codewithgenie.com/install/skill` → 200 OK
- [ ] `Accept-Language` 기반 locale 리다이렉트 동작 (1회만, 쿠키 기억 후 비활성)
- [ ] `<link rel="alternate" hreflang>` 모든 페이지 검증 (Google rich results test 통과)
- [ ] Lighthouse Performance ≥ 90, Accessibility ≥ 95, Best Practices ≥ 95, SEO ≥ 95
- [ ] PR 머지 시 Cloudflare Pages 자동 배포 검증 (테스트 PR 1회)
- [ ] R2 버킷에 RC 바이너리 업로드 + tap formula로 `brew install` smoke test 통과
- [ ] 사이트 푸터에 © 2026, Privacy, Support 링크 정상

## 10. Out of Scope (1.0 한정)

- 결제/가격 비교 페이지 (App Store가 1차 표면)
- Blog / Changelog / Newsletter
- Analytics (1.1+에서 Plausible 또는 Cloudflare Web Analytics 결정)
- 회원가입 / 계정 / 대시보드
- 다크모드 수동 토글 (`prefers-color-scheme` 자동 대응만)
- ko/en 외 다른 locale (요청 시 ko로 fallback)
- A/B 테스트 인프라
- 영상 자막 i18n (영상 자체는 en 자막으로 1.0 출시; ko 자막은 1.1)

## 11. Risks & mitigations

| ID | Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| R-E1 | Hostinger DNS CNAME 전파 지연 | 낮음 | 중 | Day 3에 미리 설정. Day 13 직전이 아닌 첫 주 내 검증. |
| R-E2 | Cloudflare Pages monorepo build root 오인식 | 중 | 중 | Day 2 첫 PR로 검증. 실패 시 Astro 프로젝트를 `web/`이 아닌 `sites/web/`로 옮기는 등 경로 조정. |
| R-E3 | ko 카피 검수 시간 부족 | 중 | 중 | en 본문은 D 메타 재사용으로 부담 분리. ko 0.5일 우선 배치. |
| R-E4 | en/ko 외 locale 사용자 경험 | 낮음 | 낮음 | ko로 fallback redirect. 이슈 발생 시 1.1에서 추가. |
| R-E5 | R2 공개 access의 비용 폭증 (DDoS/abuse) | 낮음 | 중 | Cloudflare 무료 tier로 충분. Bandwidth 알람 설정. CDN 캐시로 origin 보호. |
| R-E6 | Homebrew tap 정책 위반 (private 바이너리 배포) | 낮음 | 중 | Homebrew는 private/closed-source 바이너리도 tap에서 허용. README에 명시. 정책 위반은 official `homebrew-cask`에 한정되며 user-tap은 자유로움. |
| R-E7 | App Preview 영상이 Day 13에 미완성 | 중 | 낮음 | 영상 임베드는 Day 13 시점엔 placeholder OK (App Preview 영상 자체는 D §4.5의 일정에 의존). 사이트 라우트 200 자체는 영상과 독립. |

## 12. Cross-references

- A 스펙 §4-3: security-scoped bookmark, NSOpenPanel — `/install/cli`와 `/support` FAQ에서 "왜 NSOpenPanel이 뜨나요?" 답변 근거.
- B 스펙: License 복원 흐름 — `/support` FAQ #2 답변 근거.
- D 스펙 §메타: privacy-policy.md(en+ko), review-notes.md, what-is-new.md — `/privacy` 본문 1:1 재사용, `/support` FAQ 환불 정책 부분 정합.
- D §7 R9: 외부 CLI 아키텍처 — `/install/cli` 페이지에 압축된 형태로 동일 메시지 노출.

## 13. Open questions (defer to implementation)

다음은 구현 단계에서 결정 가능한 항목으로, 본 스펙의 진행을 막지 않는다:

- 폰트 선택: 시스템 폰트만 vs Pretendard / Inter 1쌍. 권장은 시스템 + Pretendard ko 한정.
- Tailwind v4 vs Astro scoped CSS — 최종 결정은 scaffold 단계.
- `dl.codewithgenie.com` 별도 서브도메인 vs R2 기본 도메인 사용 — 비용/속도 차이 거의 없음. 첫 PR 시 결정.
- `support@codewithgenie.com` 메일박스 셋업 (현재 `hyuni2020@gmail.com`만 운용 중) — Hostinger 메일 또는 forward로 처리. Day 7 전 결정.

---

## Revisions

- **2026-04-27 (rev 1)**: 초안 작성. 결정 잠금: scope=B, stack=Astro+Cloudflare Pages, hero=Quiet, repos=private monorepo + public homebrew tap, R2 binary, CLI/skill 무료 게이트, en+ko 양쪽.
