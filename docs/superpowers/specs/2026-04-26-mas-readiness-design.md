# A. Mac App Store 적합성 리팩터링 — 설계 문서

- **Date:** 2026-04-26
- **Sub-project:** A (출시 8개 서브프로젝트 중 1번째)
- **Target version:** Hearth 1.0.0
- **Target launch:** 2026-05-17
- **Estimated effort:** Day 1–7 (7일)
- **Owner:** Jaehyun Jang

---

## 0. 배경 & 상위 결정

Hearth 1.0.0을 **Mac App Store 단독 채널**로 유료(비소모성 IAP) 출시한다. 본 문서는 1.0의 8개 서브프로젝트 중 첫 번째 — **MAS 적합성 리팩터링** — 의 설계다.

### 상위 전략 결정 (사용자 승인 완료)

| ID | 결정 | 근거 |
|---|---|---|
| **D1** | **MAS-only** (Direct DMG 빌드 폐기) | 라이선스 시스템 단일화, 3주 일정 보호 |
| **D2** | **자동 데이터 마이그레이션 (M1)** | 기존 0.x OSS 사용자 보호 |
| **D3** | **Autostart 1.0에서 제외 (X1)** | SMAppService 통합은 1.1로 연기 |
| **D4** | **`hearth` 본체 private, `hearth-cli` public 유지** | 라이선스 보호는 앱 본체에서, CLI는 OSS 부가기능 |

### Goal
3주 sprint 중 첫 7일 안에 Hearth 코드베이스를 MAS 빌드·서명·업로드 가능 상태로 만든다.

### Non-Goals
- IAP/라이선스 게이트 구현 (서브프로젝트 B)
- 레포 분리 작업 (서브프로젝트 C-lite)
- 랜딩 페이지·마케팅 자산 (서브프로젝트 E·G)
- Universal binary (Intel) 지원 — 1.1
- CI 파이프라인 — 1.1
- Autostart 재구현 — 1.1

---

## 1. 현재 상태 감사

### 워크스페이스 구조 (긍정)
- `src-tauri/` 하위에 `app`, `core`, `cli` 3-crate 구성 — 향후 레포 분리 시 매우 가벼움.

### MAS 호환성 매트릭스

| 영역 | 현 상태 | MAS 영향 | 처리 |
|---|---|---|---|
| **Auto-updater** | `tauri-plugin-updater` v2, GitHub Releases endpoint, 24h 자동 체크 + Settings/TopBar UI | 🚫 BLOCKER | 완전 제거 (§3) |
| **Quick Capture 전역 단축키** | `tauri-plugin-global-shortcut` v2, `RegisterEventHotKey` 사용 | ✅ OK | Carbon API는 샌드박스 호환. 코드 변경 없음 (§4-2) |
| **Autostart** | `tauri-plugin-autostart` v2, `MacosLauncher::LaunchAgent` 직접 사용 | 🚫 1.0 BLOCKER | 1.0에서 제거 (X1) (§4-1) |
| **SQLite 경로** | `app.path().app_data_dir()` 추상화 | ✅ OK | Tauri가 컨테이너 경로로 자동 리매핑 |
| **사용자 선택 폴더 RW** | `entitlements.plist`에 이미 존재 | ✅ OK | 변경 없음 |
| **Finder/Ghostty 외부 호출** | `open` 명령어 사용 | ⚠️ 부분 | `NSWorkspace` API로 전환 (§4-4) |
| **OpenAI HTTPS 호출** | `https://api.openai.com` | ✅ OK | `network.client` entitlement |
| **일정 알림** | `tauri-plugin-notification` 사용 | ✅ OK | 첫 호출 시 권한 다이얼로그 자동 |

### 영향받는 파일 인벤토리

**Updater 관련 (제거 대상):**
- `src-tauri/app/Cargo.toml` — `tauri-plugin-updater = "2"`
- `src-tauri/app/src/lib.rs:32` — plugin init
- `src-tauri/app/tauri.conf.json` — `bundle.createUpdaterArtifacts`, `plugins.updater` 블록
- `src/hooks/useAppUpdater.ts` — 파일 삭제
- `src/components/SettingsGeneralSection.tsx:146` — "업데이트 확인" 버튼 → 안내 카드로 교체
- `src/components/TopBar.tsx:90` — 업데이트 배지 제거
- `.github/workflows/release.yml` (있는 경우) — `latest.json` 생성 스텝 제거

