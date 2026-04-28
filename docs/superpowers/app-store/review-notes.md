---
field: review-notes
locale: en
char_limit: n/a
source_spec_section: "§7"
---

<!-- Pasted into App Store Connect → App Review Information → Notes on
     Day 15 [09:00]. Sandbox tester credentials are filled in on
     Day 14 [17:30] right after the account is provisioned. -->

```
=== Review Notes for Hearth 1.0 ===

Thank you for reviewing Hearth.

1. SANDBOX TESTER ACCOUNT (for In-App Purchase testing)
   Email:    <!-- TBD: needs founder review — paste sandbox tester email
              issued in App Store Connect → Users and Access → Sandbox on
              Day 14 [17:30] -->
   Password: <!-- TBD: needs founder review — paste matching password -->
   Region:   United States

   To test purchases:
   - macOS System Settings → Apple ID → Media & Purchases → Sandbox Account
   - Sign in with the credentials above

2. APP OVERVIEW
   Hearth is a local-first personal workspace combining projects, memos,
   and schedules with an AI command palette. All user data is stored in
   a local SQLite database (~/Library/Application Support/com.codewithgenie.hearth/).
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
      <!-- TBD: needs founder review — if the "Force trial expiry" debug
           menu is NOT shipped in the review build, replace lines 38-41
           with: "Trial naturally expires after 14 days. To verify
           read-only behavior during review, please advance the system
           clock or use the StoreKit test transaction simulator." -->

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
