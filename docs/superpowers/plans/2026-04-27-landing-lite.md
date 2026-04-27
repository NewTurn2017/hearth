# Landing lite (sub-project E) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hearth 1.0 출시(2026-05-17)에 필요한 5페이지 × en/ko = 10라우트 정적 사이트(`hearth.codewithgenie.com`)를 Day 13(2026-05-09)까지 라이브하고, 동시에 1st-party `hearth-cli` 동반자의 Homebrew tap + R2 바이너리 배포 인프라를 Day 14(2026-05-10)까지 가동한다.

**Architecture:** Astro 5.x 정적 빌드 → Cloudflare Pages(브랜치/PR preview 포함) → Hostinger DNS CNAME → 커스텀 도메인. 사이트 콘텐츠는 monorepo `web/` 안에서 i18n 디렉토리 라우팅(`/en/...`, `/ko/...`)으로 제공. CLI 바이너리는 별도 트랙으로 GitHub Actions에서 빌드/서명/notarize/staple 후 Cloudflare R2에 업로드, public `homebrew-hearth` 탭 formula는 자동 PR로 버전 갱신. Skill 자산은 CLI 바이너리에 컴파일타임 임베드되어 `hearth-cli install-skill`로 사용자 머신에 설치.

**Tech Stack:** Astro 5.x, Tailwind v4 또는 Astro scoped CSS(scaffold 단계 결정), Cloudflare Pages, Cloudflare R2, GitHub Actions, Homebrew, `xcrun notarytool`/`stapler`, rclone(R2 endpoint), `gh` CLI, pnpm, Node 20.

**Cross-references:**
- 스펙: `docs/superpowers/specs/2026-04-27-landing-lite-design.md`
- A 스펙: `docs/superpowers/specs/2026-04-26-mas-readiness-design.md` (§4-3 NSOpenPanel 답변 근거)
- B 스펙: `docs/superpowers/specs/2026-04-26-iap-license-design.md` (License 복원 FAQ 근거)
- D 스펙: `docs/superpowers/specs/2026-04-26-app-store-submission-design.md` (privacy-policy.md 1:1 재사용, R9 review-notes 정합)

**Two parallel tracks:**
- **Track S (Site):** Phase 0–8 — Day 1~13. 외부 의존 없는 독립 트랙.
- **Track C (CLI/Tap infra):** Phase C1–C4 — Day 6~14. Track S와 병렬 진행 가능. ASC 빌드 업로드(D 스펙)와 Day 14 동기.

---

## Phase 0 — Preflight (Day 1, 2026-04-27)

### Task 0.1: 작업 worktree 준비

**Files:**
- 없음(메타 작업)

- [ ] **Step 1: 메인 레포 상태 확인**

```bash
cd /Users/genie/dev/tools/hearth
git status && git log -1 --oneline
```

Expected: working tree clean, HEAD `319b661` (또는 그 이후 main 커밋).

- [ ] **Step 2: 사이트 작업용 worktree 생성**

```bash
git worktree add ../hearth-web feat/web-landing-lite
cd ../hearth-web
```

Expected: `feat/web-landing-lite` 브랜치 체크아웃된 새 worktree.

- [ ] **Step 3: 본 plan 위치 재확인**

```bash
test -f docs/superpowers/plans/2026-04-27-landing-lite.md && echo OK
```

Expected: `OK`.

- [ ] **Step 4: 커밋 (plan만 들어간 초기 상태)**

```bash
git add docs/superpowers/plans/2026-04-27-landing-lite.md
git commit -m "docs: landing lite implementation plan (sub-project E)"
```

---

## Phase 1 — Astro scaffold + Cloudflare Pages 연결 (Day 2, 2026-04-28)

### Task 1.1: `web/` Astro 프로젝트 scaffold

**Files:**
- Create: `web/package.json`
- Create: `web/astro.config.mjs`
- Create: `web/tsconfig.json`
- Create: `web/.gitignore`
- Create: `web/src/pages/index.astro`
- Create: `web/public/robots.txt`

- [ ] **Step 1: `web/` 디렉토리에서 Astro 최소 scaffold 수동 생성**

스캐폴드는 npm create 대신 손으로(원치 않는 의존을 피하기 위해). `web/package.json`:

```json
{
  "name": "hearth-web",
  "type": "module",
  "private": true,
  "version": "0.1.0",
  "scripts": {
    "dev": "astro dev",
    "build": "astro build",
    "preview": "astro preview",
    "astro": "astro"
  },
  "dependencies": {
    "astro": "^5.0.0"
  }
}
```

- [ ] **Step 2: `web/astro.config.mjs`**

```js
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://hearth.codewithgenie.com',
  trailingSlash: 'never',
  build: { format: 'directory' },
  i18n: {
    defaultLocale: 'ko',
    locales: ['ko', 'en'],
    routing: { prefixDefaultLocale: true, redirectToDefaultLocale: false },
    fallback: { en: 'ko' },
  },
});
```

- [ ] **Step 3: `web/tsconfig.json`**

```json
{
  "extends": "astro/tsconfigs/strict",
  "include": ["src", ".astro"]
}
```

- [ ] **Step 4: `web/.gitignore`**

```
node_modules/
dist/
.astro/
.DS_Store
.env
.env.local
```

- [ ] **Step 5: `web/public/robots.txt`**

```
User-agent: *
Allow: /
Sitemap: https://hearth.codewithgenie.com/sitemap-index.xml
```

- [ ] **Step 6: 임시 `web/src/pages/index.astro` (베어 `/` 진입 시 Accept-Language 처리는 Phase 2에서 본격 구현; 지금은 임시 ko 리다이렉트)**

```astro
---
return Astro.redirect('/ko/');
---
```

- [ ] **Step 7: 의존 설치 + 로컬 빌드 검증**

```bash
cd web && pnpm install && pnpm build
```

Expected: `web/dist/` 생성, 에러 없음.

- [ ] **Step 8: 커밋**

```bash
git add web/
git commit -m "feat(web): astro 5.x scaffold with i18n config"
```

### Task 1.2: pnpm workspace 인식 또는 단독 처리 결정

**Files:**
- Modify (조건부): `pnpm-workspace.yaml` 또는 루트 `package.json`

- [ ] **Step 1: 루트 monorepo 구조 확인**

```bash
ls /Users/genie/dev/tools/hearth-web | rg -i 'pnpm|package|cargo|workspace'
```

Expected: 루트에 `pnpm-workspace.yaml` 또는 `package.json`이 이미 있을 수도, 없을 수도. 출력에 따라 분기.

- [ ] **Step 2A (루트에 pnpm workspace 있음): `web`을 워크스페이스에 추가**

`pnpm-workspace.yaml` (없다면 생성):

```yaml
packages:
  - 'web'
```

- [ ] **Step 2B (루트에 workspace 없음): 그대로 둔다**

`web/`이 자체 pnpm 프로젝트로 동작. Cloudflare Pages 빌드 명령어가 `web/`를 root로 가리키므로 문제 없음.

- [ ] **Step 3: 커밋(변경이 있었던 경우)**

```bash
git add pnpm-workspace.yaml
git commit -m "chore: add web to pnpm workspace"
```

### Task 1.3: Cloudflare Pages 프로젝트 연결

**Files:**
- 외부 작업(Cloudflare 대시보드)

- [ ] **Step 1: PR 푸시(미리보기 트리거 목적)**

```bash
git push -u origin feat/web-landing-lite
gh pr create --draft --title "feat(web): landing lite scaffold" \
  --body "Sub-project E. Cloudflare Pages 연결을 위한 초기 PR."
```

Expected: PR URL 출력.

- [ ] **Step 2: Cloudflare 대시보드에서 Pages 프로젝트 생성 (사용자 수동)**

설정값:
- Repository: private `hearth` (GitHub App 인증 필요)
- Production branch: `main`
- Root directory: `web`
- Build command: `pnpm install && pnpm build`
- Output directory: `dist` (Pages가 root 기준이므로 `web` root에서 `dist`)
- Node version: `20`
- Project name: `hearth-web` (또는 동등)

- [ ] **Step 3: 첫 빌드 결과 확인**

대시보드에서 빌드 로그 PASS 확인. Production URL: `https://hearth-web.pages.dev` (또는 실제 슬러그) → 200.

```bash
curl -I https://hearth-web.pages.dev/
```

Expected: HTTP/2 200 또는 308(→ /ko/) 로그.

- [ ] **Step 4: PR preview URL 동작 확인**