**Autostart 관련 (제거 대상):**
- `src-tauri/app/Cargo.toml` — `tauri-plugin-autostart = "2"`
- `src-tauri/app/src/lib.rs:34-36` — plugin init
- `src-tauri/app/src/cmd_autostart.rs` — 파일 삭제
- `src/components/SettingsGeneralSection.tsx` — 자동실행 토글 UI → "1.1 예정" 안내 카드로 교체

**신규 파일:**
- `src-tauri/app/src/cmd_migration.rs` — 첫 실행 데이터 마이그레이션 명령
- `src/components/MigrationWizard.tsx` — 마이그레이션 마법사 UI
- `scripts/build-mas.sh` — MAS 빌드 스크립트
- `scripts/upload-mas.sh` — App Store Connect 업로드
- `scripts/sync-version.js` — 버전 동기화
- `scripts/bump-build-number.js` — 빌드 번호 증가
- `scripts/check-signing.sh` — 인증서 사전 검증
- `build-number.json` — CFBundleVersion 추적 (커밋 대상)
- `certs/Hearth_MAS.provisionprofile` — 프로비저닝 (gitignore)

---

## 2. 샌드박스 & Entitlements

### 최종 `entitlements.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>

  <key>com.apple.security.network.client</key>
  <true/>

  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>

  <key>com.apple.security.files.bookmarks.app-scope</key>
  <true/>
</dict>
</plist>
```

**제거되는 것:** Developer ID 빌드의 hardened runtime 옵션 (MAS는 자동 적용), 자동 업데이터 관련 광범위한 네트워크 권한.

### `Info.plist` 추가

| 키 | 값 | 출처 | 이유 |
|---|---|---|---|
| `LSApplicationCategoryType` | `public.app-category.productivity` | tauri.conf 자동 | 이미 있음 |
| `ITSAppUsesNonExemptEncryption` | `<false/>` | 수동 추가 필요 | HTTPS만 사용 → 수출 규정 면제 |
| `NSHumanReadableCopyright` | `© 2026 Jaehyun Jang` | tauri.conf 자동 | 이미 있음 |

`NSAppleEventsUsageDescription` 은 Finder/Ghostty 호출을 `NSWorkspace`로 전환하면 불필요 (§4-4 참조).

---

## 3. 자동 업데이터 외과수술

### 원칙
MAS-only 결정에 따라 `#[cfg]` 분기 없이 완전 제거. 기존 OSS 0.9.x는 그 자리에 동결되므로 1.0+ 코드는 더 단순해진다.

### 변경 파일 (7곳)
§1 인벤토리 참조.

### Settings 화면 교체 디자인

기존 "업데이트 확인" 버튼 자리 → 정보 카드:

```
┌─ 업데이트 ─────────────────────────────┐
│  현재 버전: Hearth 1.0.0                │
│  Hearth는 Mac App Store를 통해           │
│  자동으로 업데이트됩니다.                 │
│                                         │
│  [ App Store에서 보기 → ]               │
└─────────────────────────────────────────┘
```

