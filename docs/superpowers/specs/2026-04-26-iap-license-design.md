# Hearth 1.0 — StoreKit 2 IAP + License Gate 설계

- **작성일**: 2026-04-26 (Sprint Day 0)
- **서브프로젝트**: B (출시 sprint 8개 중 두 번째)
- **선행**: A (MAS 적합성, commit 6b023ab)
- **일정**: Sprint Day 8-12 (5일)
- **출시 목표**: 2026-05-17 (1.0 MAS 단일 채널)

## 1. Scope

Hearth 1.0의 단일 비소모성 IAP(`io.hearth.app.pro`)와 클라이언트 라이선스 게이트를 설계한다. 14일 시간 제한 트라이얼, 만료 후 read-only 진입, StoreKit 2 로컬 검증만으로 위조 방어, Family Sharing ON 정책을 포함한다.

본 스펙은 자체 백엔드 서버를 도입하지 않는다. 결제 검증·trial 추적은 모두 디바이스 내(StoreKit 2 SDK + macOS Keychain)에서 완결된다.

## 2. 확정된 5대 결정 (사용자 승인)

| # | 결정 | 선택 | 근거 요지 |
|---|------|------|----------|
| Q1 | 무료 평가 모드 | **14일 시간 제한 트라이얼** | Mac 유틸 표준 패턴, 구현 비용 최저, 가치 통합 안 깨짐 |
| Q2 | 가격대 | **$14.99 (1회 구매)** | 무명 1.0 진입 안전, 충동구매 영역, 1.x에서 인상 여지 |
| Q3 | Family Sharing | **ON** | App Store 노출·리뷰 방어, 비가역(ON→OFF 불가)이라 안전선택 |
| Q4 | 영수증 검증 | **로컬 only (StoreKit 2 JWS)** | 서버 인프라 0, 프라이버시 깔끔, 시장규모 < 해커 ROI |
| Q5 | 라이선스 무력화 시 UX | **Read-only (데이터 노출 + mutation 차단)** | 데이터 인질 안 잡기, 전환율↑, MAS 가이드라인 친화 |

추가 정책: Settings의 외부 통합(캘린더 OAuth, AI API 키)은 read-only일 때 **읽기 전용 + disconnect만 허용** (GDPR 친화).

## 3. 아키텍처

### 3.1 컴포넌트 (3 + 1)

```
┌─ Swift StoreKit 브리지  (src-tauri/macos/HearthStoreKit/)
│   ├─ HearthStoreKit.swift  (Swift Package, @_cdecl FFI 5개)
│   ├─ Apple StoreKit 2 SDK 호출 (Product, Transaction)
│   ├─ JWS 서명은 Apple SDK가 내부 검증 → verified case만 Rust로 전달
│   └─ Transaction.updates 백그라운드 리스너 (refund/revoke 즉시 반영)
│
├─ Rust LicenseGate  (src-tauri/core/src/license/)
│   ├─ storekit.rs        — Swift FFI 래퍼, async/await
│   ├─ keychain.rs        — security-framework crate, 서비스명 "io.hearth.app.license"
│   ├─ status.rs          — 상태 머신 (determine_status)
│   └─ commands.rs        — Tauri command: get_license_status / request_purchase / restore_purchase
│
├─ React UI Gate  (src/license/)
│   ├─ LicenseProvider.tsx
│   ├─ useLicense.ts          — { status, daysLeft, isReadOnly, requestPurchase, restorePurchase }
│   ├─ useMutationGate.ts     — gate(fn, contextLabel) wrapper
│   ├─ PaywallModal.tsx
│   ├─ TrialBanner.tsx        — 상단 카운트다운
│   └─ ReadOnlyBanner.tsx     — 만료 후 영구 배너
│
└─ App Store Connect (외부)
    ├─ Product ID: io.hearth.app.pro (Non-Consumable)
    ├─ Price tier: USD $14.99 (자동 KRW 환산)
    └─ Family Sharing: ON
```

### 3.2 상태 머신

```
FirstLaunch ──> Trial(14d) ──[14d 경과 OR 시간 역행]──> TrialExpired
                   │
                   └──[StoreKit purchase success]──> Purchased
                                                       │
TrialExpired ──[purchase]──> Purchased                 │
Purchased ──[Apple revoke / refund]──> TrialExpired ◄──┘
```

상태 enum: `Trial { days_left: u32 } | TrialExpired | Purchased | Pending`.

### 3.3 핵심 원칙