PR 댓글에 Cloudflare Pages bot이 preview URL을 자동 댓글. `curl -I` 로 200 확인.

- [ ] **Step 5: PR을 ready로 승격하지는 말 것 (Phase 8까지 draft 유지)**

이 PR이 Day 13에 머지되면서 라이브가 된다. Phase 끝까지 incremental commits만.

---

## Phase 2 — 도메인 + i18n + 베이스 레이아웃 (Day 3-4, 2026-04-29 ~ 04-30)

### Task 2.1: 커스텀 도메인 연결

**Files:**
- 외부 작업(Hostinger DNS, Cloudflare)

- [ ] **Step 1: Cloudflare Pages → 프로젝트 → Custom domains → `hearth.codewithgenie.com` 추가**

CNAME target 표기 확인 (예: `hearth-web.pages.dev`).

- [ ] **Step 2: Hostinger DNS 콘솔에서 CNAME 추가**

- Type: `CNAME`
- Name: `hearth`
- Value: `hearth-web.pages.dev`
- TTL: 자동 또는 300

- [ ] **Step 3: 전파 확인**

```bash
dig +short hearth.codewithgenie.com
```

Expected: Pages CNAME chain 또는 Cloudflare IP 반환. 전파에 5-30분 소요 가능.

- [ ] **Step 4: SSL 자동 발급 확인**

```bash
curl -I https://hearth.codewithgenie.com/
```

Expected: HTTP/2 200 또는 308. Certificate가 Cloudflare 발급으로 표시.

### Task 2.2: 공통 레이아웃 컴포넌트

**Files:**
- Create: `web/src/layouts/Base.astro`
- Create: `web/src/components/Header.astro`
- Create: `web/src/components/Footer.astro`
- Create: `web/src/components/LocaleSwitcher.astro`

- [ ] **Step 1: `web/src/layouts/Base.astro`**

```astro
---
import Header from '../components/Header.astro';
import Footer from '../components/Footer.astro';

interface Props {
  title: string;
  description: string;
  locale: 'ko' | 'en';
  path: string; // e.g. "/install/cli"
}

const { title, description, locale, path } = Astro.props;
const altLocale = locale === 'ko' ? 'en' : 'ko';
const canonical = `https://hearth.codewithgenie.com/${locale}${path}`;
const altUrl = `https://hearth.codewithgenie.com/${altLocale}${path}`;
const xDefaultUrl = `https://hearth.codewithgenie.com/ko${path}`;
---
<!doctype html>
<html lang={locale}>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{title}</title>
    <meta name="description" content={description} />
    <link rel="canonical" href={canonical} />
    <link rel="alternate" hreflang={locale} href={canonical} />
    <link rel="alternate" hreflang={altLocale} href={altUrl} />
    <link rel="alternate" hreflang="x-default" href={xDefaultUrl} />
    <meta name="color-scheme" content="light dark" />
  </head>
  <body>
    <Header locale={locale} path={path} />
    <main><slot /></main>
    <Footer locale={locale} />
  </body>
</html>
```

- [ ] **Step 2: `web/src/components/LocaleSwitcher.astro`**

```astro
---
interface Props { locale: 'ko' | 'en'; path: string; }
const { locale, path } = Astro.props;
const other = locale === 'ko' ? 'en' : 'ko';
const otherLabel = other === 'ko' ? '한국어' : 'English';
---
<a href={`/${other}${path}`} hreflang={other} rel="alternate" data-locale-switch>
  {otherLabel}
</a>
<script is:inline>
  document.querySelectorAll('[data-locale-switch]').forEach((a) => {
    a.addEventListener('click', () => {
      document.cookie = `hearth_locale=${a.getAttribute('hreflang')}; path=/; max-age=31536000; SameSite=Lax`;
    });
  });
</script>
```

- [ ] **Step 3: `web/src/components/Header.astro`**

```astro
---
import LocaleSwitcher from './LocaleSwitcher.astro';
interface Props { locale: 'ko' | 'en'; path: string; }
const { locale, path } = Astro.props;
const nav = locale === 'ko'
  ? [{ href: '/install/cli', label: 'CLI 설치' }, { href: '/install/skill', label: 'Skill 설치' }, { href: '/support', label: '지원' }]
  : [{ href: '/install/cli', label: 'Install CLI' }, { href: '/install/skill', label: 'Install Skill' }, { href: '/support', label: 'Support' }];
---
<header>
  <a href={`/${locale}/`} aria-label="Hearth home">Hearth</a>
  <nav>
    {nav.map((n) => <a href={`/${locale}${n.href}`}>{n.label}</a>)}
    <LocaleSwitcher locale={locale} path={path} />
  </nav>
</header>
```

- [ ] **Step 4: `web/src/components/Footer.astro`**

```astro
---
interface Props { locale: 'ko' | 'en'; }
const { locale } = Astro.props;
const labels = locale === 'ko'
  ? { privacy: '개인정보처리방침', support: '지원', copyright: '© 2026 Hearth' }
  : { privacy: 'Privacy', support: 'Support', copyright: '© 2026 Hearth' };
---
<footer>
  <a href={`/${locale}/privacy`}>{labels.privacy}</a>
  <a href={`/${locale}/support`}>{labels.support}</a>
  <span>{labels.copyright}</span>
</footer>
```

- [ ] **Step 5: 빌드 검증**

```bash
cd web && pnpm build
```

Expected: 에러 없이 종료(컴포넌트 자체는 어디에도 import되지 않았으므로 dist에 미반영 정상).

- [ ] **Step 6: 커밋**

```bash
git add web/src/layouts web/src/components
git commit -m "feat(web): base layout, header, footer, locale switcher"
```

### Task 2.3: Accept-Language 베어 `/` 처리

**Files:**
- Modify: `web/src/pages/index.astro`

- [ ] **Step 1: 베어 `/` 진입 시 쿠키 → Accept-Language → ko 순서로 리다이렉트**

Cloudflare Pages는 Astro의 SSR이 아닌 정적 모드라 런타임 헤더 분기가 어렵다. 따라서 `/index.astro`는 `<meta http-equiv="refresh">` + `<script>` 조합의 client-side 리다이렉트로 처리한다.

```astro
---
// 정적 빌드 fallback. 자바스크립트 비활성 환경은 ko로 강제.
---
<!doctype html>
<html lang="ko">
  <head>
    <meta charset="utf-8" />
    <title>Hearth</title>
    <meta http-equiv="refresh" content="0;url=/ko/" />
    <script is:inline>
      (function () {
        var match = document.cookie.match(/(?:^|; )hearth_locale=(ko|en)/);
        var locale = match ? match[1] : (navigator.language && navigator.language.startsWith('en') ? 'en' : 'ko');
        location.replace('/' + locale + '/');
      })();
    </script>
  </head>
  <body></body>
</html>
```

- [ ] **Step 2: 빌드 + dist 검증**

```bash
pnpm build && bat web/dist/index.html | head -20
```

Expected: 위 HTML이 `dist/index.html`로 출력.

- [ ] **Step 3: 커밋**

```bash
git add web/src/pages/index.astro
git commit -m "feat(web): client-side locale redirect from bare /"
```

---

## Phase 3 — 페이지 골격 (Day 5-6, 2026-05-01 ~ 05-02)

### Task 3.1: ko/en 홈 페이지 (라우트 200, 임시 카피)

**Files:**
- Create: `web/src/pages/ko/index.astro`
- Create: `web/src/pages/en/index.astro`

- [ ] **Step 1: `web/src/pages/ko/index.astro`**

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base
  title="Hearth · 로컬에 사는 AI 에이전트 워크스페이스"
  description="Projects · Memos · Schedule, 한 곳에 차분하게."
  locale="ko"
  path="/"
>
  <section data-section="hero">
    <h1>로컬에 사는 AI 에이전트 워크스페이스</h1>
    <p>Projects · Memos · Schedule, 한 곳에 차분하게.</p>
    <p>
      <a href="https://apps.apple.com/app/hearth/id0000000000">Mac App Store에서 받기</a>
      <a href="/ko/install/cli">CLI 설치하기</a>
    </p>
  </section>
  <section data-section="differentiator">
    <h2>Claude Code에서 직접 조작하세요</h2>
    <p>Hearth는 명령형 인터페이스를 제공합니다. 자연어로 시키면 <code>skills/hearth</code>가 <code>hearth-cli</code>를 호출하고, 열려 있는 Hearth는 즉시 새로고침됩니다.</p>
  </section>
  <section data-section="features">
    <h2>한 곳에 담는 일상</h2>
    <ul>
      <li><strong>Projects</strong> — 프로젝트 단위로 모든 것을 묶는다</li>
      <li><strong>Memos</strong> — 짧은 생각, 긴 문서, 다 같은 자리에</li>
      <li><strong>Schedule</strong> — 오늘과 다음 주를 한 화면에</li>
    </ul>
  </section>
  <section data-section="local-first">
    <h2>처음부터 끝까지 로컬</h2>
    <p>데이터는 당신 Mac 안에만 있습니다. <code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>. Time Machine으로 백업되고, CLI로 직접 읽고 쓸 수 있습니다.</p>
  </section>
</Base>
```