`App Store에서 보기` 버튼:
```js
open(`macappstore://apps.apple.com/app/id${APP_STORE_ID}`)
```
`APP_STORE_ID` 는 D 서브프로젝트(App Store Connect 등록) 완료 시 환경변수로 주입.

### TopBar 처리
업데이트 배지 컴포넌트 완전 제거. 남는 자리는 기존 flex 레이아웃이 자연스럽게 흡수.

### 정리 검증
- `rg -i 'updater|update.*available|useAppUpdater|__TAURI_UPDATER__' src/ src-tauri/` → 0건
- `cargo build` 워닝 0건
- 빌드 산출물에서 `Hearth.app.tar.gz` 사라짐 확인

---

## 4. Autostart 제거 · Quick Capture · 마이그레이션

### 4-1. Autostart 제거 (X1)

§1 인벤토리 참조. UI 교체 카피:

```
┌─ 로그인 시 자동 실행 ─────────────────┐
│  Mac App Store 정책 호환을 위해         │
│  1.1에서 지원될 예정입니다.             │
│                                         │
│  대안: System Settings → 일반 →         │
│        로그인 항목에 Hearth 추가        │
└────────────────────────────────────────┘
```

토글 자리에 안내 카드를 두는 이유: "지원 안 함"이 아니라 "1.1 예정 + 지금은 OS 기능으로 대체"로 포지셔닝.

### 4-2. Quick Capture (⌃⇧H) — 샌드박스 검증

**코드 변경 없음.** Carbon `RegisterEventHotKey` API는 샌드박스에서 별도 권한·entitlement 없이 동작.

추가 작업:
1. **충돌 검증** — macOS 14/15 시스템 예약 단축키와 ⌃⇧H가 충돌 안 하는지 확인 (현재 알려진 충돌 없음)
2. **등록 실패 처리** — `register()` 실패 시 토스트: *"단축키 등록 실패. 다른 앱과 충돌 가능. 설정에서 변경하세요."* + Settings → Quick Capture 단축키 입력으로 딥링크
3. **온보딩 안내** — 온보딩 2/3 단계에서 "지금 ⌃⇧H를 한 번 눌러보세요" 인터랙티브 검증

### 4-3. 데이터 마이그레이션 마법사 (M1)

#### 트리거 조건
첫 실행 시 컨테이너 안 `data.db` 부재 → 마이그레이션 다이얼로그 노출. 두 번째 실행부터는 skip flag (`migration.completed` 또는 `migration.skipped` in settings) 기준.

#### 흐름

```
[첫 실행 감지]
  ↓
[다이얼로그] "기존 Hearth (0.x) 데이터 가져오기"
  ├─ [데이터 가져오기]   ← 권장
  ├─ [새로 시작]
  └─ [나중에 결정]      ← 메인 진입, 설정에서 재실행 가능
        ↓ (가져오기 선택)
  [NSOpenPanel] 폴더 선택
    initialDirectory: ~/Library/Application Support/com.newturn2017.hearth
        ↓
  [security-scoped bookmark 획득]
        ↓
  [검증]
    - data.db 존재? → 없으면 "올바른 폴더가 아닙니다" 재시도
    - 스키마 버전 호환 (0.9.x → 1.0)
        ↓
  [복사 — 진행률 모달]
    임시 위치(.staging/)에 복사 → atomic rename
    1. data.db          → 컨테이너/data.db
    2. settings.json    → 컨테이너/settings.json (있으면)
    3. backups/ 폴더    → 사용자 확인 후 선택적 복사
        ↓
  [스키마 마이그레이션 실행 — 0.9.x → 1.0]
        ↓
  [완료] bookmark 즉시 폐기
  토스트: "데이터 N개 항목 가져오기 완료"
```

#### 보안 노트
- bookmark는 **1회 사용 후 즉시 release** (`stopAccessingSecurityScopedResource`). 영구 보관 시 reject 가능.
- 복사는 `.staging/` 임시 폴더 → atomic rename 패턴으로 부분 실패 시 컨테이너 일관성 유지.

#### 재진입
설정 → "데이터 가져오기" 메뉴로 언제든 다시 호출 가능 (백업에서 복구 시나리오).

#### 실패 처리
| 상황 | 동작 |
|---|---|
| 권한 거부 | "설정 → 데이터 가져오기에서 재시도" 안내 |
| 잘못된 폴더 선택 | "data.db를 찾을 수 없습니다" + 재선택 |
| 스키마 호환 실패 (0.8 이하) | "이 버전은 자동 마이그레이션 미지원. 0.9.x로 먼저 업데이트 후 다시 시도" |
| 복사 도중 실패 | atomic rename으로 컨테이너 DB 보호. 토스트: "마이그레이션 실패. 원본 데이터는 안전합니다" |
| DB 손상 | "기술 지원 문의" 링크 + 백업 폴더에서 직접 선택 옵션 |

### 4-4. Finder/Ghostty 호출 — `NSWorkspace` 전환

기존 `cmd_actions.rs:23-33` 의 `open` 명령어 호출을 **`NSWorkspace` API**로 전환:

| 동작 | 기존 | 신규 |
|---|---|---|
| Finder reveal | `open -R <path>` | `NSWorkspace.shared.activateFileViewerSelectingURLs([url])` |
| Ghostty 열기 | `open ghostty://...` | `NSWorkspace.shared.open(URL)` |

