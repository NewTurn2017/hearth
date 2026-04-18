# Hearth v0.2.0 — macOS 수동 릴리즈 인프라 (서명 · 공증 · 오토업데이트)

**Date:** 2026-04-18
**Status:** Accepted (design approved; plan to follow)

## Goal

Hearth 의 첫 **공개 정식 릴리즈**(v0.2.0)를 위해 macOS 배포 인프라를 구축한다. 한 번의 **로컬 수동 릴리즈**를 완주해 서명 · 공증 · 업데이터 경로를 실제로 검증하는 것이 목적이며, GitHub Actions 이행은 후속 스펙(Spec B)으로 분리한다.

구체적으로 이 스펙이 끝나면:

1. `Developer ID Application` 서명 + Apple notarytool 공증 + stapled universal DMG 를 `scripts/release.sh` 한 방으로 생성
2. GitHub Releases 에 DMG + `.app.tar.gz` + `.sig` + `latest.json` 4 애셋이 일관되게 업로드
3. 구버전 Hearth 앱이 `tauri-plugin-updater` 로 새 버전을 자동 감지 → "지금 재시작 / 나중에" 토스트 → 2-3초 내 relaunch
4. 메타데이터(Cargo.toml authors / description / license, tauri.conf.json CSP 등)가 공개용으로 정돈됨
5. CHANGELOG · README Installation · docs/releasing.md 가 최신 상태

## Non-Goals

- **Windows / Linux 빌드 · 서명** — 별도 스펙, 수요 발생 시
- **GitHub Actions 릴리즈 파이프라인** — Spec B (수동 1회 후 이행)
- **OpenAI API 키 Keychain 이관** — 별도 소규모 스펙, v0.2.0 태깅 **직전**에 병렬 머지
- **Homebrew Cask / tap** — 별도 스펙, 실수요 생기면
- **앱 내 텔레메트리 · 크래시 리포트** — 로컬 퍼스트 약속 유지, 도입 안 함
- **롤백 / 다운그레이드 UI** — 0.x 기간엔 "최신 설치" 로 충분
- **스테이징 · 베타 채널 분리** — 0.x 통틀어 단일 `stable` 채널
- **스펙 내에서 Cargo.toml `[lib] name` 변경을 우회** — `tauri_app_lib` → `hearth_lib` 치환은 스펙 범위 내

## Architecture Overview

```
┌─────────────── Frontend ───────────────┐    ┌──────────── Backend (Rust) ────────────┐
│  useAppUpdater()         [NEW]         │    │  tauri-plugin-updater       [NEW]      │
│    30s + 24h check                     │    │  tauri-plugin-process       [NEW]      │
│    "지금 재시작 / 나중에" Toast          │    │  capabilities/default.json  [MODIFY]   │
│    dismissedVersion in localStorage    │    │    + updater:default                   │
│                                        │    │    + process:allow-restart             │
│  Toast primitive         [REUSE]       │    │                                        │
│    Undo-토스트 확장 (수동 닫기 버튼)     │◀──▶│  src/lib.rs           [MODIFY]         │
│                                        │    │    .plugin(updater).plugin(process)    │
└────────────────────────────────────────┘    │                                        │
                                              │  Cargo.toml            [MODIFY]        │
                                              │    name/desc/license/repo/authors      │
                                              │    [lib] hearth_lib                    │
                                              │    + tauri-plugin-updater = "2"        │
                                              │    + tauri-plugin-process = "2"        │
                                              │                                        │
                                              │  tauri.conf.json       [MODIFY]        │
                                              │    version 0.1.0 → 0.2.0               │
                                              │    CSP 정책 도입                        │
                                              │    bundle.macOS.* signing identity     │
                                              │    bundle.createUpdaterArtifacts:true  │
                                              │    plugins.updater.{pubkey,endpoints}  │
                                              │                                        │
                                              │  entitlements.plist   [NEW]            │
                                              │    cs.allow-jit / allow-unsigned-mem   │
                                              │    network.client / files.user-selected│
                                              └────────────────────────────────────────┘

┌─────────────── Release Infra (this spec) ───────────────┐
│  scripts/release.sh           [NEW]                     │
│    preflight → build → codesign verify → notarize       │
│    → staple → sign .app.tar.gz → latest.json →          │
│    → gh release create → post-verify                    │
│                                                         │
│  scripts/generate-manifest.sh [NEW]                     │
│  scripts/extract-release-notes.sh [NEW]                 │
│  .env.release.example         [NEW]                     │
│                                                         │
│  CHANGELOG.md                 [NEW]  (Keep-A-Changelog) │
│  docs/releasing.md            [NEW]  (체크리스트 · Tier 3│
│                                        updater 드라이런) │
│  README.md                    [MODIFY] Installation     │
│  package.json                 [MODIFY] metadata + script│
└─────────────────────────────────────────────────────────┘

                ┌──── Credentials (never in repo) ────┐
                │  /Users/genie/dev/private/          │
                │    apple_developer/                 │
                │  ├─ 인증서.p12  (.p12 export)        │
                │  ├─ AuthKey_Z2V325X3FY.p8           │
                │  ├─ AuthKey_82J37G9FXF.p8           │
                │  ├─ hearth_updater.key      [NEW]   │
                │  └─ hearth_updater.key.pub  [NEW]   │
                │                                     │
                │  keychain (macOS):                  │
                │    Developer ID Application:        │
                │      jaehyun jang (2UANJX7ATM)      │
                │  Team ID: 2UANJX7ATM                │
                │  Issuer:  a3452585-1d50-4353-...    │
                └─────────────────────────────────────┘
```