- [ ] **Step 2: `web/src/pages/en/index.astro`**

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base
  title="Hearth · A local-first workspace your AI agent can drive"
  description="Projects · Memos · Schedule, in one calm place."
  locale="en"
  path="/"
>
  <section data-section="hero">
    <h1>A local-first workspace your AI agent can drive.</h1>
    <p>Projects · Memos · Schedule, in one calm place.</p>
    <p>
      <a href="https://apps.apple.com/app/hearth/id0000000000">Get it on the Mac App Store</a>
      <a href="/en/install/cli">Install CLI</a>
    </p>
  </section>
  <section data-section="differentiator">
    <h2>Drive it from Claude Code.</h2>
    <p>Hearth ships with a real interface for agents. Say what you want; the <code>skills/hearth</code> skill dispatches <code>hearth-cli</code>; the open Hearth window refreshes in real time.</p>
  </section>
  <section data-section="features">
    <h2>Everything in one calm place.</h2>
    <ul>
      <li><strong>Projects</strong> — group everything by project</li>
      <li><strong>Memos</strong> — short thoughts, long notes, same place</li>
      <li><strong>Schedule</strong> — today and next week, one view</li>
    </ul>
  </section>
  <section data-section="local-first">
    <h2>Local-first, all the way.</h2>
    <p>Your data lives on your Mac at <code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>. Hearth never transmits to any server. Time Machine backs it up; the CLI reads and writes it directly.</p>
  </section>
</Base>
```

- [ ] **Step 3: 빌드 + 라우트 200 검증**

```bash
pnpm build
test -f web/dist/ko/index.html && test -f web/dist/en/index.html && echo OK
```

Expected: `OK`.

- [ ] **Step 4: 커밋**

```bash
git add web/src/pages/ko/index.astro web/src/pages/en/index.astro
git commit -m "feat(web): ko/en home page skeletons with seed copy"
```

### Task 3.2: `/install/cli` (ko/en)

**Files:**
- Create: `web/src/pages/ko/install/cli.astro`
- Create: `web/src/pages/en/install/cli.astro`

- [ ] **Step 1: `web/src/pages/ko/install/cli.astro`**

```astro
---
import Base from '../../../layouts/Base.astro';
---
<Base
  title="hearth-cli 설치 · Hearth"
  description="Homebrew로 hearth-cli를 설치하세요."
  locale="ko"
  path="/install/cli"
>
  <h1>hearth-cli 설치</h1>
  <p>Hearth 앱이 먼저 <a href="https://apps.apple.com/app/hearth/id0000000000">Mac App Store</a>에 설치되어 있어야 합니다.</p>

  <h2>1단계 — tap 추가 + 설치</h2>
  <pre><code>brew tap codewithgenie/hearth
brew install hearth-cli</code></pre>

  <h2>2단계 — 검증</h2>
  <pre><code>hearth-cli --version</code></pre>
  <p>Hearth 1.0과 호환되는 버전이 출력되면 성공입니다.</p>

  <h2>시스템 요구사항</h2>
  <ul>
    <li>macOS 11 이상</li>
    <li>Apple Silicon 또는 Intel 모두 지원</li>
    <li>바이너리는 Developer ID로 서명되고 Apple notarization을 통과합니다 — Gatekeeper 경고 없이 자동 실행됩니다.</li>
  </ul>

  <h2>제거</h2>
  <pre><code>brew uninstall hearth-cli
brew untap codewithgenie/hearth</code></pre>

  <h2>다음 단계</h2>
  <p>Claude Code에서 자연어로 호출하려면 <a href="/ko/install/skill">Hearth skill 설치</a>를 이어가세요.</p>
</Base>
```

- [ ] **Step 2: `web/src/pages/en/install/cli.astro`**

```astro
---
import Base from '../../../layouts/Base.astro';
---
<Base
  title="Install hearth-cli · Hearth"
  description="Install hearth-cli via Homebrew."
  locale="en"
  path="/install/cli"
>
  <h1>Install hearth-cli</h1>
  <p>Install the <a href="https://apps.apple.com/app/hearth/id0000000000">Hearth app</a> from the Mac App Store first.</p>

  <h2>Step 1 — Tap and install</h2>
  <pre><code>brew tap codewithgenie/hearth
brew install hearth-cli</code></pre>

  <h2>Step 2 — Verify</h2>
  <pre><code>hearth-cli --version</code></pre>
  <p>If it prints a Hearth 1.0–compatible version, you're done.</p>

  <h2>Requirements</h2>
  <ul>
    <li>macOS 11 or later</li>
    <li>Apple Silicon and Intel both supported</li>
    <li>Binaries are Developer ID–signed and Apple-notarized — Gatekeeper lets them run without prompts.</li>
  </ul>

  <h2>Uninstall</h2>
  <pre><code>brew uninstall hearth-cli
brew untap codewithgenie/hearth</code></pre>

  <h2>Next step</h2>
  <p>To call Hearth from Claude Code in natural language, continue to <a href="/en/install/skill">Install the Hearth skill</a>.</p>
</Base>
```

- [ ] **Step 3: 빌드 + 200 확인**

```bash
pnpm build
test -f web/dist/ko/install/cli/index.html && test -f web/dist/en/install/cli/index.html && echo OK
```

Expected: `OK`.

- [ ] **Step 4: 커밋**

```bash
git add web/src/pages/ko/install/cli.astro web/src/pages/en/install/cli.astro
git commit -m "feat(web): /install/cli ko+en skeletons"
```

### Task 3.3: `/install/skill` (ko/en)

**Files:**
- Create: `web/src/pages/ko/install/skill.astro`
- Create: `web/src/pages/en/install/skill.astro`

- [ ] **Step 1: `web/src/pages/ko/install/skill.astro`**

```astro
---
import Base from '../../../layouts/Base.astro';
---
<Base
  title="Hearth skill을 Claude Code에 추가하기"
  description="hearth-cli install-skill 한 줄."
  locale="ko"
  path="/install/skill"
>
  <h1>Hearth skill을 Claude Code에 추가하기</h1>
  <p>먼저 <a href="/ko/install/cli">hearth-cli</a>가 설치되어 있어야 합니다.</p>

  <h2>한 줄 명령</h2>
  <pre><code>hearth-cli install-skill</code></pre>

  <h2>무엇이 일어나나요?</h2>
  <ul>
    <li><code>~/.claude/skills/hearth/</code> 디렉토리가 생성됩니다.</li>
    <li>skill.md와 보조 파일이 그 안에 풀립니다.</li>
    <li>다음 Claude Code 세션부터 자동으로 인식됩니다.</li>
  </ul>

  <h2>사용 예</h2>
  <p>Claude Code에서 자연어로 호출하세요. 예: <em>"Hearth에 메모 남겨줘 — 내일 회의 안건 정리"</em>.</p>

  <h2>제거</h2>
  <pre><code>hearth-cli uninstall-skill
# 또는 수동
rm -rf ~/.claude/skills/hearth</code></pre>
</Base>
```

- [ ] **Step 2: `web/src/pages/en/install/skill.astro`**

```astro
---
import Base from '../../../layouts/Base.astro';
---
<Base
  title="Add the Hearth skill to Claude Code"
  description="One command: hearth-cli install-skill."
  locale="en"
  path="/install/skill"
>
  <h1>Add the Hearth skill to Claude Code</h1>
  <p>Make sure <a href="/en/install/cli">hearth-cli</a> is installed first.</p>

  <h2>One command</h2>
  <pre><code>hearth-cli install-skill</code></pre>

  <h2>What happens</h2>
  <ul>
    <li><code>~/.claude/skills/hearth/</code> is created.</li>
    <li>skill.md and supporting files are unpacked there.</li>
    <li>The next Claude Code session picks it up automatically.</li>
  </ul>

  <h2>Example</h2>
  <p>Call it in natural language. e.g. <em>"Leave a memo in Hearth — agenda for tomorrow's meeting."</em></p>

  <h2>Uninstall</h2>
  <pre><code>hearth-cli uninstall-skill
