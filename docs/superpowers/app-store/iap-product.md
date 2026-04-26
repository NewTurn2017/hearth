---
field: iap-product
locale: en+ko
char_limit: n/a
source_spec_section: "D §4.4; B (iap-license-design) §2 Q1-Q3, §4, §5, §10"
---

# In-App Purchase — Hearth Pro

## Product metadata (App Store Connect → In-App Purchases)

| Field | Value |
|-------|-------|
| Type | Non-Consumable |
| Reference Name | `Hearth Pro` |
| Product ID | `io.hearth.app.pro` |
| Price | USD $14.99 (Tier 15) |
| Family Sharing | ON |
| Cleared for Sale | Yes |
| Status target | Ready to Submit (before D submit on Day 14 [10:30]) |

<!-- TBD: needs founder review — confirm exact tier number on the day of
     entry; Apple periodically renumbers tier maps. The dollar value
     ($14.99) is locked; the tier identifier may shift. -->

## Localized display

### English (en-US)
- **Display Name**: `Hearth Pro`
- **Description**: `Unlock unlimited use of Hearth after the 14-day evaluation period.`

### Korean (ko-KR)
- **Display Name**: `Hearth Pro` (영문 그대로 음차)
- **Description**: `Hearth 14일 사용 기간 후에도 무제한으로 이용하세요.`

## Review screenshot

- **Source**: PaywallModal capture from B work (D11)
- **Min resolution**: 1024×768 or larger
- **File**: `screenshots/iap-review-paywall.png`
- **Capture conditions**:
  - Light background not required; the dark Hearth theme is acceptable
  - Show the paywall opened from a real mutation context (e.g. "Create
    project" attempt) so the reviewer sees the natural trigger path
  - Hide any personal user data — use the seeded dummy DB from the
    screenshot pipeline (D §5.1 step 2)

<!-- TBD: needs founder review — final review screenshot file path will be
     produced as part of B/D11. Update once captured. -->

## Review notes (IAP product-level)

```
This is a one-time purchase that unlocks the app beyond the 14-day
evaluation period. Test with the sandbox account provided in the app
review notes.
```

## Compliance cross-references (B spec)

- **R1** — UI copy uses "14-day evaluation period", never "free trial"
  (App Review guideline 3.1.2(b))
- **R2** — Read-only mode after expiry preserves all data export / copy /
  disconnect paths (no data hostage)
- **R3** — Restore Purchase button is permanently visible in
  Settings → License (required, non-removable)
- **R4** — Family Sharing entitlement is honored; `ownership_type=
  FAMILY_SHARED` branch verified via T10 with a sandbox family group
- **R5** — Paywall opens only on user mutation attempt, never automatically

## Sandbox testing prerequisites

- Sandbox tester account created in App Store Connect → Users and Access →
  Sandbox before Day 14 [17:30]
- macOS System Settings → Apple ID → Media & Purchases → Sandbox Account
  signed in with the tester credentials before launching the review build
- Test transaction simulator or `Configuration.storekit` available for dev
  builds
