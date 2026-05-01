# Hearth — MAS 출시 & 핫픽스 런북

App Store Connect(ASC) / Mac App Store(MAS) 사이클 전용 절차서. **항상 이 문서를 보고 진행한다.**

GitHub Release(DMG + 자동 업데이트) 흐름은 별도 문서: [`docs/releasing.md`](./releasing.md). 두 흐름은 서로 다른 빌드·서명·전송 경로를 가진다.

---

## 1. 현재 상태 스냅샷

**최종 갱신: 2026-05-01**

| 항목 | 값 |
| --- | --- |
| 마케팅 버전 | `1.0.0` |
| 마지막 업로드 빌드 | `7` |
| ASC 상태 | **심사 중** (Waiting for Review / In Review) |
| 출시 모드 | 수동 출시 (ASC에서 `출시` 버튼 눌러야 라이브) |
| 가격 | ₩22,000 (한국만 판매) |
| Bundle ID | `com.codewithgenie.hearth` |
| Team | `2UANJX7ATM` (jaehyun jang) |
| 라이브 URL (예정) | https://apps.apple.com/kr/app/hearth/id6764480247 |
| 제출 PR | #36 (squash → `eac3940`) |

> 이 표는 출시·업로드·핫픽스가 일어날 때마다 갱신한다. 마지막 항목은 항상 진실.

---

## 2. 사전 준비 (1회만)

설치/등록은 한 번만, 이후엔 확인만.

### 환경 변수 (영구)
`~/.zshenv`에 등록되어 있어야 함:
```sh
export APP_STORE_API_KEY_ID="A3C3ZX22AJ"
export APP_STORE_API_ISSUER_ID="..."   # ASC API issuer UUID
export API_PRIVATE_KEYS_DIR="$HOME/.private_keys"
```

### 비밀 자료
| 파일 | 위치 |
| --- | --- |
| ASC API 키 (.p8) | `~/.private_keys/AuthKey_A3C3ZX22AJ.p8` |
| MAS 프로비저닝 프로파일 | `certs/Hearth_MAS.provisionprofile` (`HEARTH_MAS_PROFILE` 환경변수로 override 가능) |
| 코드 서명 인증서 (Keychain) | `Apple Distribution: jaehyun jang (2UANJX7ATM)`<br>`3rd Party Mac Developer Installer: jaehyun jang (2UANJX7ATM)` |

### 사전 점검
```sh
bash scripts/check-signing.sh
```
빠진 게 있으면 fail-fast.

---

## 3. 정상 릴리스 사이클 (다음 마이너/메이저)

심사 통과 → 출시 → 안정화 후 다음 버전을 낼 때.

```
[ main 동기화 ]
    └─→ git checkout main && git pull --ff-only

[ 버전 bump (마이너 이상) ]
    └─→ ./scripts/bump-version.sh 1.1.0
    └─→ CHANGELOG.md에 새 섹션 추가 → 커밋

[ 빌드 ]
    └─→ bash scripts/build-mas.sh
        ├─ check-signing
        ├─ sync-version (package.json → tauri.conf + Cargo)
        ├─ bump-build-number (build-number.json +1)
        ├─ tauri build (1차 서명)
        ├─ provisioning profile 임베드 + 재서명
        ├─ productbuild .pkg
        └─ altool --validate-app
    => dist-mas/Hearth-<ver>-<build>.pkg

[ 업로드 ]
    └─→ bash scripts/upload-mas.sh
        => ASC 처리 10–30분

[ ASC 콘솔 작업 ]
    1. 앱 → 새 버전 (1.1.0) 만들기
    2. 빌드 선택 (방금 올린 빌드)
    3. What's New, 스크린샷(변경 시), 심사 정보 입력
    4. "심사를 위해 제출" 클릭
    5. 통과 시 "출시" 버튼으로 라이브
```

---

## 4. 핫픽스 사이클

### 4-A. 심사 중 핫픽스 (현재 1.0.0 build 7 같은 상태)

심사 결과 나오기 전 버그 발견 → 새 빌드를 올려 같은 버전에서 갈아끼움.

```
[ 핫픽스 브랜치 ]
    └─→ git checkout main && git pull
    └─→ git checkout -b fix/<짧은-이슈명>
    └─→ 코드 수정 + 커밋

[ ASC 콘솔: 기존 제출 거부 ]
    1. 앱 → 1.0.0 → "심사 거부" (Reject this binary)
       ※ "이 빌드 거부"이므로 버전(1.0.0) 자체는 유지됨
    2. 상태가 "개발자 거부됨"으로 바뀜

[ 빌드 + 업로드 ]
    └─→ bash scripts/build-mas.sh   # build 8 자동 부여
    └─→ bash scripts/upload-mas.sh

[ ASC 콘솔: 새 빌드 재제출 ]
    1. 1.0.0 페이지에서 빌드를 8로 교체
    2. "심사를 위해 제출" 다시

[ PR 머지 ]
    └─→ 핫픽스 PR을 main에 머지
```