# or manually
rm -rf ~/.claude/skills/hearth</code></pre>
</Base>
```

- [ ] **Step 3: 빌드 + 200 확인**

```bash
pnpm build
test -f web/dist/ko/install/skill/index.html && test -f web/dist/en/install/skill/index.html && echo OK
```

Expected: `OK`.

- [ ] **Step 4: 커밋**

```bash
git add web/src/pages/ko/install/skill.astro web/src/pages/en/install/skill.astro
git commit -m "feat(web): /install/skill ko+en skeletons"
```

### Task 3.4: `/privacy` 라우트 placeholder (Phase 4에서 본문 채움)

**Files:**
- Create: `web/src/pages/ko/privacy.astro`
- Create: `web/src/pages/en/privacy.astro`

- [ ] **Step 1: ko/en placeholder 라우트**

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base title="Privacy · Hearth" description="Hearth privacy policy." locale="ko" path="/privacy">
  <h1>개인정보처리방침</h1>
  <p>본문은 Phase 4에서 채워집니다.</p>
</Base>
```

(en 버전은 동일하게 `locale="en"`, 헤더 영문.)

- [ ] **Step 2: 200 확인 + 커밋**

```bash
pnpm build && test -f web/dist/ko/privacy/index.html && test -f web/dist/en/privacy/index.html && echo OK
git add web/src/pages/ko/privacy.astro web/src/pages/en/privacy.astro
git commit -m "feat(web): /privacy placeholder routes (200)"
```

### Task 3.5: `/support` 라우트 placeholder (Phase 4에서 FAQ 채움)

**Files:**
- Create: `web/src/pages/ko/support.astro`
- Create: `web/src/pages/en/support.astro`

- [ ] **Step 1: ko/en placeholder**

ko:

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base title="지원 · Hearth" description="Hearth 지원 정보." locale="ko" path="/support">
  <h1>How can we help?</h1>
  <p>본문은 Phase 4에서 채워집니다.</p>
</Base>
```

en은 동일 패턴.

- [ ] **Step 2: 빌드 + 커밋**

```bash
pnpm build && test -f web/dist/ko/support/index.html && test -f web/dist/en/support/index.html && echo OK
git add web/src/pages/ko/support.astro web/src/pages/en/support.astro
git commit -m "feat(web): /support placeholder routes (200)"
```

### Task 3.6: 모든 라우트 200 OK Cloudflare preview 검증

**Files:**
- 없음(검증)

- [ ] **Step 1: PR push**

```bash
git push
```

- [ ] **Step 2: PR preview URL이 댓글로 달리면, 10개 라우트 모두 200 확인**

```bash
PREVIEW=https://<branch>.<project>.pages.dev  # 댓글에서 복사
for path in / /ko/ /en/ /ko/install/cli /en/install/cli /ko/install/skill /en/install/skill /ko/privacy /en/privacy /ko/support /en/support; do
  echo "$path -> $(curl -sI "$PREVIEW$path" | head -n 1)"
done
```

Expected: 모두 200 또는 308.

---

## Phase 4 — Privacy + Support 콘텐츠 (Day 7, 2026-05-03)

### Task 4.1: privacy 본문 1:1 재사용

**Files:**
- Modify: `web/src/pages/ko/privacy.astro`
- Modify: `web/src/pages/en/privacy.astro`
- Reference: `docs/superpowers/app-store/privacy-policy.md` (또는 D 메타 위치 — Phase 시작 시 위치 재확인)

- [ ] **Step 1: D 메타 privacy 마크다운 위치 확인**

```bash
fd -t f 'privacy-policy' /Users/genie/dev/tools/hearth/docs/superpowers/app-store/
```

Expected: 9개 섹션짜리 ko/en 마크다운 발견.

- [ ] **Step 2: ko 본문을 `web/src/pages/ko/privacy.astro`에 9개 섹션 그대로 옮김**

마크다운 → Astro로 옮길 때는 `<h2>`, `<p>`, `<ul>`만 사용. 변경 없이 1:1.

마지막에 빌드 시점 표기:

```astro
---
const lastUpdated = new Date().toISOString().slice(0, 10);
---
<!-- ... -->
<p><small>Last updated: {lastUpdated}</small></p>
```

- [ ] **Step 3: en 본문도 동일하게 옮김**

- [ ] **Step 4: ASC 메타와의 동기 의무를 코드 코멘트로 박제**

`web/src/pages/ko/privacy.astro` 상단에 1줄 코멘트:

```astro
---
// 본문은 docs/superpowers/app-store/privacy-policy.md(ko)와 1:1 동기. 변경 시 양쪽 모두 갱신.
---
```

- [ ] **Step 5: 빌드 + 커밋**

```bash
pnpm build
git add web/src/pages/ko/privacy.astro web/src/pages/en/privacy.astro
git commit -m "feat(web): privacy ko+en full content (1:1 sync with ASC meta)"
```

### Task 4.2: support FAQ 6개 작성

**Files:**
- Modify: `web/src/pages/ko/support.astro`
- Modify: `web/src/pages/en/support.astro`

- [ ] **Step 1: ko 본문 — 연락처 + FAQ 6개**

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base title="지원 · Hearth" description="Hearth 지원 및 FAQ." locale="ko" path="/support">
  <h1>How can we help?</h1>

  <h2>연락처</h2>
  <p>이메일: <a href="mailto:support@codewithgenie.com">support@codewithgenie.com</a></p>
  <p>응답 시간: 영업일 기준 48시간 이내.</p>

  <h2>자주 묻는 질문</h2>

  <h3>1. 설치 후 첫 실행에서 NSOpenPanel이 나타나는데 왜요?</h3>
  <p>Hearth와 hearth-cli가 같은 SQLite 파일(<code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>)을 공유하기 위해 macOS 샌드박스가 한 번의 사용자 동의를 요구합니다. "Application Support" 폴더를 한 번만 선택하면 security-scoped bookmark가 저장되어 다음부터는 자동으로 접근합니다.</p>

  <h3>2. 구매 영수증 복원은 어떻게 하나요?</h3>
  <p>Hearth 앱 → 설정 → License → "Restore Purchase". 같은 Apple ID로 결제한 적이 있다면 즉시 복원됩니다.</p>

  <h3>3. DB 파일은 어디에 있나요? 백업 가능한가요?</h3>
  <p>경로: <code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>. Time Machine 백업 대상에 자동 포함됩니다. 별도 복사도 자유롭게 하세요.</p>

  <h3>4. CLI 또는 skill이 작동하지 않을 때 체크리스트</h3>
  <ol>
    <li>Hearth 앱이 실행 중인지 확인</li>
    <li><code>hearth-cli --version</code>이 1.0과 호환되는지 확인</li>
    <li>Application Support 접근 권한 동의 여부 (Settings → Privacy)</li>
  </ol>

  <h3>5. macOS 업그레이드 후 데이터가 사라졌어요</h3>
  <p>Time Machine에서 <code>~/Library/Application Support/com.newturn2017.hearth/</code>를 복원하면 됩니다. 복원 후 Hearth를 재시작하세요.</p>

  <h3>6. 환불 정책</h3>
  <p>환불은 Apple App Store에서 직접 신청합니다 — Hearth가 처리하지 않습니다. <a href="https://reportaproblem.apple.com">reportaproblem.apple.com</a>에서 영수증을 선택하고 환불을 요청하세요.</p>
</Base>
```

- [ ] **Step 2: en 본문 — 동일 6개 FAQ 영문**

```astro
---
import Base from '../../layouts/Base.astro';
---
<Base title="Support · Hearth" description="Hearth support and FAQ." locale="en" path="/support">
  <h1>How can we help?</h1>

  <h2>Contact</h2>
  <p>Email: <a href="mailto:support@codewithgenie.com">support@codewithgenie.com</a></p>
  <p>Response time: within 48 business hours.</p>

  <h2>Frequently asked</h2>

  <h3>1. Why does macOS show an NSOpenPanel on first launch?</h3>
  <p>Hearth and hearth-cli share a single SQLite file at <code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>. macOS sandboxing asks for one-time user consent. Pick the "Application Support" folder once; a security-scoped bookmark is saved and access becomes automatic afterwards.</p>

  <h3>2. How do I restore my purchase?</h3>
  <p>In Hearth: Settings → License → "Restore Purchase". If the same Apple ID purchased before, it restores immediately.</p>

  <h3>3. Where is the database file? Can I back it up?</h3>
  <p>Path: <code>~/Library/Application Support/com.newturn2017.hearth/data.db</code>. Time Machine includes it automatically. Feel free to copy it elsewhere too.</p>

  <h3>4. CLI or skill not working — checklist</h3>
  <ol>
    <li>Make sure the Hearth app is running.</li>
    <li>Confirm <code>hearth-cli --version</code> is Hearth 1.0–compatible.</li>
    <li>Check macOS Privacy settings for Application Support access consent.</li>
  </ol>

  <h3>5. My data disappeared after a macOS upgrade</h3>
  <p>Restore <code>~/Library/Application Support/com.newturn2017.hearth/</code> from Time Machine, then relaunch Hearth.</p>

  <h3>6. Refund policy</h3>
  <p>Refunds are processed by Apple — not by Hearth. Visit <a href="https://reportaproblem.apple.com">reportaproblem.apple.com</a>, select the receipt, and request a refund.</p>
</Base>
```