- **단일 진실원**: 결제 상태는 항상 `Transaction.currentEntitlements`에서 조회. keychain의 `purchased` 캐시는 오프라인 부트스트랩용일 뿐, 부팅 후 1회 StoreKit 동기화로 정정.
- **시간 역행 방어**: 매 실행 시 `now < last_seen_at - 5min` 이면 `tamper_flag=true` 영구 set → 즉시 TrialExpired.
- **데이터-라이선스 분리**: TrialExpired 상태에서도 모든 read 경로(list/get)는 절대 차단하지 않는다. 차단은 mutation 경로(create/update/delete + 외부 호출)에서만.
- **depth-in-defense**: UI 게이트 + Rust command 게이트 둘 다 적용. UI만으로는 키보드/devtools 우회 가능.

## 4. StoreKit 2 브리지 (가장 까다로운 부분)

### 4.1 결정: 자체 Swift helper 임베드

`tauri-plugin-storekit` 같은 OSS는 사실상 부재 또는 미성숙. 자체 Swift Package를 만들어 Rust에서 FFI로 호출.

```
src-tauri/macos/HearthStoreKit/
  ├─ Package.swift
  ├─ Sources/HearthStoreKit/HearthStoreKit.swift
  │   ├─ @_cdecl("hearth_storekit_init")
  │   ├─ @_cdecl("hearth_storekit_fetch_product")           // products.first(where: id)
  │   ├─ @_cdecl("hearth_storekit_purchase")                // Product.purchase()
  │   ├─ @_cdecl("hearth_storekit_current_entitlement")     // Transaction.currentEntitlements
  │   ├─ @_cdecl("hearth_storekit_restore")                 // AppStore.sync()
  │   └─ Transaction.updates 백그라운드 listener (Task)
  └─ build.rs (in src-tauri/) — xcrun swiftc로 .a static lib 빌드 → Cargo가 link
```

### 4.2 Rust FFI

```rust
// src-tauri/core/src/license/storekit.rs
unsafe extern "C" fn hearth_storekit_purchase(
    product_id: *const c_char,
    callback: extern "C" fn(*const c_char, *mut c_void),
    user_data: *mut c_void,
);
```

Swift에서 callback 호출 시 JSON 페이로드:
```json
{
  "transaction_id": "2000000123456789",
  "product_id": "io.hearth.app.pro",
  "purchase_date": "2026-05-20T10:30:00Z",
  "is_family_share": false,
  "ownership_type": "PURCHASED" | "FAMILY_SHARED"
}
```

콜백을 `tokio::sync::oneshot`으로 래핑해 async/await 인터페이스 제공.

### 4.3 검증 정책

- Swift 측 `Transaction.currentEntitlements`는 verified/unverified case로 분기. **verified case만 Rust로 전달**, unverified는 무시 + 로깅.
- Rust는 JWS 재검증을 시도하지 않는다 (Apple 인증서 처리 없음). Swift SDK 신뢰가 곧 Apple SDK 신뢰.

### 4.4 Sandbox 테스트

- App Store Connect → Users and Access → Sandbox Testers 계정 생성
- macOS 시스템 설정 → App Store → Sandbox Account에 로그인
- 개발 빌드는 `Configuration.storekit` 파일 동봉(Xcode StoreKit Configuration File)으로 로컬 결제 시뮬 가능

### 4.5 리스크

| 위험 | 완화 |
|------|------|
| Tauri 빌드 파이프라인에 Swift 컴파일 추가 | `build.rs`에서 `xcrun swiftc` 호출, GitHub Actions `macos-latest`에서 검증 |
| 첫 실행 시 StoreKit 응답 지연(2-3초) | UI는 keychain 캐시 기반 낙관적 표시(Trial(N) or Purchased), 응답 후 정정 |
| Family Sharing entitlement 처리 누락 | T10 (가족 계정 로그인) 필수 통과 |

## 5. 트라이얼 타이밍 & 변조 방지

### 5.1 Keychain 저장

```
service: "io.hearth.app.license"
account: "trial"
items:
  ├─ trial_started_at    : ISO8601 (첫 실행 시 1회 기록, 절대 덮어쓰기 금지)
  ├─ last_seen_at        : ISO8601 (매 실행 시 업데이트)
  ├─ tamper_flag         : "true" | "false"
  └─ last_known_status   : "trial" | "expired" | "purchased" (콜드스타트 UI 힌트용 캐시; 부팅 후 StoreKit 응답으로 정정)
```