## Decisions (summary)

| 질문 | 선택 |
|------|------|
| 1차 릴리즈 대상 · 서명 전략 | **B** — macOS 우선, Developer ID + notarize |
| 오토업데이터 포함 여부 | **A** — 포함 (Ed25519 + `tauri-plugin-updater`) |
| 빌드 파이프라인 | **C** — 로컬 수동 1회 → 이후 CI 이행 (Spec B) |
| 버전 전략 | **D** — `0.1.0` → `0.2.0`, 0.x 동안 점진 |
| 배포 채널 | **A** — GitHub Releases 전용 |
| 업데이트 체크 UX | **D** — 시작 후 30s + 24h 주기, 토스트로 "지금 재시작 / 나중에" |
| OpenAI 키 저장 | **C** — 이 스펙 범위 외, 별도 소규모 스펙 |

## Metadata & Manifest Cleanup

### `src-tauri/Cargo.toml`

- `name = "tauri-app"` → `"hearth"`; `[lib] name = "tauri_app_lib"` → `"hearth_lib"`, `src-tauri/src/main.rs` 의 `tauri_app_lib::run()` 를 `hearth_lib::run()` 으로 치환
- `version = "0.1.0"` → `"0.2.0"`
- `authors = ["you"]` → `["Jaehyun Jang <hyuni2020@gmail.com>"]`
- `description = "A Tauri App"` → `"Local-first personal workspace for projects, memos, and schedules."`
- 신규 키: `license = "MIT"`, `repository = "https://github.com/NewTurn2017/hearth"`, `readme = "../README.md"`, `rust-version = "1.75"`
- 신규 디펜던시: `tauri-plugin-updater = "2"`, `tauri-plugin-process = "2"`

### `src-tauri/tauri.conf.json`

- `version` `0.1.0` → `0.2.0` (3곳 version 동기는 `scripts/bump-version.sh` 전담)
- `app.security.csp` `null` → 아래 정책 (구현 시 `tauri dev` 로 DevTools 위반 보며 최종 튜닝)
  - `default-src 'self'`
  - `connect-src 'self' https://api.openai.com http://127.0.0.1:18080 http://localhost:18080 ipc: http://ipc.localhost`
  - `script-src 'self'`
  - `style-src 'self' 'unsafe-inline'` (Tailwind 4 런타임 inline)
  - `img-src 'self' data:`
  - `font-src 'self' data:`
  - `object-src 'none'`
- `bundle` 확장:
  - `category: "public.app-category.productivity"`
  - `shortDescription`, `longDescription`, `publisher: "Jaehyun Jang"`, `copyright: "© 2026 Jaehyun Jang"`, `homepage: "https://github.com/NewTurn2017/hearth"`
  - `createUpdaterArtifacts: true`
  - `macOS.signingIdentity: "Developer ID Application: jaehyun jang (2UANJX7ATM)"`
  - `macOS.providerShortName: "2UANJX7ATM"`
  - `macOS.entitlements: "entitlements.plist"`
  - `macOS.minimumSystemVersion: "11.0"`
- `plugins.updater`:
  - `pubkey`: `hearth_updater.key.pub` 내용 (Base64)
  - `endpoints: ["https://github.com/NewTurn2017/hearth/releases/latest/download/latest.json"]`

### `package.json`