- [ ] **Step 3: 빌드 + 커밋**

```bash
pnpm build
git add web/src/pages/ko/support.astro web/src/pages/en/support.astro
git commit -m "feat(web): support FAQ ko+en (6 items, contact, response SLA)"
```

### Task 4.3: support 메일박스 결정

**Files:**
- 외부(Hostinger 메일 또는 forwarding)

- [ ] **Step 1: `support@codewithgenie.com` 운용 결정**

옵션 A: Hostinger 메일 박스 신규 발급. 옵션 B: `support@` → `hyuni2020@gmail.com` forwarding (Hostinger DNS).

- [ ] **Step 2: 선택한 옵션을 적용 + smoke test**

자기 자신에게 외부 메일 1통 발송 → `support@codewithgenie.com` → Gmail inbox 도착 확인.

Expected: 5분 이내 도착.

---

## Phase 5 — 히어로 + 차별점 섹션 디자인/카피 1차 (Day 8, 2026-05-04)

### Task 5.1: 스타일링 의사결정 + Tailwind v4 도입(또는 scoped CSS)

**Files:**
- Modify: `web/package.json`
- Create: `web/src/styles/global.css`
- Modify: `web/astro.config.mjs` (Tailwind 선택 시)

- [ ] **Step 1: 결정 — Tailwind v4 선택 (사이트 규모 작지만 디자인 토큰 일관성을 위해)**

```bash
cd web && pnpm add -D tailwindcss@next @tailwindcss/vite
```

- [ ] **Step 2: `web/astro.config.mjs`에 Vite plugin 추가**

```js
import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  site: 'https://hearth.codewithgenie.com',
  trailingSlash: 'never',
  build: { format: 'directory' },
  vite: { plugins: [tailwindcss()] },
  i18n: {
    defaultLocale: 'ko',
    locales: ['ko', 'en'],
    routing: { prefixDefaultLocale: true, redirectToDefaultLocale: false },
    fallback: { en: 'ko' },
  },
});
```

- [ ] **Step 3: `web/src/styles/global.css`**

```css
@import "tailwindcss";

@theme {
  --color-paper: #fafaf7;
  --color-ink: #1a1814;
  --color-ember: #c75a1a;
  --font-sans: ui-sans-serif, "Pretendard Variable", "Pretendard", system-ui, sans-serif;
}

body {
  background: var(--color-paper);
  color: var(--color-ink);
  font-family: var(--font-sans);
  font-feature-settings: "ss01";
}
```

- [ ] **Step 4: `Base.astro`에 import 추가**

```astro
---
import '../styles/global.css';
// ...rest
---
```

- [ ] **Step 5: 빌드 + 커밋**

```bash
pnpm build
git add web/package.json web/pnpm-lock.yaml web/astro.config.mjs web/src/styles/global.css web/src/layouts/Base.astro
git commit -m "feat(web): tailwind v4 + paper/ink/ember tokens"
```

### Task 5.2: 히어로 섹션 폴리시 (ko/en)

**Files:**
- Modify: `web/src/pages/ko/index.astro`
- Modify: `web/src/pages/en/index.astro`

- [ ] **Step 1: 히어로 영역에 단일 컬럼 + 큰 타이포그래피 + CTA 2개 적용**

ko (en도 대칭으로 동일 구조):

```astro
<section class="mx-auto max-w-3xl px-6 py-24 text-center">
  <h1 class="text-5xl font-semibold tracking-tight md:text-6xl">로컬에 사는<br/>AI 에이전트 워크스페이스</h1>
  <p class="mt-6 text-lg text-ink/70">Projects · Memos · Schedule, 한 곳에 차분하게.</p>
  <div class="mt-10 flex flex-col items-center justify-center gap-3 md:flex-row md:gap-4">
    <a href="https://apps.apple.com/app/hearth/id0000000000"
       class="inline-block rounded-full bg-ink px-6 py-3 text-paper">Mac App Store에서 받기</a>
    <a href="/ko/install/cli"
       class="inline-block rounded-full border border-ink/20 px-6 py-3">CLI 설치하기</a>
  </div>
</section>
```

- [ ] **Step 2: 차별점 섹션 — 어두운 톤 + 영상 placeholder**

```astro
<section class="bg-ink text-paper">
  <div class="mx-auto max-w-4xl px-6 py-24">
    <h2 class="text-3xl font-semibold md:text-4xl">Claude Code에서 직접 조작하세요</h2>
    <p class="mt-4 text-paper/80">Hearth는 명령형 인터페이스를 제공합니다. 자연어로 시키면 <code class="rounded bg-paper/10 px-1">skills/hearth</code>가 <code class="rounded bg-paper/10 px-1">hearth-cli</code>를 호출하고, 열려 있는 Hearth는 즉시 새로고침됩니다.</p>
    <div class="mt-8 aspect-video w-full rounded-lg bg-paper/5">
      <!-- Phase 6에서 <video> 임베드 -->
    </div>
  </div>
</section>
```

- [ ] **Step 3: 나머지 두 섹션도 같은 max-w + 여백 규칙으로 정리**

`features`와 `local-first` 섹션을 paper 배경 + `mx-auto max-w-3xl px-6 py-20`로 통일.

- [ ] **Step 4: CTA repeat + footer 직전 섹션 1개 추가**

```astro
<section class="mx-auto max-w-3xl px-6 py-20 text-center">
  <div class="flex flex-col items-center justify-center gap-3 md:flex-row md:gap-4">
    <a href="https://apps.apple.com/app/hearth/id0000000000"
       class="inline-block rounded-full bg-ink px-6 py-3 text-paper">Mac App Store에서 받기</a>
    <a href="/ko/install/cli"
       class="inline-block rounded-full border border-ink/20 px-6 py-3">CLI 설치하기</a>
  </div>
</section>
```

- [ ] **Step 5: en에도 동일 구조 적용 (텍스트만 영문)**

- [ ] **Step 6: 빌드 + 커밋**

```bash
pnpm build
git add web/src/pages/ko/index.astro web/src/pages/en/index.astro
git commit -m "feat(web): hero + differentiator polish (Quiet local-first tone)"
```

---

## Phase 6 — ko 카피 정밀 검수 + App Preview 영상 임베드 (Day 9-10, 2026-05-05 ~ 05-06)

### Task 6.1: ko 카피 검수 (founder 직접)

**Files:**
- Modify: 모든 `web/src/pages/ko/*.astro`

- [ ] **Step 1: founder가 ko 라우트 5개를 한 번 통독**

```bash
pnpm dev
# → http://localhost:4321/ko/, /ko/install/cli, /ko/install/skill, /ko/privacy, /ko/support 통독
```

- [ ] **Step 2: 발견된 문장 수정 (toner: Quiet local-first 유지)**

체크리스트:
- 영어 표현 직역 흔적이 없는가?
- "에이전트", "워크스페이스" 같은 용어가 일관되는가?
- CTA 동사가 명료한가?

- [ ] **Step 3: 커밋**

```bash
git add web/src/pages/ko
git commit -m "copy(web): ko polish pass (founder review)"
```

### Task 6.2: App Preview 영상 임베드 (영상 1차 컷이 있는 경우)

**Files:**
- Modify: `web/src/pages/ko/index.astro`
- Modify: `web/src/pages/en/index.astro`
- 외부: R2 버킷에 영상 업로드(또는 `/web/public/`에 직접 — 선택)

- [ ] **Step 1: 영상 위치 결정**

옵션 A: `web/public/preview.mp4` (Cloudflare Pages 자체 호스팅 — 빌드 산출물에 포함)
옵션 B: R2 `media/preview.mp4` (별도 도메인) — 빌드 사이즈 절약