**효과:** `NSAppleEventsUsageDescription` 불필요. AppleEvents 권한 다이얼로그도 안 뜸. 사용자 경험 깔끔.

구현: Rust 에서 `objc2` crate로 `NSWorkspace` 호출하거나 작은 Swift shim. **objc2 직접 사용 권장** (의존성 1개 추가, Swift sidecar 불필요).

---

## 5. 빌드 · 서명 · 업로드 파이프라인

### 5-1. Apple Developer 포털 자산 (1회 셋업)

| 자산 | 종류 | 용도 |
|---|---|---|
| **App ID** `com.newturn2017.hearth` | — | Capabilities: **App Sandbox + In-App Purchase** 켜기 |
| **인증서 1: Apple Distribution** | macOS 코드서명 | `.app` 서명 |
| **인증서 2: Mac Installer Distribution** | 패키지 서명 | `.pkg` 서명 |
| **Provisioning Profile: Mac App Store** | App ID + 인증서 묶음 | `.app` 안에 `embedded.provisionprofile` 임베드 |

기존 `Developer ID Application` 인증서는 MAS 빌드에 사용 안 함.

### 5-2. `tauri.conf.json` 변경

```json
"macOS": {
  "signingIdentity": "Apple Distribution: jaehyun jang (2UANJX7ATM)",
  "providerShortName": "2UANJX7ATM",
  "entitlements": "entitlements.plist",
  "provisioningProfile": "embedded.provisionprofile",
  "minimumSystemVersion": "11.0"
}
```

### 5-3. 빌드 스크립트 — `scripts/build-mas.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

# 1. 클린
rm -rf src-tauri/target/release/bundle/macos
rm -rf dist-mas

# 2. 버전 동기화
node scripts/sync-version.js

# 3. Build number 증가
node scripts/bump-build-number.js

# 4. .app 빌드 (Tauri가 1차 서명)
npx tauri build --bundles app --target aarch64-apple-darwin

APP_PATH="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/Hearth.app"
PKG_PATH="dist-mas/Hearth-$(node -p "require('./package.json').version").pkg"

# 5. Provisioning profile 임베드
cp ./certs/Hearth_MAS.provisionprofile "$APP_PATH/Contents/embedded.provisionprofile"

# 6. 재서명 (provisioning profile 포함 상태로)
codesign --force --options runtime \
  --entitlements src-tauri/app/entitlements.plist \
  --sign "Apple Distribution: jaehyun jang (2UANJX7ATM)" \
  "$APP_PATH"

# 7. 서명 검증
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

# 8. .pkg 생성 + 서명
mkdir -p dist-mas
productbuild --component "$APP_PATH" /Applications \
  --sign "3rd Party Mac Developer Installer: jaehyun jang (2UANJX7ATM)" \
  "$PKG_PATH"

# 9. Apple 사전 검증
xcrun altool --validate-app -f "$PKG_PATH" -t macos \
  --apiKey "$APP_STORE_API_KEY_ID" \
  --apiIssuer "$APP_STORE_API_ISSUER_ID"

echo "✅ Build OK. Run: scripts/upload-mas.sh"
```

### 5-4. 업로드 스크립트 — `scripts/upload-mas.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