**왜 keychain (UserDefaults·sqlite·파일 아님):**
- 앱 삭제·재설치에 살아남음 → 가장 흔한 트라이얼 리셋 우회 차단
- macOS Sandbox에서 자기 앱 keychain access group만 접근 → 타 앱 변조 불가
- 사용자가 의도적으로 Keychain Access.app으로 지우면 리셋되지만, 그 정도 의지는 결제 거부와 동일 → 정책상 무시

### 5.2 상태 결정 로직

```rust
fn determine_status(now: DateTime<Utc>, kc: &KeychainStore, sk: &StoreKitState) -> LicenseStatus {
    // 1. 결제 우선
    if sk.has_active_entitlement() {
        return LicenseStatus::Purchased;
    }

    // 2. 트라이얼 시작 시각 로드 or 최초 생성
    let started = match kc.read_trial_started_at() {
        Some(t) => t,
        None => { kc.write_trial_started_at(now); now }
    };

    // 3. 시간 역행 감지 (5분 tolerance)
    if let Some(last) = kc.read_last_seen_at() {
        if now < last - Duration::minutes(5) {
            kc.set_tamper_flag(true);
        }
    }
    kc.write_last_seen_at(now);

    // 4. tamper_flag 영구 만료
    if kc.read_tamper_flag() {
        return LicenseStatus::TrialExpired;
    }

    // 5. 14일 경과 체크
    let days_used = (now - started).num_days();
    if days_used >= 14 {
        LicenseStatus::TrialExpired
    } else {
        LicenseStatus::Trial { days_left: (14 - days_used) as u32 }
    }
}
```

**5분 tolerance 근거:** NTP 동기화·서머타임·타임존 변경은 정상 사용자에게도 발생. 5분 미만 역행은 정상으로 간주.

### 5.3 UI 카운트다운

| 잔여 일수 | 표시 |
|----------|------|
| D-14 ~ D-7 | 우상단 작은 텍스트 "Trial: Nd left" |
| D-6 ~ D-1 | 강조 색상 + 결제 버튼 노출 |
| D-Day (마지막날) | 모달 1회: "오늘 종료, 내일부터 read-only" |
| TrialExpired | 영구 배너 + 모든 mutation UI 비활성 |

D-6부터 강조하는 이유: 14일 중 8일 경과 시점이 사용자가 가치 인지 또는 이탈 결정 분기점이라 가정.

### 5.4 Restore Purchase

Settings → 라이선스 섹션에 "구매 복원" 버튼 명시 노출. 누르면 `AppStore.sync()` 호출 후 `Transaction.currentEntitlements` 강제 재조회. 다른 Mac에서 산 사람·재설치 후 즉시 Purchased로 복원. App Store 심사상 필수 (R3).

## 6. UI 통합 (read-only 모드)

### 6.1 단일 게이트 훅

```tsx
// src/license/useMutationGate.ts
export function useMutationGate() {
  const { isReadOnly, requestPurchase } = useLicense();
  return {
    gate<T extends (...args: any[]) => any>(fn: T, contextLabel: string): T {
      return ((...args) => {
        if (isReadOnly) {
          openPaywallModal({ context: contextLabel, onPurchase: requestPurchase });
          return undefined;
        }
        return fn(...args);
      }) as T;
    }
  };
}
```

사용:
```tsx
const { gate } = useMutationGate();
<button onClick={gate(createProject, "create_project")}>+ 프로젝트</button>
```

`contextLabel`은 어떤 액션에서 paywall이 열렸는지 카운트 → 미래 가격·트라이얼 조정용 텔레메트리 (자체 서버 없으므로 로컬 집계만).

### 6.2 Mutation 진입점 인벤토리 (게이트 적용 대상)

| 영역 | 진입점 |
|------|-------|
| 프로젝트 | 생성, 수정, 삭제, 아카이브 |
| 메모 | 생성, 수정, 삭제, 태그 변경 |
| 일정 | 생성, 수정, 삭제, 외부 캘린더 sync trigger |
| AI | 호출 (비용 발생) |
| QuickCapture | 새 항목 저장 |
| Settings | 외부 캘린더 추가, AI 키 입력 → **read-only 표시** |
| Settings - 디스커넥트 | **항상 허용** (Q4-c) |

대략 **UI mutation 진입점 ~20개, Rust mutation command ~30개**. 정확 카운트는 D10 작업 시 grep 인벤토리.