소형(<5MB) → 옵션 A, 대형(≥5MB) → 옵션 B 권장.

- [ ] **Step 2: 영상 + 포스터 추가**

```bash
# 옵션 A의 경우
cp ~/path/to/preview.mp4 web/public/preview.mp4
cp ~/path/to/preview-poster.jpg web/public/preview-poster.jpg
```

- [ ] **Step 3: 차별점 섹션 placeholder를 `<video>`로 교체**

```astro
<div class="mt-8 aspect-video w-full overflow-hidden rounded-lg bg-paper/5">
  <video
    class="h-full w-full"
    src="/preview.mp4"
    poster="/preview-poster.jpg"
    controls
    preload="none"
    playsinline
  ></video>
</div>
```

- [ ] **Step 4: 영상이 미완성이면 placeholder 유지하고 R-E7 위험 수용을 메모**

해당 task를 skip 처리하고 PR 본문에 "App Preview embed deferred — waiting on D §4.5 cut" 명기.

- [ ] **Step 5: 빌드 + 커밋(영상 임베드한 경우)**

```bash
pnpm build
git add web/public web/src/pages/ko/index.astro web/src/pages/en/index.astro
git commit -m "feat(web): embed App Preview video on home"
```

### Task 6.3: 디자인 폴리시 (전체 라우트)

**Files:**
- Modify: 컴포넌트 + 페이지 (필요한 곳만)

- [ ] **Step 1: 헤더 + 푸터 스타일링 적용**

`Header.astro`, `Footer.astro`에 `mx-auto max-w-5xl px-6 py-6 flex justify-between items-center` 등 적용.

- [ ] **Step 2: 코드 블록(`<pre><code>`)에 mono + 배경 통일**

`global.css`에 추가:

```css
pre {
  background: color-mix(in srgb, var(--color-ink) 96%, white);
  color: var(--color-paper);
  border-radius: 0.5rem;
  padding: 1rem;
  overflow-x: auto;
  font-size: 0.875rem;
}
code { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
```

- [ ] **Step 3: prefers-color-scheme 다크 자동 대응**

```css
@media (prefers-color-scheme: dark) {
  :root {
    --color-paper: #14120f;
    --color-ink: #f2efe8;
  }
}
```

- [ ] **Step 4: 빌드 + 커밋**

```bash
pnpm build
git add web
git commit -m "style(web): global polish (header/footer/code/dark)"
```

---

## Phase 7 — en 카피 + Lighthouse + a11y (Day 11-12, 2026-05-07 ~ 05-08)

### Task 7.1: en 카피 검수

**Files:**
- Modify: 모든 `web/src/pages/en/*.astro`

- [ ] **Step 1: founder 1차 통독 + 자연스럽지 않은 표현 수정**

체크리스트:
- 시제/관사 일관성
- "your" vs "the" 선택 일관성
- 한국어 직역 흔적("It is composed of...") 제거

- [ ] **Step 2: 외부 검수가 가능하면 friend/colleague에게 1회 review (옵션, Day 12 이내)**

실패해도 founder 단독 통과로 진행 (스펙 §6.3).

- [ ] **Step 3: 커밋**

```bash
git add web/src/pages/en
git commit -m "copy(web): en polish pass"
```

### Task 7.2: Lighthouse audit + 수정

**Files:**
- 측정만(필요한 경우 부분 수정)

- [ ] **Step 1: PR preview URL에 대해 Lighthouse 측정**

방법 A: Chrome DevTools → Lighthouse 탭. 방법 B (CI로): `npx -y lighthouse <URL> --output=json --output-path=./lh.json --chrome-flags="--headless"`.

5개 ko 라우트에 대해 측정.

목표 (스펙 §9):
- Performance ≥ 90
- Accessibility ≥ 95
- Best Practices ≥ 95
- SEO ≥ 95

- [ ] **Step 2: 흔한 감점 항목 수정**

- 이미지에 `width`/`height` 미지정 → 추가
- `<a>` 빈 텍스트 → aria-label 추가
- 폰트 로딩 시 FOUT → `font-display: swap` 명시
- `<html lang>` 누락 → Base에서 이미 처리됨, 검증만

- [ ] **Step 3: 재측정 + 커밋**

```bash
git add web
git commit -m "perf(web): lighthouse fixes (img dims, a11y labels, font-display)"
```

### Task 7.3: a11y 점검

**Files:**
- 필요 시 수정

- [ ] **Step 1: 키보드 탐색**

`Tab` 키로 모든 인터랙티브 요소 도달 가능 + visible focus ring 확인. 누락 시 `:focus-visible` 스타일 추가.

- [ ] **Step 2: 색대비**

`#1a1814` on `#fafaf7` ≈ 14.5:1 (WCAG AAA 통과). 보조 색(ember, ink/70 등)도 4.5:1 이상 확인.

- [ ] **Step 3: 헤딩 계층**

각 페이지에 `<h1>` 정확히 1개. `<h2>` → `<h3>` 순서 깨지지 않게 확인.

- [ ] **Step 4: hreflang 검증**

```bash
curl -s https://<preview>/ko/ | rg 'rel="alternate"'
```

Expected: ko, en, x-default 3개 모두 출력.

- [ ] **Step 5: 커밋(수정이 있던 경우)**

```bash
git add web
git commit -m "a11y(web): focus-visible, heading hierarchy, hreflang verified"
```

---

## Phase 8 — Live deadline (Day 13, 2026-05-09)

### Task 8.1: DoD §9 체크리스트 한 번에 검증

**Files:**
- 검증 전용

- [ ] **Step 1: PR을 ready로 승격 + 머지**

```bash
gh pr ready
gh pr merge --merge
```

Expected: main 머지 → Cloudflare Pages production 빌드 자동 트리거.

- [ ] **Step 2: 라이브 도메인 라우트 200 일괄 확인**

```bash
for path in / /ko/ /en/ /ko/install/cli /en/install/cli /ko/install/skill /en/install/skill /ko/privacy /en/privacy /ko/support /en/support; do
  printf "%-25s -> %s\n" "$path" "$(curl -sI "https://hearth.codewithgenie.com$path" | head -n 1)"
done
```

Expected: 모두 `HTTP/2 200`.

- [ ] **Step 3: SSL + 도메인 확인**

```bash
curl -vI https://hearth.codewithgenie.com/ 2>&1 | rg -i 'subject|issuer|HTTP'
```

Expected: Cloudflare 서명 인증서, HTTP/2 200.

- [ ] **Step 4: hreflang Google rich results test**

브라우저로 https://search.google.com/test/rich-results → URL 입력 → "Page is eligible".

- [ ] **Step 5: Lighthouse production 1회 더 측정**

목표 점수 모두 통과 확인. 미달 시 hotfix PR.

- [ ] **Step 6: D 스펙(ASC 메타 입력)에 인계**

핸드오프: privacy URL = `https://hearth.codewithgenie.com/ko/privacy`, marketing URL = `https://hearth.codewithgenie.com/`, support URL = `https://hearth.codewithgenie.com/ko/support`. D 스펙 메타 단계에서 사용.

- [ ] **Step 7: 마무리 커밋(있다면) + worktree 정리는 sprint 종료 시 일괄**

---

# Track C — CLI/Tap distribution infra (Day 6 ~ Day 14)

> Track S와 병렬로 진행. 다른 worktree 또는 이 worktree의 별도 디렉토리에서 작업.

## Phase C1 — Public homebrew tap repo (Day 6, 2026-05-02)

### Task C1.1: `homebrew-hearth` public repo 생성

**Files:**
- 외부: GitHub `codewithgenie/homebrew-hearth`
- Create (해당 repo 안): `Formula/hearth-cli.rb`, `README.md`, `.github/workflows/test.yml`(옵션)

- [ ] **Step 1: GitHub에서 public repo 생성**

```bash
gh repo create codewithgenie/homebrew-hearth --public --description "Hearth CLI tap" --add-readme
gh repo clone codewithgenie/homebrew-hearth ~/tmp/homebrew-hearth
cd ~/tmp/homebrew-hearth
```

- [ ] **Step 2: skeleton formula 작성**

`Formula/hearth-cli.rb`:

```ruby
class HearthCli < Formula
  desc "CLI companion for Hearth — drive your local workspace from Claude Code"
  homepage "https://hearth.codewithgenie.com"
  version "0.0.0-skeleton"
  on_arm do
    url "https://dl.codewithgenie.com/cli/hearth-cli-0.0.0-skeleton-arm64.tar.gz"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end
  on_intel do
    url "https://dl.codewithgenie.com/cli/hearth-cli-0.0.0-skeleton-x86_64.tar.gz"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end
  def install
    bin.install "hearth-cli"
  end
  test do
    system "#{bin}/hearth-cli", "--version"
  end
end
```