PKG=$(ls -t dist-mas/*.pkg | head -1)
echo "Uploading: $PKG"

xcrun altool --upload-app -f "$PKG" -t macos \
  --apiKey "$APP_STORE_API_KEY_ID" \
  --apiIssuer "$APP_STORE_API_ISSUER_ID"
```

빌드/업로드 분리 이유: 빌드 번호는 영구 소비되므로 업로드는 의식적인 두 번째 명령으로.

### 5-5. App Store Connect API Key (1회 셋업)

1. App Store Connect → Users and Access → Integrations → Keys → Generate (Role: App Manager)
2. `.p8` 파일을 `~/.private_keys/AuthKey_XXX.p8` 에 저장 (gitignore)
3. 환경변수 (`~/.zshrc`):
   ```
   export APP_STORE_API_KEY_ID="XXXXXX"
   export APP_STORE_API_ISSUER_ID="xxxx-xxxx-xxxx"
   export API_PRIVATE_KEYS_DIR="$HOME/.private_keys"
   ```

### 5-6. 사전 검증 — `scripts/check-signing.sh`

```bash
#!/usr/bin/env bash
# 인증서 5개 존재 확인
security find-identity -v -p codesigning | grep -q "Apple Distribution: jaehyun jang"
security find-identity -v -p codesigning | grep -q "3rd Party Mac Developer Installer"
test -f ./certs/Hearth_MAS.provisionprofile
test -n "$APP_STORE_API_KEY_ID"
test -n "$APP_STORE_API_ISSUER_ID"
echo "✅ All signing assets present"
```

빌드 스크립트 첫 줄에서 호출.

---

## 6. 테스트 · TestFlight · 일정 · 리스크

### 6-1. 샌드박스 동작 검증 시나리오

빌드 후 `.app` 을 `/Applications`에 직접 복사 → 실행 → 다음 매트릭스 통과:

| # | 시나리오 | 기대 동작 |
|---|---|---|
| T1 | 첫 실행, OSS 데이터 없음 | 마이그레이션 다이얼로그 → "새로 시작" → 빈 상태 진입 |
| T2 | 첫 실행, OSS 데이터 있음 | 다이얼로그 → "데이터 가져오기" → 폴더 선택 → 복사 완료 → 기존 항목 보임 |
| T3 | 두 번째 실행 | 다이얼로그 안 뜸, 바로 진입 |
| T4 | Quick Capture ⌃⇧H | 다른 앱 활성 상태에서 오버레이 표시 |
| T5 | 백업 폴더 사용자 지정 | 임의 폴더 선택 → 백업 파일 생성 확인 |
| T6 | OpenAI 키 입력 후 ⌘K | 정상 응답 |
| T7 | 일정 알림 토글 ON 후 시각 도달 | 알림 표시 (첫 회 시 권한 다이얼로그) |
| T8 | Finder 우클릭 → 폴더 열기 | Finder에 해당 파일 하이라이트 |
| T9 | 설정 → "App Store에서 보기" | 빈 ID라도 macappstore:// 스킴 동작 확인 |
| T10 | 설정 → "데이터 가져오기" 재실행 | 마법사 다시 호출 가능 |

### 6-2. TestFlight 베타 흐름

- **대상:** 본인 + 신뢰할 수 있는 1~3명 (가족·친구)
- **시점:** Day 14 빌드 업로드 후, App Review 제출 **전** Day 15
- **목적:** 실제 App Store 설치 경로로 데이터 마이그레이션·IAP 결제 흐름 검증
- **기간:** 1~2일

App Store Review와 별개로 즉시 진행 가능. TestFlight 자체는 별도 심사 (~24시간) 필요하나 가벼움.

### 6-3. Day-by-day 일정 (서브프로젝트 A 한정)

| Day | 작업 | 산출물 |
|---|---|---|
| **D1 (4/27)** | Apple Developer 포털 셋업: App ID Capabilities (App Sandbox + IAP) 켜기, 인증서 2개 발급, Provisioning Profile 생성·다운로드. App Store Connect API Key 생성. `scripts/check-signing.sh` 통과 | 인증서·프로파일 로컬 저장 |
| **D2** | Updater 제거 (§3). UI 교체 (Settings 카드, TopBar 배지 제거). grep으로 잔재 0건 확인 | 4개 파일 수정 + 1개 삭제 |
| **D3** | Autostart 제거 (§4-1). UI 교체. `cmd_autostart.rs` 삭제. 빌드 워닝 0 | 3개 파일 수정 + 1개 삭제 |
| **D4** | `entitlements.plist` 갱신 (§2). `tauri.conf.json` macOS 블록 갱신 (§5-2). `Info.plist` 항목 추가. Finder/Ghostty `NSWorkspace` 전환 (§4-4) | 설정 파일 + objc2 통합 |
| **D5** | 마이그레이션 마법사 구현 (§4-3). `cmd_migration.rs` + `MigrationWizard.tsx` + 설정 → 재진입 메뉴 | 신규 2 파일 + 설정 UI |
| **D6** | 빌드 스크립트 4개 작성 (§5-3, 5-4, 5-6, sync-version, bump-build-number). 첫 MAS 빌드 성공 | `scripts/*` + `dist-mas/Hearth-1.0.0.pkg` |
| **D7** | 샌드박스 동작 매트릭스 T1–T10 모두 통과. App ID와 빌드 메타데이터 일치 확인. App Store Connect 사전 검증 통과 | "MAS 빌드 가능" 상태 달성 |

### 6-4. 심사 reject 시나리오 카탈로그

| # | 사유 | 발생 가능성 | 대응 |
|---|---|---|---|
| R1 | Privacy Policy URL 누락 | 高 | 랜딩 페이지(E-lite)에 `/privacy` 페이지 필수 |
| R2 | App Sandbox 비활성 | 低 | entitlements.plist 검증 자동화 |
| R3 | IAP 메타데이터 불일치 | 中 | B 서브프로젝트에서 별도 다룸 |
| R4 | 자동실행/전역단축키 정당화 부족 | 中 | App Review Notes 필드에 사용 시나리오 명시 |
| R5 | 데모 계정/사용 가이드 누락 | 中 | Review Notes에 "OpenAI 키 없이도 모든 핵심 기능 동작" 명시 + 데모 데이터 스크린샷 |
| R6 | 컨테이너 외부 쓰기 시도 (마이그레이션 bookmark 누수) | 中 | 4-3의 즉시 release 패턴 엄수 + 코드 리뷰 |
| R7 | Hardcoded test account | 低 | gitignore된 .env 사용 |
| R8 | 스크린샷 품질 | 中 | D 서브프로젝트에서 다룸 |

**원칙:** Day 14 제출 → 첫 reject가 와도 Day 17까지 1회 수정 + 재제출 버퍼 확보.

### 6-5. Definition of Done (서브프로젝트 A)

- [ ] `scripts/check-signing.sh` 통과
- [ ] `scripts/build-mas.sh` 성공 → `dist-mas/Hearth-1.0.0.pkg` 생성
- [ ] `xcrun altool --validate-app` 통과
- [ ] T1–T10 샌드박스 동작 매트릭스 모두 통과
- [ ] `rg -i 'updater|autostart'` 결과 0건 (안내 카드 카피 제외)
- [ ] `cargo build --release` 워닝 0건
- [ ] 마이그레이션 마법사: 0.9.5 데이터 → 1.0 컨테이너 복사·열람 검증

A 완료 시점에 B 서브프로젝트 (StoreKit 2 IAP + 라이선스 게이트) 설계 시작.

---

## 7. Out of Scope (1.1 이후)

| 항목 | 이유 | 예상 시기 |
|---|---|---|
| 로그인 시 자동 실행 (SMAppService) | Tauri 플러그인 미지원, Swift 사이드카 비용 | 1.1 |
| Universal binary (Intel) | 시장 비중 작음, 빌드 시간 증가 | 1.1 또는 1.2 |
| GitHub Actions MAS CI | 인증서·프로파일 시크릿 관리 추가 작업 | 1.1 |
| Direct DMG 채널 부활 | 라이선스 시스템 복잡도 증가 | 시장 신호 보고 검토 |
| Notarization 자동화 | MAS는 불필요 | Direct 채널 부활 시 |

---

## 8. 참고

- 후속 서브프로젝트: B (IAP) → C-lite (레포 분리) → D (App Store Connect 제출) → E-lite (랜딩)
- 출시 마스터 일정: Day 1–7 (A) → Day 8–12 (B) → Day 13 (C-lite) → Day 14–15 (D) → Day 16 (E-lite) → Day 17–20 (심사) → **Day 21 (5/17) 출시**