> **핵심**: 마케팅 버전(`1.0.0`)은 그대로, **CFBundleVersion(빌드 번호)**만 자동 증가. ASC는 같은 버전 내 빌드 번호 중복을 거절하므로 `bump-build-number.js`가 매번 +1.

### 4-B. 출시 후 핫픽스 (라이브 1.0.0 → 1.0.1)

이미 사용자가 다운로드 가능한 상태에서 버그 발견.

```
[ 핫픽스 브랜치 + 패치 버전 bump ]
    └─→ ./scripts/bump-version.sh 1.0.1
    └─→ CHANGELOG.md "## [1.0.1]" 섹션 추가
    └─→ 코드 수정 + 커밋

[ ASC 콘솔: 새 버전 만들기 ]
    1. 앱 → "새 버전 추가" → 1.0.1
    2. What's New 작성 (변경점 1–3줄)

[ 빌드 + 업로드 ]
    └─→ bash scripts/build-mas.sh
    └─→ bash scripts/upload-mas.sh

[ 제출 + 출시 ]
    1. 빌드 선택 → 심사 제출
    2. 통과 → 자동/수동 출시
```

### 4-C. 심사 거절(Reject) 응답

ASC Resolution Center에 답변 작성 → "다시 심사 요청" 또는 새 빌드 필요시 4-A 흐름.

자주 나오는 거절:
- **5.1.1 (권한 사유 부족)**: `NSUserNotificationsUsageDescription` 등 Info.plist `*UsageDescription` 키가 사용자 친화적이고 구체적인지 확인. 답변엔 "tauri-plugin-notification 표준 패턴, 사용자가 OS 다이얼로그에서 명시적으로 허용/거부 선택" 같은 맥락 제공.
- **2.1 (앱 크래시/기능 미작동)**: 재현 시나리오 + 픽스 제출 빌드 번호 명시.
- **3.1.1 (IAP 미사용 앱에서 결제 언급)**: 앱이 IAP가 없는 1.0 단계라면 결제 관련 문구를 description/스크린샷에서 제거.

---

## 5. TestFlight 내부 테스트

업로드한 모든 빌드는 ASC 처리 후 자동으로 TestFlight 빌드로 등록됨 (수출 규정 준수 답변 후).

```
[ ASC 콘솔: TestFlight 탭 ]
    1. 빌드 옆 "수출 규정 준수" 답변 (HTTPS-only → 면제)
    2. 내부 테스터 그룹에 빌드 할당
    3. 테스터에게 메일 발송됨

[ 로컬 설치 ]
    1. /Applications/Hearth.app 제거
       ⚠️  ~/Library/Containers/com.codewithgenie.hearth/ 는 보존 → DB 유지
    2. TestFlight.app에서 설치
```

---

## 6. 빌드 산출물 & 임시 파일

| 경로 | 용도 | 커밋? |
| --- | --- | --- |
| `dist-mas/Hearth-<ver>-<build>.pkg` | 업로드 대상 | ❌ (gitignore) |
| `build-number.json` | CFBundleVersion 카운터 | ⚠️ 빌드마다 변경되지만 `main`에 주기적 동기화 필요 |
| `src-tauri/target/` | Rust 빌드 캐시 | ❌ |
| `~/Library/Containers/com.codewithgenie.hearth/` | MAS 샌드박스 데이터 | (로컬, 백업만) |

---

## 7. 데모 환경

심사용 데모 데이터는 `~/Desktop/hearth-demo/` 폴더에 있음. MAS 빌드 위저드가 이 폴더를 가리키도록 설정. 본인 작업으로 복귀하려면:
1. 앱 내 마이그레이션 위저드 reset
2. 평소 사용하는 데이터 폴더 재선택
3. (필요시) `~/Desktop/hearth-demo/` 삭제

---

## 8. 알려진 follow-ups (다음 핫픽스 후보)

| ID | 증상 | 우선순위 |
| --- | --- | --- |
| FB-001 | 프로젝트 카드 "Finder에서 열기" → 샌드박스 권한 차단 다이얼로그. NSOpenPanel + security-scoped bookmark 흐름이 프로젝트 폴더 경로에는 미적용 | **P0 — 1.0.0 핫픽스 (4-A)** |

신규 follow-up이 발견되면 이 표에 추가 → 다음 사이클에 반영.

---

## 9. 빠른 참조 (Cheat sheet)

```sh
# 사전 점검
bash scripts/check-signing.sh

# 풀 빌드
bash scripts/build-mas.sh && bash scripts/upload-mas.sh

# 빌드만 다시 (이미 코드는 OK, 업로드 실패 재시도용은 아님 — build-number가 또 증가함)
bash scripts/build-mas.sh

# 버전 bump
./scripts/bump-version.sh 1.1.0

# 현재 버전/빌드 확인
node -p "require('./package.json').version"
node -p "require('./build-number.json').build"
```

---

## 10. 변경 이력

이 문서를 수정할 때마다 한 줄 추가.

- 2026-05-01: 초판. 1.0.0 build 7 심사 중 상태 반영, 4-A 핫픽스 흐름 정리, FB-001 follow-up 등록.