- [ ] **Step 3: README 1줄 명시**

```markdown
# homebrew-hearth

This tap distributes the **CLI companion** for [Hearth](https://hearth.codewithgenie.com), a commercial macOS app on the Mac App Store. The CLI source is closed; binaries are Developer ID–signed and Apple-notarized.

## Install

    brew tap codewithgenie/hearth
    brew install hearth-cli

See https://hearth.codewithgenie.com/install/cli for details.
```

- [ ] **Step 4: 머지 (PR 없이 main 직접 푸시)**

```bash
git add Formula README.md
git commit -m "feat: skeleton hearth-cli formula"
git push
```

## Phase C2 — R2 버킷 + dummy 바이너리 + smoke test (Day 7, 2026-05-03)

### Task C2.1: R2 버킷 생성 + 공개 access

**Files:**
- 외부: Cloudflare R2 대시보드

- [ ] **Step 1: 버킷 `hearth-releases` 생성**

Cloudflare 대시보드 → R2 → Create bucket. Public access: Enable public.

- [ ] **Step 2: 도메인 결정 — `dl.codewithgenie.com` (권장)**

R2 → Settings → Custom domain → `dl.codewithgenie.com` 추가. Hostinger DNS에 CNAME 추가:
- Name: `dl`
- Value: R2가 안내한 CNAME target

- [ ] **Step 3: rclone 설정**

```bash
rclone config create r2 s3 \
  provider Cloudflare \
  access_key_id <R2_ACCESS_KEY> \
  secret_access_key <R2_SECRET> \
  endpoint https://<account-id>.r2.cloudflarestorage.com \
  acl public-read
```

(자격증명은 `pass`/keychain에 저장, 셸 히스토리 잔존 주의.)

### Task C2.2: dummy 바이너리 업로드 + brew install smoke

**Files:**
- 임시: `/tmp/hearth-cli-stub`
- Modify (homebrew tap): `Formula/hearth-cli.rb`

- [ ] **Step 1: stub 바이너리 + tarball 생성**

```bash
mkdir -p /tmp/hearth-stub && cd /tmp/hearth-stub
cat > hearth-cli <<'EOF'
#!/bin/sh
echo "hearth-cli 0.0.1-rc1 (stub)"
EOF
chmod +x hearth-cli
tar czf hearth-cli-0.0.1-rc1-arm64.tar.gz hearth-cli
tar czf hearth-cli-0.0.1-rc1-x86_64.tar.gz hearth-cli
SHA_ARM=$(shasum -a 256 hearth-cli-0.0.1-rc1-arm64.tar.gz | awk '{print $1}')
SHA_X86=$(shasum -a 256 hearth-cli-0.0.1-rc1-x86_64.tar.gz | awk '{print $1}')
echo "ARM=$SHA_ARM X86=$SHA_X86"
```

- [ ] **Step 2: R2 업로드**

```bash
rclone copy hearth-cli-0.0.1-rc1-arm64.tar.gz r2:hearth-releases/cli/
rclone copy hearth-cli-0.0.1-rc1-x86_64.tar.gz r2:hearth-releases/cli/
```

- [ ] **Step 3: 공개 접근 확인**

```bash
curl -I https://dl.codewithgenie.com/cli/hearth-cli-0.0.1-rc1-arm64.tar.gz
```

Expected: HTTP/2 200.

- [ ] **Step 4: tap formula 업데이트 (rc1)**

`Formula/hearth-cli.rb` 의 version, url, sha256 모두 rc1 값으로 교체. 머지.

- [ ] **Step 5: brew smoke test**

```bash
brew untap codewithgenie/hearth 2>/dev/null
brew tap codewithgenie/hearth
brew install hearth-cli
hearth-cli --version
```

Expected: `hearth-cli 0.0.1-rc1 (stub)`.

- [ ] **Step 6: 정리**

```bash
brew uninstall hearth-cli
brew untap codewithgenie/hearth
```

## Phase C3 — 1.0 RC CLI 빌드 + 서명 + notarize 워크플로 (Day 10, 2026-05-06)

### Task C3.1: GitHub Actions 워크플로 작성

**Files:**
- Create: `.github/workflows/release-cli.yml` (메인 hearth 레포)
- Reference: 기존 `cli/`(또는 `crates/hearth-cli/`) Rust 프로젝트

- [ ] **Step 1: 워크플로 파일 작성**

```yaml
name: release-cli

on:
  push:
    tags: ['cli-v*']
  workflow_dispatch:
    inputs:
      version:
        description: 'CLI version (e.g. 1.0.0)'
        required: true

jobs:
  build:
    runs-on: macos-14
    permissions: { contents: read }
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: 'aarch64-apple-darwin,x86_64-apple-darwin' }
      - name: Build (universal)
        run: |
          cargo build --release --target aarch64-apple-darwin -p hearth-cli
          cargo build --release --target x86_64-apple-darwin -p hearth-cli
      - name: Codesign
        env:
          CERT_P12_BASE64: ${{ secrets.DEVELOPER_ID_CERT_P12 }}
          CERT_PASSWORD: ${{ secrets.DEVELOPER_ID_CERT_PASSWORD }}
          KEYCHAIN_PASSWORD: ${{ secrets.KEYCHAIN_PASSWORD }}
        run: |
          echo "$CERT_P12_BASE64" | base64 --decode > /tmp/cert.p12
          security create-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p "$KEYCHAIN_PASSWORD" build.keychain
          security import /tmp/cert.p12 -k build.keychain -P "$CERT_PASSWORD" -T /usr/bin/codesign
          security set-key-partition-list -S apple-tool:,apple: -s -k "$KEYCHAIN_PASSWORD" build.keychain
          for arch in aarch64-apple-darwin x86_64-apple-darwin; do
            codesign --sign "Developer ID Application" --options runtime --timestamp \
              "target/$arch/release/hearth-cli"
          done
      - name: Notarize + staple
        env:
          AC_API_KEY_ID: ${{ secrets.AC_API_KEY_ID }}
          AC_API_KEY_ISSUER: ${{ secrets.AC_API_KEY_ISSUER }}
          AC_API_KEY_BASE64: ${{ secrets.AC_API_KEY_BASE64 }}
        run: |
          echo "$AC_API_KEY_BASE64" | base64 --decode > /tmp/AuthKey.p8
          for arch in aarch64-apple-darwin x86_64-apple-darwin; do
            cd "target/$arch/release"
            zip /tmp/hearth-cli-$arch.zip hearth-cli
            xcrun notarytool submit /tmp/hearth-cli-$arch.zip \
              --key /tmp/AuthKey.p8 --key-id "$AC_API_KEY_ID" --issuer "$AC_API_KEY_ISSUER" \
              --wait
            xcrun stapler staple hearth-cli || true   # bare binary cannot be stapled; ticket bundled in notarization receipt
            cd -
          done
      - name: Pack tarballs
        id: pack
        run: |
          VERSION="${{ github.event.inputs.version || github.ref_name }}"
          VERSION="${VERSION#cli-v}"
          mkdir -p dist
          tar czf "dist/hearth-cli-${VERSION}-arm64.tar.gz"   -C target/aarch64-apple-darwin/release hearth-cli
          tar czf "dist/hearth-cli-${VERSION}-x86_64.tar.gz"  -C target/x86_64-apple-darwin/release hearth-cli
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          echo "sha_arm=$(shasum -a 256 dist/hearth-cli-${VERSION}-arm64.tar.gz | awk '{print $1}')" >> "$GITHUB_OUTPUT"
          echo "sha_x86=$(shasum -a 256 dist/hearth-cli-${VERSION}-x86_64.tar.gz | awk '{print $1}')" >> "$GITHUB_OUTPUT"
      - name: Upload to R2
        env:
          R2_ACCESS_KEY_ID: ${{ secrets.R2_ACCESS_KEY_ID }}
          R2_SECRET_ACCESS_KEY: ${{ secrets.R2_SECRET_ACCESS_KEY }}
          R2_ENDPOINT: ${{ secrets.R2_ENDPOINT }}
        run: |
          brew install rclone
          rclone config create r2 s3 provider Cloudflare \
            access_key_id "$R2_ACCESS_KEY_ID" \
            secret_access_key "$R2_SECRET_ACCESS_KEY" \
            endpoint "$R2_ENDPOINT" \
            acl public-read
          rclone copy dist/ r2:hearth-releases/cli/
      - name: Bump tap formula via PR
        env:
          GH_TOKEN: ${{ secrets.HOMEBREW_TAP_PAT }}
          VERSION: ${{ steps.pack.outputs.version }}
          SHA_ARM: ${{ steps.pack.outputs.sha_arm }}
          SHA_X86: ${{ steps.pack.outputs.sha_x86 }}
        run: |
          gh repo clone codewithgenie/homebrew-hearth /tmp/tap
          cd /tmp/tap
          git checkout -b "bump-$VERSION"
          python3 - <<EOF
          import re, pathlib
          p = pathlib.Path("Formula/hearth-cli.rb")
          src = p.read_text()
          src = re.sub(r'version "[^"]+"', f'version "{${'VERSION'}}"', src)
          src = re.sub(r'(arm64\.tar\.gz")\s+sha256 "[0-9a-f]+"', f'\\1\n    sha256 "${'SHA_ARM'}"', src, flags=re.S)
          src = re.sub(r'(x86_64\.tar\.gz")\s+sha256 "[0-9a-f]+"', f'\\1\n    sha256 "${'SHA_X86'}"', src, flags=re.S)
          src = re.sub(r'hearth-cli-[0-9.A-Za-z\-]+-arm64', f'hearth-cli-{${'VERSION'}}-arm64', src)
          src = re.sub(r'hearth-cli-[0-9.A-Za-z\-]+-x86_64', f'hearth-cli-{${'VERSION'}}-x86_64', src)
          p.write_text(src)
          EOF
          git add Formula/hearth-cli.rb
          git commit -m "bump: hearth-cli $VERSION"
          git push -u origin "bump-$VERSION"
          gh pr create --title "bump: hearth-cli $VERSION" --body "Auto-generated."
```