- `version: "0.2.0"`, `author`, `license: "MIT"`, `repository.{type,url}`, `bugs.url`, `homepage`
- 스크립트 추가: `"release": "bash scripts/release.sh"`

### `src-tauri/entitlements.plist` (신규)

```xml
com.apple.security.cs.allow-jit                        = true
com.apple.security.cs.allow-unsigned-executable-memory = true
com.apple.security.network.client                      = true
com.apple.security.files.user-selected.read-write      = true
```

- 샌드박스 비활성 (Developer ID + 비-App-Store, `~/Library/Application Support/` 접근 필요)
- Hardened Runtime 은 `codesign --options runtime` 로 강제

### README / CHANGELOG

- `CHANGELOG.md` 신설, [Keep-A-Changelog](https://keepachangelog.com/) 포맷. 첫 엔트리 `[0.2.0] - 2026-04-18`
- README Installation 섹션: 플레이스홀더 제거 → **DMG 다운로드 + Applications 드래그 + 첫 실행 우클릭 → 열기** 안내 + 업데이트 동작 1문단
- README 에 짧은 "Releasing" 섹션 (상세는 `docs/releasing.md` 로 링크)

## macOS Signing & Notarization

### Signing identity

- `Developer ID Application: jaehyun jang (2UANJX7ATM)` — 이미 로컬 키체인 (2031-03 만료)
- `.p12` 백업은 `/Users/genie/dev/private/apple_developer/인증서.p12` — Spec B (CI) 에서 base64 인코딩하여 GH Secret 으로 등록

### Target architecture

- `tauri build --target universal-apple-darwin` — aarch64 + x86_64 단일 universal DMG
- 산출물:
  - `src-tauri/target/universal-apple-darwin/release/bundle/dmg/Hearth_0.2.0_universal.dmg`
  - `.../bundle/macos/Hearth.app`
  - `.../bundle/macos/Hearth.app.tar.gz` + `.sig` (Tauri 가 `createUpdaterArtifacts: true` 로 생성)

### Signing flow

1. `tauri build` 내부에서 `codesign --options runtime --entitlements entitlements.plist --sign "Developer ID Application: jaehyun jang (2UANJX7ATM)" --timestamp` 실행
2. 스크립트가 `codesign --verify --deep --strict --verbose=2 Hearth.app` 명시 검증

### Notarization (App Store Connect API key 방식)

- 자격증명 (`.env.release` 에서 로드):
  - `APPLE_API_KEY_PATH=/Users/genie/dev/private/apple_developer/AuthKey_Z2V325X3FY.p8`
  - `APPLE_API_ISSUER=a3452585-1d50-4353-bc5b-6a36da9452ad`
  - `APPLE_API_KEY_ID=Z2V325X3FY`
- `xcrun notarytool submit Hearth_0.2.0_universal.dmg --key … --key-id … --issuer … --wait --output-format json`
- 실패 시 `xcrun notarytool log <submission-id>` 출력 후 중단

### Stapling

- `xcrun stapler staple Hearth_0.2.0_universal.dmg`
- `xcrun stapler staple .../bundle/macos/Hearth.app`
- Staple 후 `.app.tar.gz` 재생성 (`tar -czf Hearth.app.tar.gz Hearth.app`) — updater tarball 은 **stapled bits 를 포함**해야 함
- 검증: `xcrun stapler validate Hearth_0.2.0_universal.dmg` + `spctl --assess --type execute --verbose Hearth.app` (상태 `Notarized Developer ID` 기대)

### Credential 보관 원칙

- `.p12`, `.p8`, `hearth_updater.key`(passphrase 암호화) 모두 `/Users/genie/dev/private/apple_developer/` 에 유지
- 레포에는 `.env.release.example` 만 커밋, 실제 `.env.release` 은 `.gitignore`
- 키체인에 서명 identity 가 이미 있어 `codesign` 단계는 추가 입력 불필요 (*본인 Mac 전제*; 다른 기기에서 빌드하려면 `.p12` 임포트 필요 — Spec B 에서 다룸)

## Auto-updater Architecture

### 키쌍 생성 · 보관

- 1회: `tauri signer generate -w /Users/genie/dev/private/apple_developer/hearth_updater.key` (passphrase 입력)
- `hearth_updater.key` (private, passphrase 암호화) + `hearth_updater.key.pub`
- public key(Base64) → `tauri.conf.json > plugins.updater.pubkey`
- **private key 는 `/Users/genie/dev/private/apple_developer/` 밖으로 나가지 않음.** 분실 시 모든 구버전 사용자에게 수동 재설치 공지 필요 → 최소 2곳(외장 SSD 등)에 복호화 암호와 함께 오프라인 백업

### Rust 배선

- `Cargo.toml` 신규 디펜던시 (메타 섹션 참조)
- `src/lib.rs` 의 `tauri::Builder::default()` 체인에 `.plugin(tauri_plugin_updater::Builder::new().build()).plugin(tauri_plugin_process::init())`
- `capabilities/default.json` 에 `updater:default` + `process:allow-restart` 권한

### 프론트엔드 훅 (`src/hooks/useAppUpdater.ts` 신규)

```ts
mount:
  setTimeout(check, 30_000)
  setInterval(check, 24 * 60 * 60 * 1000)

check():
  const update = await check()              // @tauri-apps/plugin-updater
  if (!update?.available) return
  if (localStorage['updater.dismissedVersion'] === update.version) return
  showToast({
    msg: `새 버전 ${update.version} 준비됨`,
    sticky: true,
    actions: [
      { label: '지금 재시작', run: async () => {
          await update.downloadAndInstall()
          await relaunch()
        }},
      { label: '나중에', run: () => {
          localStorage['updater.dismissedVersion'] = update.version
        }},
    ],
  })
```

- "나중에" → 해당 버전만 dismiss. 다음 버전 나오면 다시 토스트
- 토스트는 기존 Undo 토스트 프리미티브 확장 (`sticky` 플래그로 무한 유지 + 수동 닫기 버튼)
- 유닛 테스트: version 비교 로직, dismiss 저장, 오프라인 silent fail (Vitest, `check()` 는 모킹)

### 매니페스트 (`latest.json`)

```json
{
  "version": "0.2.0",
  "notes": "<CHANGELOG 0.2.0 섹션 본문>",
  "pub_date": "2026-04-18T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<Tauri signer sign 출력>",
      "url": "https://github.com/NewTurn2017/hearth/releases/latest/download/Hearth.app.tar.gz"
    },
    "darwin-x86_64": {
      "signature": "<동일>",
      "url": "<동일>"
    }
  }
}
```

- 두 arch 키 모두 같은 universal tarball 을 지칭 (`darwin-universal` 호환성 불확실 → 양쪽 명시)
- `signature` 는 `tauri signer sign -k hearth_updater.key Hearth.app.tar.gz` 산출물
- URL 은 **고정 퍼머링크** (`/releases/latest/download/…`) — 매 릴리즈마다 매니페스트 수정 불필요

### 실패 모드

- 네트워크 오프라인 → `check()` 예외 swallow, 디버그 로그만
- 서명 불일치 → plugin 이 다운로드 거부 (공개키 내장 검증)
- 앱이 `/Applications/Hearth.app` 가 아닌 위치에서 실행 → README Installation 에서 **반드시 Applications 드래그** 명시

## Release Script (`scripts/release.sh`)

### Preflight (하나라도 실패 시 early exit)

- `git status --porcelain` 비어 있음
- 현재 브랜치 `main`
- `cargo test` + `npm test` 초록 (스킵 `--skip-tests`)
- 3곳 `version` 동일 (version bump 은 별도 `scripts/bump-version.sh`)
- 대상 태그 `vX.Y.Z` 가 원격에 없음
- `CHANGELOG.md` 에 `[X.Y.Z]` 엔트리 존재
- 필수 env 로드: `APPLE_API_KEY_PATH`, `APPLE_API_ISSUER`, `APPLE_API_KEY_ID`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
- `gh auth status` 인증됨, 키체인에 Developer ID 존재, `xcrun notarytool`/`stapler` 사용 가능
- `rustup target list --installed` 에 `aarch64-apple-darwin` + `x86_64-apple-darwin` 둘 다 있음 (universal build 전제 — 없으면 스크립트가 설치 커맨드 안내 후 중단)

### 실행 단계 (순차, 중단 시 다음 단계 skip)

1. `VERSION=$(jq -r .version package.json)`, `TAG="v$VERSION"`
2. `npm ci` + `cd src-tauri && cargo fetch` (reproducible)
3. `npm run tauri build -- --target universal-apple-darwin` (Tauri 가 codesign + updater tarball + `.sig` 까지)
4. **서명 검증**: `codesign --verify --deep --strict --verbose=2 Hearth.app`
5. **공증 제출**: `xcrun notarytool submit "$DMG" … --wait --output-format json` — status `Accepted` 외 중단 (+ `notarytool log`)
6. **Staple**: `xcrun stapler staple "$DMG"` + `xcrun stapler staple "$APP"` + `.app.tar.gz` **재생성**
7. **Updater 서명**: `npx tauri signer sign -k $TAURI_SIGNING_PRIVATE_KEY -p $TAURI_SIGNING_PRIVATE_KEY_PASSWORD Hearth.app.tar.gz`
8. **`latest.json` 생성**: `scripts/generate-manifest.sh $VERSION` → `dist/release/latest.json`
9. **릴리즈 노트 추출**: `scripts/extract-release-notes.sh $VERSION` → `dist/release/notes.md`
10. `git tag -s "$TAG" -m "Hearth $VERSION"` + `git push origin "$TAG"`
11. `gh release create "$TAG" --title "Hearth $VERSION" --notes-file dist/release/notes.md dist/release/latest.json $DMG Hearth.app.tar.gz Hearth.app.tar.gz.sig`
12. **Post-verify**: `curl -sL …/releases/latest/download/latest.json | jq .version` == `$VERSION`

### 모드 플래그

- `--dry-run` — 1~9 까지만, 태그 푸시·Release 생성 skip
- `--skip-tests` — 테스트 재실행 생략
- `--verbose` — `set -x`

### 아이덴포턴시 · 복구

- 1~9 는 재실행 안전 (결과물 덮어쓰기, notarize 는 동일 DMG 재제출 허용)
- 10~11 은 태그/Release 존재 시 거부 → 수동 정리 (`gh release delete $TAG && git push --delete origin $TAG`) 후 재실행
- 공증 중 sleep → `xcrun notarytool info <id>` 로 상태 확인 후 수동 이어받기

### 최종 출력

- 버전 · 태그 · DMG size · 공증 submission id · Release URL · updater 매니페스트 URL
- 다음 단계 체크리스트 (Tier 2 스모크, 오토업데이트 라운드트립)

## GitHub Releases Layout

### 태그 / 릴리즈

- 태그: `v0.2.0` (SemVer, `v` 프리픽스)
- 제목: `Hearth 0.2.0`
- Pre-release off (0.x 기간 단일 `stable` 채널)
- Draft 없음, 바로 publish

### 애셋 4개 (매 릴리즈 동일)

| 파일 | 용도 | 파일명 규칙 |
|------|------|------------|
| `Hearth_0.2.0_universal.dmg` | 최종 사용자 다운로드 | 버전 · 아키텍처 포함 |
| `Hearth.app.tar.gz` | Updater payload (stapled) | **고정명** (매니페스트 URL 단순화) |
| `Hearth.app.tar.gz.sig` | Updater 서명 | 고정명 |
| `latest.json` | Updater 매니페스트 | 고정명 |

### 릴리즈 노트 본문

- `extract-release-notes.sh` 가 CHANGELOG 의 `## [X.Y.Z] - YYYY-MM-DD` 섹션 바디만 추출
- 자동 부록 2문단:
  - macOS 설치 안내 ("DMG 더블클릭 → Applications → 첫 실행만 우클릭 → 열기")
  - 기존 사용자 안내 ("앱이 켜져 있으면 자동으로 업데이트 토스트가 뜹니다")

### 퍼머링크

- `https://github.com/NewTurn2017/hearth/releases/latest/download/<filename>` — GitHub 이 latest non-prerelease 로 302
- 업데이터 endpoint 는 `latest.json` 한 URL 만 알면 됨 → 매 릴리즈 tauri.conf.json 수정 불요

## Testing & Verification

### Tier 1 — 자동화 (스크립트 내부)

- `cargo test` (21) + `npm test` (19) — Preflight
- `codesign --verify --deep --strict --verbose=2`
- `xcrun stapler validate` + `spctl --assess --type execute --verbose` (staple 후)
- `notarytool` 상태 `Accepted` 외 중단
- 매니페스트 정합성 check (`curl` + `jq`)

### Tier 2 — 공개 직후 스모크 (`docs/releasing.md`)

**방법:** macOS 게스트/테스트 유저로 로그인 (Gatekeeper 첫 실행 재현)

1. GH Releases latest → DMG → Applications 드래그
2. 첫 실행 우클릭 → 열기 ("알 수 없는 개발자" 경고 없어야 정상, 공증 확인 프롬프트는 네트워크 지연 시 1회 가능)
3. 메인 창 렌더 (사이드바, 프로젝트/메모/캘린더 탭)
4. `⌘K` 명령 팔레트
5. 새 프로젝트 → 앱 종료 · 재시작 → 지속 확인
6. 설정 → AI 탭 → OpenAI 키로 `"프로젝트 목록 보여줘"` 응답
7. 설정 → AI 탭 → provider `local` + MLX 서버 → 응답
8. 설정 → 백업 탭 → 지금 백업 → 파일 생성
9. `~/Library/Application Support/com.newturn2017.hearth/data.db` 존재
10. 항목 1개라도 실패 → **yank + 재빌드**

### Tier 3 — Updater 라운드트립

- **Pre-flight 드라이런** (v0.2.0 태깅 전):
  - 로컬 빌드 `fake-0.1.9.app` (version=0.1.9, 같은 pubkey)
  - 게스트 계정에 설치
  - 임시 endpoint 가리킨 빌드로 토스트 유도 → "지금 재시작" → 2-3초 내 새 버전, DB 보존 확인
- **실전 검증** (v0.2.1 릴리즈 때):
  - v0.2.0 본인 Mac 에서 토스트 UX 재확인
  - "나중에" 다음 버전 재알림
  - 오프라인 silent 확인

Tier 3 드라이런은 **Tier 2 보다 먼저** — 업데이터가 부서지면 사용자가 0.2.0 에 고립되므로.

### 안티테스트 (1회)

- `latest.json` version 비움 → 토스트 미표시, 콘솔만
- `.sig` 1byte 변조 → plugin 다운로드 거부
- 오프라인 기동 → 30s 체크 silent fail, 정상 사용

## Rollout Plan

### 구현 순서

1. 메타데이터 정리 + `hearth_lib` 치환 (단독 PR, 기존 테스트 초록)
2. CSP 정책 도입 — `tauri dev` DevTools 위반 관찰 → 튜닝
3. Entitlements + signing config 추가 (실제 서명은 릴리즈 단계에서만 발생)
4. `tauri signer generate` → pubkey 를 tauri.conf.json 에 커밋
5. Updater Rust 배선 (`lib.rs`, `capabilities/default.json`)
6. 프론트엔드 `useAppUpdater` + Toast 확장 + Vitest
7. 릴리즈 스크립트 뼈대 + preflight (`--dry-run`)
8. 빌드/서명/검증 단계 추가 → `--dry-run`
9. Notarize + staple + updater 서명 추가 → 드라이런 (공증 제출 실제 1회)
10. `gh release create` 단계 + 퍼머링크 검증 (첫 진짜 릴리즈로 겸함)
11. CHANGELOG 0.2.0 작성, README Installation 교체, `docs/releasing.md` 체크리스트
12. Tier 3 updater 드라이런
13. **Release Day**: Tier 2 스모크 → v0.2.0 태깅 → Release publish

1~6 순차 PR, 7~10 같은 브랜치 이터레이션, 11~13 릴리즈 주간 집중.

### Release Day 체크리스트 (요약)

- [ ] main clean · tests green
- [ ] CHANGELOG 0.2.0 본문 확정 (user-facing 톤)
- [ ] `.env.release` 자격증명 로드 확인
- [ ] `scripts/release.sh --dry-run`
- [ ] Tier 3 updater 드라이런 OK
- [ ] `scripts/release.sh` (실행 ~15분, 대부분 notarize 대기)
- [ ] Tier 2 스모크 on 게스트 계정
- [ ] README DMG 링크 동작 확인
- [ ] (선택) 공지

### Rollback 전략

- **1순위: v0.2.1 롤포워드.** 버그 수정 → 스크립트 재실행 → 업데이터가 자동 푸시 (오토업데이트의 원래 용도)
- **치명적 보안 문제만 yank.** 업로드 후 24h 이후엔 퍼머링크 fallback 때문에 yank 대신 롤포워드 권장.
- **다운그레이드 지원 안 함** (0.x SemVer 규정, 스키마 forward-only)

### Post-release (D+1 ~ D+7)

- GH Issues 라벨 `release-0.2.0` 로 설치/Gatekeeper/충돌/업데이터 실패 모니터
- 7일 무사 → Spec B (CI 이행) 착수 가능

### 유저 커뮤니케이션

- README 상단에 shields.io GH release 배지 추가 (자동 갱신)
- Installation 섹션에 Gatekeeper 첫 실행 안내 1문단
- 업데이트는 "앱이 알려줍니다" — 추가 작업 없음

## Open Questions

없음. 모든 결정 사항은 위 "Decisions" 표에 반영됨.