### 6.3 Rust 측 방어선

모든 mutation Tauri command 입구에 1줄:
```rust
#[tauri::command]
async fn create_project(state: State<'_, AppState>, ...) -> Result<Project, AppError> {
    state.license.require_active()?;  // ← Trial / Purchased만 통과, 그 외 LicenseError::ReadOnly
    // 기존 로직...
}
```

`require_active()`는 thin helper. UI만 막으면 키보드 단축키·Tauri devtools 우회 가능하므로 Rust에서도 거부.

Read commands(`list_projects`, `get_memo`, `search_*`)는 절대 게이트하지 않는다.

### 6.4 영향 파일 인벤토리

- **신규** (~12 파일):
  - `src/license/` 6개 (Provider, useLicense, useMutationGate, PaywallModal, TrialBanner, ReadOnlyBanner)
  - `src-tauri/core/src/license/` 4개 (mod, storekit, keychain, status, commands)
  - `src-tauri/macos/HearthStoreKit/` 2개 (Package.swift, HearthStoreKit.swift)
- **수정**:
  - 모든 mutation Tauri command (~30개) — `require_active()?` 1줄 추가
  - 모든 mutation UI 진입점 (~20개) — `gate(fn, label)` 래핑
  - `App.tsx` — TrialBanner / ReadOnlyBanner mount
  - `src-tauri/build.rs` — Swift 컴파일 단계 추가
  - `tauri.conf.json` — macOS sandbox entitlements (이미 A 스펙에서 IAP capability 포함)
  - `Settings.tsx` — Restore Purchase 버튼 + 외부 통합 read-only 처리

## 7. 테스트 매트릭스 (T1-T12)

| ID | 시나리오 | 기대 결과 |
|----|---------|----------|
| T1 | 첫 실행 (keychain 비어있음) | trial_started_at 기록, Trial(14d) 표시 |
| T2 | 1일 경과 후 재실행 | Trial(13d) 표시, last_seen_at 업데이트 |
| T3 | 14일 경과 후 재실행 | TrialExpired, read-only 모드, 영구 배너 노출 |
| T4 | Trial 중 시스템 시간 1년 뒤로 | 만료 처리 (시간 전진은 정상 진행) |
| T5 | Trial 중 시스템 시간 1일 뒤로 (>5분 역행) | tamper_flag set → TrialExpired (영구) |
| T6 | 앱 삭제 후 재설치 | trial_started_at 살아있음 → 남은 일수 그대로 |
| T7 | Sandbox 계정으로 Purchase | Purchased 상태, 모든 mutation 정상 |
| T8 | Purchase 후 Apple test refund | 다음 실행 시 entitlement 사라짐 → TrialExpired |
| T9 | 다른 Mac에서 Restore Purchase | Purchased 즉시 복원 |
| T10 | Family Sharing 가족 계정 로그인 | Purchased (ownership_type=FAMILY_SHARED) |
| T11 | TrialExpired 상태에서 메모 편집 시도 | UI 버튼 비활성, 키보드 단축키 차단, Rust command가 LicenseError::ReadOnly 반환 |
| T12 | TrialExpired 상태에서 캘린더 disconnect | 허용 (Q4-c 정책) |

자동화 가능: T1-T6, T11, T12 (타임 mocking + Tauri command 단위 테스트).
수동 필수: T7-T10 (Sandbox 계정 + 실 디바이스).

## 8. 심사 reject 카탈로그 (R1-R5)

| ID | 위험 | 대응 |
|----|------|------|
| R1 | "free trial" 문구 사용으로 비소모성 IAP 가이드라인(3.1.2(b)) 위반 의심 | UI 카피는 "14일 무료 체험" 대신 "결제 전 14일 사용 가능" 같은 우회 표현. 가이드라인 인용 답변 준비 |
| R2 | 데이터 export 경로 부재 → "데이터 인질" reject | read-only 모드에서도 메모 복사·드래그·JSON export 항상 활성. 우클릭 메뉴 살아있음 |
| R3 | Restore Purchase 버튼 없음 → 자동 reject | Settings → 라이선스 섹션에 명시 노출 |
| R4 | Family Sharing ON인데 가족 entitlement 처리 안 함 | T10 통과 필수, ownership_type 로깅 |
| R5 | Paywall 모달이 데이터 자체를 가림 → 데이터 접근권 침해 | read-only는 데이터 표시 그대로, mutation만 차단. PaywallModal은 사용자 액션 트리거에만 표시 (자동 표시 X) |