> **Note (구현자에게):** 위 Python heredoc은 sed로 잘 안 잡히는 멀티라인 패턴 때문이다. 실 구현 시 더 견고한 형태로 다듬을 것 — 핵심은 version + 두 sha256 + 두 url 슬러그 갱신.

- [ ] **Step 2: 필요한 GitHub secrets 등록**

`gh secret set` 또는 GitHub UI:
- `DEVELOPER_ID_CERT_P12` (base64 인코딩된 .p12)
- `DEVELOPER_ID_CERT_PASSWORD`
- `KEYCHAIN_PASSWORD` (임의 강한 패스워드)
- `AC_API_KEY_ID`, `AC_API_KEY_ISSUER`, `AC_API_KEY_BASE64` (App Store Connect API key, base64 .p8)
- `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_ENDPOINT`
- `HOMEBREW_TAP_PAT` (`codewithgenie/homebrew-hearth`에 PR 푸시할 PAT, repo 권한)

- [ ] **Step 3: skill 임베드 확인**

`hearth-cli` 크레이트의 `build.rs` 또는 main.rs에서 `include_dir!("../skills/hearth")` 매크로로 디렉토리를 포함. Phase C3 시점에 이미 `crates/hearth-cli`에 구현되어 있다고 가정 (Day 1 핸드오프의 worktree 진행 상태 참조). 누락이 발견되면 별 task로 추가.

- [ ] **Step 4: 워크플로 dispatch (rc 버전)**

```bash
gh workflow run release-cli.yml -f version=1.0.0-rc1
```

job log에서 모든 단계 PASS 확인. R2에 새 tarball 출현 + tap repo에 PR 생성됨 확인.

- [ ] **Step 5: PR 머지 + brew install rc1 검증**

```bash
gh pr merge -R codewithgenie/homebrew-hearth --merge $(gh pr list -R codewithgenie/homebrew-hearth --json number -q '.[0].number')
brew untap codewithgenie/hearth 2>/dev/null
brew tap codewithgenie/hearth
brew install hearth-cli
hearth-cli --version  # → 1.0.0-rc1
hearth-cli install-skill
test -d ~/.claude/skills/hearth && echo OK
```

Expected: `OK`. skill 디렉토리 존재.

- [ ] **Step 6: 정리(다음 sprint 단계 위해 uninstall)**

```bash
hearth-cli uninstall-skill || rm -rf ~/.claude/skills/hearth
brew uninstall hearth-cli
brew untap codewithgenie/hearth
```

- [ ] **Step 7: 워크플로 파일 commit (메인 레포)**

```bash
git add .github/workflows/release-cli.yml
git commit -m "ci: release-cli workflow (sign, notarize, R2, tap PR)"
```

## Phase C4 — 1.0 GA build + tap PR 머지 + 최종 brew install (Day 14, 2026-05-10)

> D 스펙의 ASC 빌드 업로드와 같은 날.

### Task C4.1: GA 워크플로 dispatch + 검증

**Files:**
- 없음(워크플로 실행)

- [ ] **Step 1: GA 버전 dispatch**

```bash
gh workflow run release-cli.yml -f version=1.0.0
```

또는 태그 푸시: `git tag cli-v1.0.0 && git push origin cli-v1.0.0`.

- [ ] **Step 2: 워크플로 PASS + R2 객체 확인**

```bash
curl -I https://dl.codewithgenie.com/cli/hearth-cli-1.0.0-arm64.tar.gz
curl -I https://dl.codewithgenie.com/cli/hearth-cli-1.0.0-x86_64.tar.gz
```

Expected: 둘 다 HTTP/2 200.

- [ ] **Step 3: tap PR 머지**

```bash
PR=$(gh pr list -R codewithgenie/homebrew-hearth --json number -q '.[0].number')
gh pr merge -R codewithgenie/homebrew-hearth --merge "$PR"
```

- [ ] **Step 4: 깨끗한 환경에서 최종 brew install (가능하면 별도 Mac 또는 깨끗한 사용자 계정)**

```bash
brew tap codewithgenie/hearth
brew install hearth-cli
hearth-cli --version  # → 1.0.0
hearth-cli install-skill
ls ~/.claude/skills/hearth
```

Expected: skill 파일들 존재.

- [ ] **Step 5: 사이트 `/install/cli` 페이지의 사용자 흐름을 그대로 따라가며 친구 1명에게 cross-machine 검증 요청 (옵션)**

검증 결과를 D 스펙 review-notes에 추가 메모로 남길 것.

---

## Done-of-Definition (E 전체)

스펙 §9 + §8 종합. Day 13 종료 시:

- [ ] `https://hearth.codewithgenie.com/` → 200 OK
- [ ] `/ko/`, `/en/` → 200 OK
- [ ] `/ko/install/cli`, `/en/install/cli` → 200 OK
- [ ] `/ko/install/skill`, `/en/install/skill` → 200 OK
- [ ] `/ko/privacy`, `/en/privacy` → 200 OK
- [ ] `/ko/support`, `/en/support` → 200 OK
- [ ] Accept-Language 기반 locale 리다이렉트 동작 (1회만, 쿠키 기억 후 비활성)
- [ ] `<link rel="alternate" hreflang>` 모든 페이지에 정확히 3개(ko/en/x-default)
- [ ] Lighthouse Perf ≥ 90, A11y ≥ 95, Best Practices ≥ 95, SEO ≥ 95
- [ ] PR 머지 → Cloudflare Pages auto-deploy 검증 1회
- [ ] R2 RC 바이너리 + tap formula → `brew install hearth-cli` smoke test PASS
- [ ] 푸터에 © 2026, Privacy, Support 링크 정상

Day 14 종료 시 (Track C):
- [ ] R2에 1.0.0 GA 바이너리 업로드 완료
- [ ] tap formula 1.0.0 PR 자동 생성 + 머지
- [ ] 깨끗한 환경에서 `brew install hearth-cli` → `hearth-cli --version` = 1.0.0 확인
- [ ] `hearth-cli install-skill` → `~/.claude/skills/hearth/` 생성 확인

---

## Cross-references (구현 중 참조)

- A 스펙 §4-3: NSOpenPanel + security-scoped bookmark (Phase 4 support FAQ #1 답변 정합성)
- B 스펙: License 복원 흐름 (Phase 4 support FAQ #2 답변 정합성)
- D 스펙 §메타: privacy-policy.md(en+ko) — Phase 4 본문 1:1 재사용
- D 스펙 §7 R9: 외부 CLI 아키텍처 (1st-party companion 메시지) — Phase 3 `/install/cli` 카피 정합성
- D 스펙 §4.5: App Preview 영상 — Phase 6 차별점 섹션 임베드 의존