## 9. Day-by-day 일정 (Sprint Day 8-12)

### D8 — Swift StoreKit helper + Rust FFI 스켈레톤
- `src-tauri/macos/HearthStoreKit/` Swift Package 생성, 5개 `@_cdecl` 함수 stub
- `src-tauri/build.rs`에 `xcrun swiftc` 호출 단계 추가
- Rust 측 FFI 래퍼 + tokio oneshot 콜백 어댑터
- `Configuration.storekit` 파일로 로컬 결제 흐름 dry-run 통과
- **Done**: dev 빌드에서 "구매" 버튼 누르면 sandbox 결제 모달이 뜨고 결과가 Rust 로그에 찍힘

### D9 — LicenseGate 상태 머신 + keychain
- `keychain.rs` (security-framework crate, `io.hearth.app.license` 서비스)
- `status.rs` 상태 결정 로직 + 시간 역행 방어
- 단위 테스트로 T1-T6 자동화 (`mock_clock` 패턴)
- **Done**: Rust 단위 테스트로 T1-T6 PASS

### D10 — React 게이트 훅 + Rust command 게이트 일괄 적용
- `src/license/` 6개 파일 생성
- 모든 mutation Tauri command (~30개)에 `require_active()?` 1줄 추가
  - grep으로 카운트 확인, ast-grep으로 일괄 패치 가능
- 모든 mutation UI 진입점 (~20개)에 `gate()` 래핑
- **Done**: `rg "tauri::command" src-tauri/`와 게이트 적용 카운트 일치, T11 PASS

### D11 — Paywall/Banner UI + Family Sharing + Restore
- `PaywallModal`, `TrialBanner`, `ReadOnlyBanner` 디자인·구현
- `Settings.tsx`에 Restore Purchase 버튼 + 외부 통합 read-only/disconnect 처리
- Family Sharing 분기 처리 (`ownership_type`)
- **Done**: T7, T9, T10, T12 PASS (수동 sandbox)

### D12 — Sandbox 계정 E2E + 심사 reject 자가 점검
- T8 (refund) 수동 검증
- R1-R5 카탈로그 한 줄씩 자가 체크
- 카피 라이팅 최종 검토 ("free trial" 문구 grep)
- **Done**: T1-T12 전부 PASS, R1-R5 자가 점검 통과

## 10. DoD 체크리스트

- [ ] T1-T12 전부 PASS
- [ ] App Store Connect Product ID `io.hearth.app.pro` 등록 (Non-Consumable, Family Sharing ON, $14.99 tier)
- [ ] 모든 mutation Tauri command에 `require_active()?` 적용 (≥30개, grep 카운트 일치)
- [ ] 모든 mutation UI 진입점에 `gate()` 적용 (≥20개)
- [ ] read-only 모드에서 export·복사·disconnect 정상 동작 (R2 / Q4-c 보장)
- [ ] Settings에 Restore Purchase 버튼 노출 (R3)
- [ ] 트라이얼 카운트다운 D-6부터 강조 표시
- [ ] privacy policy에 "결제는 Apple이 처리, 자체 서버 미사용" 명시
- [ ] UI 카피에 "free trial" 문구 부재 (R1)
- [ ] Sandbox 계정으로 Purchase → Refund → TrialExpired 흐름 1회 실증

## 11. Out of Scope (1.1+)

- 구독(Subscription) 모델 — 1.0은 비소모성 1회 구매 단일
- 디바이스 대수 제한 — 현재 무제한 (Apple ID 단위 entitlement)
- 자체 라이선스 서버 — Q4 결정에 따라 영구 미도입(엔터프라이즈 라이선스 도입 시 재논의)
- 학생/교육 할인 코드 — App Store Promo Code로 대응 가능, 별도 구현 없음
- 다국어 paywall 카피 — 1.0은 영어/한국어만, 추가 언어는 출시 후

## 12. 의존성

- **A 스펙 선행 필수**: entitlements.plist에 `com.apple.developer.in-app-payments` 포함, App ID Capabilities에 IAP 활성화. App Store Connect Product ID 등록은 D 스펙(제출)에서 다루지만 D10 sandbox 테스트 위해 D8 전 등록 필요.
- **외부 의존**: macOS 11+ (StoreKit 2 minimum). Hearth 1.0 deployment target과 일치 필요 → A 스펙 cross-check.
- **crate 신규**: `security-framework` (keychain). Rust ecosystem에서 표준.
