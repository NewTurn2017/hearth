# Hearth v0.2.0 macOS 릴리즈 인프라 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `docs/superpowers/specs/2026-04-18-tauri-release-design.md` 를 구현해 Hearth v0.2.0 을 **서명 + 공증 + 오토업데이트**가 동작하는 universal macOS DMG 로 GitHub Releases 에 공개 배포한다. 한 번의 로컬 수동 릴리즈로 전체 경로를 검증하며, CI 이행·Windows/Linux·Keychain 이관은 별도 스펙 범위 밖.

**Architecture:** 추가형 (additive). Rust 측에 `tauri-plugin-updater` + `tauri-plugin-process` 를 등록하고, `src-tauri/tauri.conf.json` 에 macOS `Developer ID Application: jaehyun jang (2UANJX7ATM)` 서명 블록 + Ed25519 updater pubkey + CSP 정책을 추가한다. 프론트엔드는 기존 `ToastProvider` 를 `sticky` + 다중 action 지원으로 확장하고, `useAppUpdater` 훅이 30s + 24h 주기로 매니페스트를 확인해 토스트를 띄운다. 로컬 릴리즈 자동화는 `scripts/release.sh` (+ 보조 스크립트 3개) 가 preflight → build → codesign verify → notarize → staple → updater signature → `latest.json` → `gh release create` → post-verify 를 순차 실행한다. GitHub Releases 의 `/releases/latest/download/<file>` 퍼머링크를 고정 엔드포인트로 사용해 매 릴리즈마다 `tauri.conf.json` 을 수정하지 않아도 된다.

**Tech Stack:** Tauri 2 (+ `tauri-plugin-updater` 2, `tauri-plugin-process` 2, `tauri-plugin-dialog` 2), React 19, TypeScript 5.8, Vitest + @testing-library/react, Rust 2021, `rusqlite` 0.34, `reqwest` 0.12, macOS `codesign`, Apple `notarytool`, `xcrun stapler`, GitHub CLI (`gh`), bash + `jq`.

---

## File Structure

**Created (code/config)**

```
src-tauri/entitlements.plist                    Hardened Runtime entitlements
                                                (allow-jit, allow-unsigned-executable-memory,
                                                 network.client, files.user-selected.rw)

src/hooks/useAppUpdater.ts                      30s + 24h check loop, dismiss logic,
                                                toast trigger, relaunch on confirm
src/hooks/__tests__/useAppUpdater.test.ts       Vitest: available+new → toast; dismissed → silent;
                                                no-update → noop; offline → silent fail;
                                                confirm runs downloadAndInstall + relaunch

scripts/release.sh                              End-to-end release driver (~350 lines bash)
scripts/bump-version.sh                         Updates 3 version sites atomically
scripts/generate-manifest.sh                    Emits dist/release/latest.json
scripts/extract-release-notes.sh                Extracts a CHANGELOG section body
.env.release.example                            Self-documenting template for release credentials
```

**Created (docs)**

```
CHANGELOG.md                                    Keep-A-Changelog format, 0.2.0 entry
docs/releasing.md                               Release Day checklist + Tier 3 updater ceremony
```

**Modified**

```
src-tauri/Cargo.toml                            name "tauri-app" → "hearth",
                                                [lib] name "tauri_app_lib" → "hearth_lib",
                                                version 0.2.0, authors/desc/license/repo/readme/
                                                rust-version, + tauri-plugin-updater / -process

src-tauri/src/main.rs                           tauri_app_lib::run() → hearth_lib::run()

src-tauri/src/lib.rs                            + .plugin(tauri_plugin_updater::Builder::new().build())
                                                + .plugin(tauri_plugin_process::init())

src-tauri/capabilities/default.json             + "updater:default", "process:allow-restart"

src-tauri/tauri.conf.json                       version 0.2.0;
                                                app.security.csp set;
                                                bundle.{category,shortDescription,longDescription,
                                                  publisher,copyright,homepage,
                                                  createUpdaterArtifacts,macOS.*};
                                                plugins.updater.{pubkey,endpoints}

src/ui/Toast.tsx                                + api.info(message, { sticky?, actions? });
                                                actions render as extra buttons;
                                                sticky skips TTL

src/components/Layout.tsx                       + useAppUpdater() mount (one-time)

package.json                                    version 0.2.0,
                                                author/license/repository/bugs/homepage,
                                                scripts.release "bash scripts/release.sh"

README.md                                       Installation 섹션 교체 (DMG + Gatekeeper + 업데이트);
                                                + shields.io release 배지;
                                                + 짧은 Releasing 섹션 (docs/releasing.md 링크)

.gitignore                                      + .env.release (실제 자격증명)
                                                + dist/release/            (스크립트 중간 산출)
```

**Credentials (레포 밖, 이미 존재 또는 Task 8 에서 생성)**

```
/Users/genie/dev/private/apple_developer/
  인증서.p12                              (이미 존재)
  AuthKey_Z2V325X3FY.p8                (이미 존재, Issuer a3452585-1d50-4353-bc5b-6a36da9452ad)
  hearth_updater.key                   Ed25519 private, passphrase 암호화  [Task 8]
  hearth_updater.key.pub               Ed25519 public                        [Task 8]
```

---

## Prerequisites (set up once before Task 1)

- macOS 15+ (현재 darwin 25.4.0 ✓)
- Xcode Command Line Tools — `xcode-select --install` 1회. `xcrun --find notarytool` + `xcrun --find stapler` 가 경로 반환해야 함
- Rust 1.75+ + 두 타겟: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- Node 20+, npm 10+
- `gh` CLI 인증 (`gh auth status`) — NewTurn2017 계정
- `jq` (Homebrew), `openssl` (시스템 기본)
- 키체인에 `Developer ID Application: jaehyun jang (2UANJX7ATM)` — `security find-identity -v -p codesigning` 로 확인 (이미 설치됨)

이 중 하나라도 빠졌으면 Task 1 진입 전에 해결. 모든 후속 태스크는 위 전제를 기반.

---

## Task 1: Rename Cargo package + lib + main.rs callsite

**Files:**
- Modify: `src-tauri/Cargo.toml:1-7` (package name 만)
- Modify: `src-tauri/Cargo.toml:13-16` (lib.name)
- Modify: `src-tauri/src/main.rs:5`

- [ ] **Step 1: Patch Cargo.toml package.name**

Open `src-tauri/Cargo.toml`. Replace line 2:

```toml
name = "tauri-app"
```

with:

```toml
name = "hearth"
```

- [ ] **Step 2: Patch [lib] name**

In the same file, replace line 14:

```toml
name = "tauri_app_lib"
```

with:

```toml
name = "hearth_lib"
```

- [ ] **Step 3: Patch main.rs callsite**

In `src-tauri/src/main.rs` line 5, replace:

```rust
    tauri_app_lib::run()
```

with:

```rust
    hearth_lib::run()
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo build --lib`
Expected: compiles without errors. (Binary build is unnecessary here; only verify the lib rename.)

- [ ] **Step 5: Run existing tests**

Run: `cd src-tauri && cargo test`
Expected: 21 tests pass. (No test references `tauri_app_lib` by name — all live inside the crate.)

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth
git add src-tauri/Cargo.toml src-tauri/src/main.rs
git commit -m "refactor(cargo): rename package tauri-app → hearth"
```

---

## Task 2: Cargo.toml metadata fields

**Files:**
- Modify: `src-tauri/Cargo.toml:1-9` (package block)

- [ ] **Step 1: Replace the `[package]` block**

Open `src-tauri/Cargo.toml`. Replace lines 1–6 (the entire `[package]` block) with:

```toml
[package]
name = "hearth"
version = "0.2.0"
description = "Local-first personal workspace for projects, memos, and schedules."
authors = ["Jaehyun Jang <hyuni2020@gmail.com>"]
license = "MIT"
repository = "https://github.com/NewTurn2017/hearth"
readme = "../README.md"
edition = "2021"
rust-version = "1.75"
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo build --lib`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore(cargo): populate package metadata (authors, license, repo, rust-version)"
```

---

## Task 3: Add updater + process plugin deps

**Files:**
- Modify: `src-tauri/Cargo.toml` (`[dependencies]` block, ~lines 20-30)

- [ ] **Step 1: Append two dependencies**

Open `src-tauri/Cargo.toml`. Locate the `[dependencies]` block and add two lines at the end (before any following blank line or block):

```toml
tauri-plugin-updater = "2"
tauri-plugin-process = "2"
```

- [ ] **Step 2: Fetch and verify**

Run: `cd src-tauri && cargo fetch`
Expected: downloads the two new crates.

Run: `cd src-tauri && cargo build --lib`
Expected: compiles (plugins unused at this point — that's fine until Task 10).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat(tauri): add updater + process plugin dependencies"
```

---

## Task 4: package.json metadata + release script entry

**Files:**
- Modify: `package.json`

- [ ] **Step 1: Patch package.json**

Open `package.json` and produce this final state (preserving the existing `dependencies` / `devDependencies` blocks verbatim):

```json
{
  "name": "hearth",
  "version": "0.2.0",
  "private": true,
  "type": "module",
  "description": "Local-first personal workspace for projects, memos, and schedules — with an AI command palette.",
  "author": "Jaehyun Jang <hyuni2020@gmail.com>",
  "license": "MIT",
  "homepage": "https://github.com/NewTurn2017/hearth",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/NewTurn2017/hearth.git"
  },
  "bugs": {
    "url": "https://github.com/NewTurn2017/hearth/issues"
  },
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest",
    "release": "bash scripts/release.sh"
  },
  "dependencies": { … preserve as-is … },
  "devDependencies": { … preserve as-is … }
}
```

(Use the actual dependency/devDependency blocks from the current file; do not hand-merge.)

- [ ] **Step 2: Verify**

Run: `jq -r .version package.json && jq -r .author package.json && jq -r .scripts.release package.json`
Expected:
```
0.2.0
Jaehyun Jang <hyuni2020@gmail.com>
bash scripts/release.sh
```

- [ ] **Step 3: Commit**

```bash
git add package.json
git commit -m "chore(pkg): populate package.json metadata + release script entry"
```

---

## Task 5: Create entitlements.plist

**Files:**
- Create: `src-tauri/entitlements.plist`

- [ ] **Step 1: Write the file**

Write `src-tauri/entitlements.plist`:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
</dict>
</plist>
```

- [ ] **Step 2: Validate plist syntax**

Run: `plutil -lint src-tauri/entitlements.plist`
Expected: `src-tauri/entitlements.plist: OK`

- [ ] **Step 3: Commit**

```bash
git add src-tauri/entitlements.plist
git commit -m "feat(tauri): add Hardened Runtime entitlements for Developer ID builds"
```

---

## Task 6: tauri.conf.json — version + bundle + macOS signing + CSP

**Files:**
- Modify: `src-tauri/tauri.conf.json`

This is one coordinated JSON edit since all fields live in the same file.

- [ ] **Step 1: Rewrite tauri.conf.json**

Replace the entire contents of `src-tauri/tauri.conf.json` with:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Hearth",
  "version": "0.2.0",
  "identifier": "com.newturn2017.hearth",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "Hearth",
        "width": 1200,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600
      }
    ],
    "security": {
      "csp": "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost https://api.openai.com http://127.0.0.1:18080 http://localhost:18080; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; object-src 'none'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "createUpdaterArtifacts": true,
    "category": "public.app-category.productivity",
    "shortDescription": "Local-first personal workspace.",
    "longDescription": "Hearth is a local-first personal workspace that unifies projects, sticky memos, and schedules, driven by an AI command palette that runs entirely on-device (MLX) or via OpenAI.",
    "publisher": "Jaehyun Jang",
    "copyright": "© 2026 Jaehyun Jang",
    "homepage": "https://github.com/NewTurn2017/hearth",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "signingIdentity": "Developer ID Application: jaehyun jang (2UANJX7ATM)",
      "providerShortName": "2UANJX7ATM",
      "entitlements": "entitlements.plist",
      "minimumSystemVersion": "11.0"
    }
  },
  "plugins": {
    "updater": {
      "pubkey": "REPLACE_WITH_PUBKEY_IN_TASK_7",
      "endpoints": [
        "https://github.com/NewTurn2017/hearth/releases/latest/download/latest.json"
      ]
    }
  }
}
```

Note: `plugins.updater.pubkey` is a deliberate placeholder; Task 7 will overwrite it with the real generated public key. Leaving it as-is means `tauri build` would fail at the manifest verification step, which is fine because we won't build until after Task 7 lands.

- [ ] **Step 2: Validate JSON**

Run: `jq . src-tauri/tauri.conf.json > /dev/null && echo ok`
Expected: `ok`

- [ ] **Step 3: Quick dev smoke (CSP)**

Run: `npm run tauri dev` in one terminal. Let it boot (~20s). Open DevTools (`⌘⌥I`), Console tab.

Expected: the app loads. Zero `Refused to ... Content Security Policy` violations in console. If any appear, widen the CSP **only** for the exact directive that failed (e.g., if a Tailwind 4 inline `<style>` is blocked, confirm `style-src` has `'unsafe-inline'`) and re-run. Document the adjustment in the commit message.

Quit `tauri dev` when satisfied.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat(tauri): v0.2.0 bundle metadata + macOS signing + CSP policy

- Developer ID Application signing with entitlements + Hardened Runtime
- createUpdaterArtifacts for tauri-plugin-updater payloads
- CSP restricts sources to self + OpenAI + local MLX + Tauri IPC
- Placeholder pubkey (replaced in next task)"
```

---

## Task 7: Generate Ed25519 updater keypair + embed pubkey

**Files:**
- Create (outside repo): `/Users/genie/dev/private/apple_developer/hearth_updater.key` + `.pub`
- Modify: `src-tauri/tauri.conf.json` (`plugins.updater.pubkey`)

- [ ] **Step 1: Generate keypair**

Run (interactive — prompts for passphrase twice):

```bash
cd /Users/genie/dev/tools/hearth
npx tauri signer generate -w /Users/genie/dev/private/apple_developer/hearth_updater.key
```

Expected output includes two file paths:
- `/Users/genie/dev/private/apple_developer/hearth_updater.key`
- `/Users/genie/dev/private/apple_developer/hearth_updater.key.pub`

**Save the passphrase** to 1Password (or equivalent secure store) under "Hearth updater signing key." This passphrase plus the `.key` file together are the only way to ship a new `.app.tar.gz`.

- [ ] **Step 2: Copy pubkey into tauri.conf.json**

Get the pubkey content (Base64 single line):

```bash
cat /Users/genie/dev/private/apple_developer/hearth_updater.key.pub
```

Replace the placeholder `"REPLACE_WITH_PUBKEY_IN_TASK_8"` in `src-tauri/tauri.conf.json` with that exact Base64 string (wrapped in quotes).

- [ ] **Step 3: Verify JSON still parses**

Run: `jq -r '.plugins.updater.pubkey' src-tauri/tauri.conf.json | wc -c`
Expected: a number > 40 (Ed25519 pubkey is ~88 chars Base64).

- [ ] **Step 4: Back up the private key offline**

Copy `hearth_updater.key` to a second location (external SSD or encrypted vault), with the passphrase stored alongside in the password manager. If the key is lost, every installed Hearth is frozen at its current version until the user manually re-downloads a new DMG with a new pubkey — this is a *one-way failure*.

No commit in this sub-step; the file is outside the repo.

- [ ] **Step 5: Commit the pubkey embed**

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat(updater): embed Ed25519 public key for release signing"
```

---

## Task 8: Register updater + process plugins + capabilities

**Files:**
- Modify: `src-tauri/src/lib.rs:24-26` (Builder chain)
- Modify: `src-tauri/capabilities/default.json` (permissions array)

- [ ] **Step 1: Extend the Tauri Builder chain**

Open `src-tauri/src/lib.rs`. Replace lines 24–26:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
```

with:

```rust
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
```

- [ ] **Step 2: Extend capabilities**

Open `src-tauri/capabilities/default.json`. Replace its contents with:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:default",
    "updater:default",
    "process:allow-restart"
  ]
}
```

- [ ] **Step 3: Verify**

Run: `cd src-tauri && cargo build --lib`
Expected: compiles.

Run: `cd src-tauri && cargo test`
Expected: 21 tests pass (no regressions).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat(tauri): register updater + process plugins with capabilities"
```

---

## Task 9: Extend Toast primitive — sticky + actions

**Files:**
- Modify: `src/ui/Toast.tsx`
- Create: `src/ui/__tests__/Toast.test.tsx`

- [ ] **Step 1: Write the failing test**

Create `src/ui/__tests__/Toast.test.tsx`:

```tsx
import { describe, it, expect, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ToastProvider, useToast } from "../Toast";

function Trigger({ onReady }: { onReady: (api: ReturnType<typeof useToast>) => void }) {
  const api = useToast();
  onReady(api);
  return null;
}

function setup() {
  let captured: ReturnType<typeof useToast> | null = null;
  render(
    <ToastProvider>
      <Trigger onReady={(a) => { captured = a; }} />
    </ToastProvider>
  );
  return () => captured!;
}

describe("ToastProvider.info", () => {
  it("renders message with multiple action buttons", () => {
    const getApi = setup();
    const run1 = vi.fn();
    const run2 = vi.fn();
    act(() => {
      getApi().info("새 버전 v0.3.0 준비됨", {
        sticky: true,
        actions: [
          { label: "지금 재시작", run: run1 },
          { label: "나중에", run: run2 },
        ],
      });
    });
    expect(screen.getByText("새 버전 v0.3.0 준비됨")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "지금 재시작" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "나중에" })).toBeInTheDocument();
  });

  it("invokes the action's run on click and dismisses the toast", async () => {
    const user = userEvent.setup();
    const getApi = setup();
    const run = vi.fn();
    act(() => {
      getApi().info("hello", {
        sticky: true,
        actions: [{ label: "ok", run }],
      });
    });
    await user.click(screen.getByRole("button", { name: "ok" }));
    expect(run).toHaveBeenCalledTimes(1);
    expect(screen.queryByText("hello")).not.toBeInTheDocument();
  });

  it("sticky toast does not auto-dismiss", () => {
    vi.useFakeTimers();
    const getApi = setup();
    act(() => {
      getApi().info("stays", { sticky: true, actions: [] });
    });
    act(() => { vi.advanceTimersByTime(10_000); });
    expect(screen.getByText("stays")).toBeInTheDocument();
    vi.useRealTimers();
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test -- src/ui/__tests__/Toast.test.tsx`
Expected: FAIL — `getApi().info is not a function` (method doesn't exist yet).

- [ ] **Step 3: Extend the Toast primitive**

Replace the entire contents of `src/ui/Toast.tsx` with:

```tsx
// src/ui/Toast.tsx
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { CheckCircle2, AlertCircle, Info, RotateCcw, X } from "lucide-react";
import { cn } from "../lib/cn";
import { Icon } from "./Icon";

type Kind = "success" | "error" | "info";

export interface ToastAction {
  label: string;
  run: () => void | Promise<void>;
}

interface ToastItem {
  id: number;
  kind: Kind;
  message: string;
  undo?: () => void | Promise<void>;
  sticky?: boolean;
  actions?: ToastAction[];
}

interface InfoOpts {
  sticky?: boolean;
  actions?: ToastAction[];
}

interface ToastApi {
  success: (message: string, opts?: { undo?: () => void | Promise<void> }) => void;
  error: (message: string) => void;
  info: (message: string, opts?: InfoOpts) => void;
}

const ToastCtx = createContext<ToastApi | null>(null);

const TTL_MS = 5000;

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const idRef = useRef(0);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const remove = useCallback((id: number) => {
    setItems((prev) => prev.filter((t) => t.id !== id));
    const timer = timers.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const push = useCallback(
    (item: Omit<ToastItem, "id">) => {
      const id = ++idRef.current;
      setItems((prev) => [...prev, { ...item, id }]);
      if (!item.sticky) {
        const timer = setTimeout(() => remove(id), TTL_MS);
        timers.current.set(id, timer);
      }
    },
    [remove]
  );

  const api: ToastApi = {
    success: (message, opts) =>
      push({ kind: "success", message, undo: opts?.undo }),
    error: (message) => push({ kind: "error", message }),
    info: (message, opts) =>
      push({
        kind: "info",
        message,
        sticky: opts?.sticky,
        actions: opts?.actions,
      }),
  };

  useEffect(() => () => timers.current.forEach(clearTimeout), []);

  return (
    <ToastCtx.Provider value={api}>
      {children}
      <div className="fixed bottom-4 right-4 z-[200] flex flex-col gap-2 pointer-events-none">
        {items.map((t) => {
          const iconMap = {
            success: CheckCircle2,
            error: AlertCircle,
            info: Info,
          } as const;
          const tintMap = {
            success: "text-[var(--color-success)]",
            error: "text-[var(--color-danger)]",
            info: "text-[var(--color-brand-hi)]",
          } as const;
          const borderMap = {
            success: "border-[var(--color-border)]",
            error: "border-[var(--color-danger)]",
            info: "border-[var(--color-brand-hi)]",
          } as const;
          return (
            <div
              key={t.id}
              className={cn(
                "pointer-events-auto flex items-center gap-2 min-w-[260px] max-w-[420px]",
                "px-3 py-2 rounded-[var(--radius-md)] border shadow-[var(--shadow-e2)]",
                "bg-[var(--color-surface-2)]",
                borderMap[t.kind]
              )}
            >
              <Icon icon={iconMap[t.kind]} size={16} className={tintMap[t.kind]} />
              <span className="flex-1 text-[12px] text-[var(--color-text)]">
                {t.message}
              </span>
              {t.undo && (
                <button
                  onClick={async () => {
                    try {
                      await t.undo!();
                    } finally {
                      remove(t.id);
                    }
                  }}
                  className="inline-flex items-center gap-1 text-[11px] text-[var(--color-brand-hi)] hover:underline"
                >
                  <Icon icon={RotateCcw} size={14} />
                  Undo
                </button>
              )}
              {t.actions?.map((a) => (
                <button
                  key={a.label}
                  onClick={async () => {
                    try {
                      await a.run();
                    } finally {
                      remove(t.id);
                    }
                  }}
                  className="inline-flex items-center text-[11px] text-[var(--color-brand-hi)] hover:underline whitespace-nowrap"
                >
                  {a.label}
                </button>
              ))}
              <button
                onClick={() => remove(t.id)}
                className="text-[var(--color-text-dim)] hover:text-[var(--color-text)]"
                aria-label="dismiss"
              >
                <Icon icon={X} size={14} />
              </button>
            </div>
          );
        })}
      </div>
    </ToastCtx.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastCtx);
  if (!ctx) throw new Error("useToast must be used inside <ToastProvider>");
  return ctx;
}
```

- [ ] **Step 4: Run the new tests**

Run: `npm test -- src/ui/__tests__/Toast.test.tsx`
Expected: 3 tests pass.

- [ ] **Step 5: Run the full frontend test suite (no regressions)**

Run: `npm test`
Expected: all pre-existing tests still pass (19 + 3 new).

- [ ] **Step 6: Commit**

```bash
git add src/ui/Toast.tsx src/ui/__tests__/Toast.test.tsx
git commit -m "feat(ui): Toast info variant with sticky + multi-action support"
```

---

## Task 10: `useAppUpdater` hook + Vitest

**Files:**
- Create: `src/hooks/useAppUpdater.ts`
- Create: `src/hooks/__tests__/useAppUpdater.test.ts`

- [ ] **Step 1: Write the failing tests**

Create `src/hooks/__tests__/useAppUpdater.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";

const checkMock = vi.fn();
const relaunchMock = vi.fn();

vi.mock("@tauri-apps/plugin-updater", () => ({
  check: (...args: unknown[]) => checkMock(...args),
}));
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: (...args: unknown[]) => relaunchMock(...args),
}));

let capturedInfoArgs: unknown[] = [];
const infoMock = vi.fn((...args: unknown[]) => {
  capturedInfoArgs = args;
});

vi.mock("../../ui/Toast", () => ({
  useToast: () => ({
    success: vi.fn(),
    error: vi.fn(),
    info: infoMock,
  }),
}));

import { useAppUpdater } from "../useAppUpdater";

function drainPromises() {
  return new Promise<void>((r) => setTimeout(r, 0));
}

beforeEach(() => {
  vi.useFakeTimers();
  checkMock.mockReset();
  relaunchMock.mockReset();
  infoMock.mockReset();
  capturedInfoArgs = [];
  localStorage.clear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAppUpdater", () => {
  it("shows toast when an update is available", async () => {
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    expect(infoMock).toHaveBeenCalledTimes(1);
    const [message, opts] = capturedInfoArgs as [string, { sticky: boolean; actions: { label: string }[] }];
    expect(message).toContain("0.3.0");
    expect(opts.sticky).toBe(true);
    expect(opts.actions.map((a) => a.label)).toEqual(["지금 재시작", "나중에"]);
  });

  it("does not toast when update.available is false", async () => {
    checkMock.mockResolvedValue({ available: false });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("does not toast when dismissedVersion matches", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({ available: true, version: "0.3.0" });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("toasts again for a newer version than dismissed", async () => {
    localStorage.setItem("updater.dismissedVersion", "0.3.0");
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.1",
      downloadAndInstall: vi.fn().mockResolvedValue(undefined),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(infoMock).toHaveBeenCalledTimes(1);
  });

  it("swallows errors from check() (offline)", async () => {
    checkMock.mockRejectedValue(new Error("offline"));
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(infoMock).not.toHaveBeenCalled();
  });

  it("confirm action downloads + relaunches", async () => {
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall,
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    const [, opts] = capturedInfoArgs as [string, { actions: { label: string; run: () => Promise<void> }[] }];
    const confirm = opts.actions.find((a) => a.label === "지금 재시작")!;
    await act(async () => {
      await confirm.run();
    });
    expect(downloadAndInstall).toHaveBeenCalledTimes(1);
    expect(relaunchMock).toHaveBeenCalledTimes(1);
  });

  it("dismiss action persists the version to localStorage", async () => {
    checkMock.mockResolvedValue({
      available: true,
      version: "0.3.0",
      downloadAndInstall: vi.fn(),
    });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    const [, opts] = capturedInfoArgs as [string, { actions: { label: string; run: () => void }[] }];
    const dismiss = opts.actions.find((a) => a.label === "나중에")!;
    act(() => {
      dismiss.run();
    });
    expect(localStorage.getItem("updater.dismissedVersion")).toBe("0.3.0");
  });

  it("re-checks after 24h interval", async () => {
    checkMock.mockResolvedValue({ available: false });
    renderHook(() => useAppUpdater());
    await act(async () => {
      vi.advanceTimersByTime(30_001);
      await drainPromises();
    });
    expect(checkMock).toHaveBeenCalledTimes(1);
    await act(async () => {
      vi.advanceTimersByTime(24 * 60 * 60 * 1000);
      await drainPromises();
    });
    expect(checkMock).toHaveBeenCalledTimes(2);
  });
});
```

- [ ] **Step 2: Run to verify failure**

Run: `npm test -- src/hooks/__tests__/useAppUpdater.test.ts`
Expected: FAIL — `Cannot find module '../useAppUpdater'`.

- [ ] **Step 3: Implement the hook**

Create `src/hooks/useAppUpdater.ts`:

```ts
import { useEffect } from "react";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useToast } from "../ui/Toast";

const STARTUP_DELAY_MS = 30_000;
const PERIODIC_MS = 24 * 60 * 60 * 1000;
const DISMISS_KEY = "updater.dismissedVersion";

export function useAppUpdater(): void {
  const toast = useToast();

  useEffect(() => {
    let cancelled = false;

    async function runCheck() {
      if (cancelled) return;
      let update: Awaited<ReturnType<typeof check>> = null;
      try {
        update = await check();
      } catch {
        return;
      }
      if (cancelled || !update?.available) return;
      if (localStorage.getItem(DISMISS_KEY) === update.version) return;

      toast.info(`새 버전 ${update.version} 준비됨`, {
        sticky: true,
        actions: [
          {
            label: "지금 재시작",
            run: async () => {
              await update!.downloadAndInstall();
              await relaunch();
            },
          },
          {
            label: "나중에",
            run: () => {
              localStorage.setItem(DISMISS_KEY, update!.version);
            },
          },
        ],
      });
    }

    const startTimer = setTimeout(runCheck, STARTUP_DELAY_MS);
    const intervalTimer = setInterval(runCheck, PERIODIC_MS);

    return () => {
      cancelled = true;
      clearTimeout(startTimer);
      clearInterval(intervalTimer);
    };
    // toast is stable from context; react-hooks lint will accept [toast]
  }, [toast]);
}
```

- [ ] **Step 4: Run tests to verify pass**

Run: `npm test -- src/hooks/__tests__/useAppUpdater.test.ts`
Expected: 8 tests pass.

- [ ] **Step 5: Verify full suite**

Run: `npm test`
Expected: all existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add src/hooks/useAppUpdater.ts src/hooks/__tests__/useAppUpdater.test.ts
git commit -m "feat(hooks): useAppUpdater — 30s + 24h check loop with dismiss"
```

---

## Task 11: Wire `useAppUpdater` into Layout

**Files:**
- Modify: `src/components/Layout.tsx` (one import + one hook call)

- [ ] **Step 1: Add the import**

Open `src/components/Layout.tsx`. Near the other hook imports at the top of the file, add:

```tsx
import { useAppUpdater } from "../hooks/useAppUpdater";
```

- [ ] **Step 2: Call the hook inside `Layout`**

Locate the `Layout` component body. Add `useAppUpdater();` as the first statement inside the component function (above any existing `useState` / `useEffect`). Example:

```tsx
export function Layout(/* existing props */) {
  useAppUpdater();
  // … existing hooks …
}
```

The hook is self-contained (uses only `useToast` from context), so placement just needs to be inside the provider tree. `App.tsx` already wraps `<Layout>` with `<ToastProvider>`, so this is safe.

- [ ] **Step 3: Run full frontend suite**

Run: `npm test`
Expected: all tests pass.

- [ ] **Step 4: Smoke via `tauri dev`**

Run: `npm run tauri dev`
Wait for app to boot. The updater call will fire after 30s and silently fail (endpoint `latest.json` has no `0.2.0` entry yet). Open DevTools → Console. Expected: no errors, no toast (because no update is advertised above current 0.2.0). Quit.

- [ ] **Step 5: Commit**

```bash
git add src/components/Layout.tsx
git commit -m "feat(layout): mount useAppUpdater on app start"
```

---

## Task 12: `CHANGELOG.md` with 0.2.0 entry

**Files:**
- Create: `CHANGELOG.md`

- [ ] **Step 1: Write the file**

Create `CHANGELOG.md` in repo root:

```markdown
# Changelog

All notable changes to Hearth are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-04-18

First public release.

### Added
- macOS universal DMG (aarch64 + x86_64), signed with Developer ID and notarized by Apple.
- Auto-updater (`tauri-plugin-updater`): checks for updates 30 seconds after launch and every 24 hours. When a newer version is available, a toast offers "지금 재시작" (install now) or "나중에" (skip this version).
- `CHANGELOG.md` + `docs/releasing.md` + `scripts/release.sh` release tooling.
- Content Security Policy restricting network access to `self`, the OpenAI API, and the local MLX endpoint.

### Changed
- Cargo package renamed `tauri-app` → `hearth`; library name `tauri_app_lib` → `hearth_lib`.
- `package.json` / `Cargo.toml` / `tauri.conf.json` populated with authorship, license (MIT), and repository metadata.
- README Installation section now points to the GitHub Releases DMG.

### Known limitations
- Windows and Linux builds are not yet distributed.
- The OpenAI API key is still stored in the local SQLite database in plain text; migration to the macOS Keychain is tracked in a separate spec.

[Unreleased]: https://github.com/NewTurn2017/hearth/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.0
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add CHANGELOG with 0.2.0 entry"
```

---

## Task 13: Rewrite README Installation section

**Files:**
- Modify: `README.md` (Installation section, approximately lines 40–52)

- [ ] **Step 1: Locate the Installation section**

In `README.md`, find the block that currently reads:

```markdown
## Installation

릴리즈 바이너리는 아직 없습니다. 지금은 [Building from Source](#building-from-source) 섹션을 참고해 직접 빌드해 주세요.
```

(and the "시스템 요구사항" table that follows).

- [ ] **Step 2: Replace with**

```markdown
## Installation

### macOS (공식 릴리즈)

1. [최신 릴리즈 페이지](https://github.com/NewTurn2017/hearth/releases/latest)에서 `Hearth_<버전>_universal.dmg` 다운로드
2. DMG 더블클릭 → 열린 창에서 **Hearth.app** 을 **Applications** 폴더로 드래그
3. **첫 실행만** Finder 에서 `Applications/Hearth.app` 우클릭 → **열기** (Gatekeeper 확인 1회)
4. 이후에는 일반 앱처럼 실행

> 앱이 공증 (notarization) 된 상태라 "알 수 없는 개발자" 경고는 뜨지 않습니다. 첫 실행 시 네트워크 지연으로 "인터넷에서 다운로드되었습니다" 확인 프롬프트가 1회 뜰 수 있어요 — 그냥 "열기"로 통과.

### 업데이트

앱을 켜 두면 30초 뒤부터, 그리고 이후 24시간마다 새 버전을 자동으로 확인합니다. 새 버전이 있으면 우측 하단에 토스트가 뜨고, **지금 재시작** 을 누르면 2-3초 안에 새 버전으로 교체됩니다. **나중에** 를 누르면 해당 버전은 다음 릴리즈가 나오기 전까지 다시 조르지 않습니다.

### Windows / Linux

아직 공식 빌드 없음. 원하면 [Building from Source](#building-from-source) 참고.

### 시스템 요구사항

| 항목 | 최소 사양 |
|------|----------|
| OS | macOS 11+ (Big Sur), Windows 10+, Linux (glibc 2.31+) |
| 메모리 | 4 GB (MLX 백엔드 쓰면 16 GB 권장) |
| 저장 공간 | 150 MB (MLX 모델 별도, ~3 GB) |
| 기타 | Rust 1.75+, Node.js 20+, npm (소스 빌드 시) |
```

Keep all other README content unchanged.

- [ ] **Step 3: Optional — add release badge near top**

At the top of README, below the existing Tauri/React/Rust/License/Platform badges, add:

```markdown
[![Latest Release](https://img.shields.io/github/v/release/NewTurn2017/hearth?display_name=tag&sort=semver)](https://github.com/NewTurn2017/hearth/releases/latest)
```

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs(readme): rewrite Installation for the v0.2.0 DMG release"
```

---

## Task 14: `docs/releasing.md` — Release Day & Tier 3 checklist

**Files:**
- Create: `docs/releasing.md`

- [ ] **Step 1: Write the file**

Create `docs/releasing.md`:

```markdown
# Releasing Hearth

End-to-end checklist for cutting a new macOS release. Runs fully from a developer Mac with the credentials stored in `/Users/genie/dev/private/apple_developer/`. See [the release design spec](superpowers/specs/2026-04-18-tauri-release-design.md) for the rationale behind each step.

## Prerequisites (once)

- `Developer ID Application: jaehyun jang (2UANJX7ATM)` in keychain
- `.env.release` filled from `.env.release.example`
- `/Users/genie/dev/private/apple_developer/hearth_updater.key` (+ passphrase)
- `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- `gh auth status` OK
- `xcrun --find notarytool && xcrun --find stapler` both resolve

## Bump version

```bash
./scripts/bump-version.sh 0.3.0   # pick the next semver
```

This updates `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

## Write the changelog entry

Add a new `## [0.3.0] - YYYY-MM-DD` section to `CHANGELOG.md` with the user-visible changes in this release (`Added` / `Changed` / `Fixed` / `Removed`). Commit.

## Dry-run the release

```bash
./scripts/release.sh --dry-run
```

This runs steps 1–9 (build, codesign verify, notarize, staple, sign updater tarball, build `latest.json`, extract release notes) but stops before tagging and publishing. Review `dist/release/latest.json` and `dist/release/notes.md` for sanity.

## Tier 3: Updater round-trip dry-run (first release only)

Skip this section for releases **after** v0.2.0 — it only exists to prove the updater works before any user installs v0.2.0. For v0.3.0+, rely on the fact that real v0.2.0 → v0.3.0 auto-update verifies the path implicitly.

For the **first** release (v0.2.0):

1. Check out a throwaway branch. Temporarily set `version = "0.1.9"` in all three manifests and `pubkey` unchanged.
2. `npm run tauri build -- --target universal-apple-darwin` — you get a local `fake-0.1.9.app`.
3. In a macOS guest/test user account, copy the fake app into `~/Applications` and launch.
4. While that fake app is running, upload the real v0.2.0 build to a **private** test GitHub Release (draft, or a separate test repo). Point the fake build's endpoint at that test `latest.json`.
5. Wait ~30 s after app launch. The "새 버전 0.2.0 준비됨" toast should appear.
6. Click **지금 재시작**. Within 2–3 seconds the app relaunches; window title remains "Hearth" and the DB (projects/memos/etc.) persists.
7. Delete the test release/repo; discard the throwaway branch; confirm the real `tauri.conf.json` endpoint is back to production.

If any step fails, fix in `scripts/release.sh` or `useAppUpdater.ts` before cutting v0.2.0.

## Real release

```bash
./scripts/release.sh
```

Expected elapsed time: 10–20 minutes, dominated by notarization (Apple typically responds in 2–5 min but the SLA is up to 1 hour). The script streams progress and aborts on any non-`Accepted` status.

On success: a new `vX.Y.Z` tag is pushed, a GitHub Release is published with 4 assets, and the script prints the Release URL.

## Tier 2: Post-release smoke

Perform once per release in a macOS guest user account (to reproduce first-run Gatekeeper):

- [ ] Download DMG from Releases → Applications
- [ ] Right-click → Open (1 Gatekeeper prompt max)
- [ ] Main window renders (sidebar + tabs)
- [ ] `⌘K` palette opens
- [ ] Create a project; quit; reopen; project persists
- [ ] Settings → AI → OpenAI key → `프로젝트 목록 보여줘` returns
- [ ] Settings → AI → provider `local` + MLX running → chat responds
- [ ] Settings → Backup → 지금 백업 → file created in configured dir
- [ ] `~/Library/Application Support/com.newturn2017.hearth/data.db` exists

Any failure → yank (`gh release delete vX.Y.Z && git push --delete origin vX.Y.Z`), fix, roll forward with vX.Y.(Z+1). Do **not** edit an existing release in place.

## Rollback

Prefer a roll-forward (vX.Y.(Z+1)). Only yank for security incidents; the automatic update channel will pull the new release to all existing users within ~24 hours of their next launch.
```

- [ ] **Step 2: Commit**

```bash
git add docs/releasing.md
git commit -m "docs: release day + Tier 3 updater dry-run checklist"
```

---

## Task 15: `.gitignore` + `.env.release.example`

**Files:**
- Modify: `.gitignore`
- Create: `.env.release.example`

- [ ] **Step 1: Extend `.gitignore`**

Append to `/Users/genie/dev/tools/hearth/.gitignore`:

```gitignore

# Release pipeline
.env.release
dist/release/
```

- [ ] **Step 2: Create `.env.release.example`**

Create `/Users/genie/dev/tools/hearth/.env.release.example`:

```bash
# Copy this file to .env.release and fill in values.
# .env.release is git-ignored. Never commit the real file.
#
# These credentials drive scripts/release.sh. See docs/releasing.md
# and docs/superpowers/specs/2026-04-18-tauri-release-design.md for context.

# --- Apple App Store Connect API (for notarization via notarytool) ---
# .p8 key file stored outside this repo.
APPLE_API_KEY_PATH=/Users/genie/dev/private/apple_developer/AuthKey_Z2V325X3FY.p8
APPLE_API_KEY_ID=Z2V325X3FY
APPLE_API_ISSUER=a3452585-1d50-4353-bc5b-6a36da9452ad

# --- Tauri updater signing (Ed25519) ---
# Generated by: `npx tauri signer generate -w <path>`
# The matching public key is embedded in src-tauri/tauri.conf.json.
TAURI_SIGNING_PRIVATE_KEY=/Users/genie/dev/private/apple_developer/hearth_updater.key
TAURI_SIGNING_PRIVATE_KEY_PASSWORD=change-me

# --- GitHub ---
# Repository slug used by gh release create. Change if you fork.
GH_REPO=NewTurn2017/hearth
```

- [ ] **Step 3: Commit**

```bash
git add .gitignore .env.release.example
git commit -m "chore: gitignore release artifacts + .env.release template"
```

---

## Task 16: `scripts/bump-version.sh`

**Files:**
- Create: `scripts/bump-version.sh`

- [ ] **Step 1: Write the script**

Create `scripts/bump-version.sh`:

```bash
#!/usr/bin/env bash
# Bump the Hearth version in all three manifests atomically.
# Usage: ./scripts/bump-version.sh 0.3.0
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <semver>" >&2
  exit 64
fi

NEW="$1"

# Basic semver shape check (X.Y.Z with optional -prerelease)
if ! [[ "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$ ]]; then
  echo "Error: '$NEW' is not a plausible semver string." >&2
  exit 65
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# package.json
tmp="$(mktemp)"
jq --arg v "$NEW" '.version = $v' package.json > "$tmp"
mv "$tmp" package.json

# src-tauri/tauri.conf.json
tmp="$(mktemp)"
jq --arg v "$NEW" '.version = $v' src-tauri/tauri.conf.json > "$tmp"
mv "$tmp" src-tauri/tauri.conf.json

# src-tauri/Cargo.toml — bespoke edit of the first `version = "…"` after [package]
python3 - "$NEW" <<'PY'
import pathlib, re, sys
new = sys.argv[1]
p = pathlib.Path("src-tauri/Cargo.toml")
text = p.read_text()
out = re.sub(
    r'(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")',
    lambda m: m.group(1) + new + m.group(3),
    text,
    count=1,
)
if out == text:
    sys.exit("Failed to substitute version in Cargo.toml")
p.write_text(out)
PY

echo "Bumped to $NEW in package.json, tauri.conf.json, Cargo.toml"
```

- [ ] **Step 2: Make executable**

Run: `chmod +x scripts/bump-version.sh`

- [ ] **Step 3: Test (idempotently — bump to current value)**

Run: `./scripts/bump-version.sh 0.2.0`
Expected:
```
Bumped to 0.2.0 in package.json, tauri.conf.json, Cargo.toml
```
Run: `git diff --stat`
Expected: zero changes (since we bumped to the value already present).

- [ ] **Step 4: Test a round-trip**

Run: `./scripts/bump-version.sh 9.9.9 && jq -r .version package.json && jq -r .version src-tauri/tauri.conf.json && grep -m1 '^version' src-tauri/Cargo.toml`
Expected:
```
Bumped to 9.9.9 in package.json, tauri.conf.json, Cargo.toml
9.9.9
9.9.9
version = "9.9.9"
```

Revert: `./scripts/bump-version.sh 0.2.0`

- [ ] **Step 5: Commit**

```bash
git add scripts/bump-version.sh
git commit -m "chore(scripts): atomic version bump across package.json, tauri.conf, Cargo.toml"
```

---

## Task 17: `scripts/extract-release-notes.sh` + `scripts/generate-manifest.sh`

**Files:**
- Create: `scripts/extract-release-notes.sh`
- Create: `scripts/generate-manifest.sh`

- [ ] **Step 1: `extract-release-notes.sh`**

Create `scripts/extract-release-notes.sh`:

```bash
#!/usr/bin/env bash
# Extract the body (without heading) of a CHANGELOG section.
# Usage: ./scripts/extract-release-notes.sh 0.2.0
# Prints to stdout. Exit non-zero if no such section exists.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version>" >&2
  exit 64
fi

VER="$1"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CHANGELOG="$ROOT/CHANGELOG.md"

if [[ ! -f "$CHANGELOG" ]]; then
  echo "CHANGELOG.md not found at $CHANGELOG" >&2
  exit 66
fi

# Emit lines between "## [VER]" and the next "## [" heading (exclusive).
awk -v ver="$VER" '
  BEGIN { found = 0; in_section = 0 }
  /^## \[/ {
    if (in_section) { exit }
    if ($0 ~ "^## \\[" ver "\\]") { found = 1; in_section = 1; next }
  }
  in_section { print }
  END { if (!found) exit 1 }
' "$CHANGELOG" | sed -e '/./,$!d' | awk 'NR>0 { buf = buf $0 "\n" } END { printf "%s", buf }'
```

- [ ] **Step 2: `generate-manifest.sh`**

Create `scripts/generate-manifest.sh`:

```bash
#!/usr/bin/env bash
# Build dist/release/latest.json for the updater.
# Usage: ./scripts/generate-manifest.sh <version> <signature>
# Writes to dist/release/latest.json.
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "Usage: $0 <version> <signature>" >&2
  exit 64
fi

VER="$1"
SIG="$2"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$ROOT/dist/release"
mkdir -p "$OUT_DIR"
OUT="$OUT_DIR/latest.json"

NOTES="$("$ROOT/scripts/extract-release-notes.sh" "$VER")"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
URL="https://github.com/NewTurn2017/hearth/releases/latest/download/Hearth.app.tar.gz"

jq -n \
  --arg version "$VER" \
  --arg notes   "$NOTES" \
  --arg pub     "$PUB_DATE" \
  --arg sig     "$SIG" \
  --arg url     "$URL" \
  '{
    version: $version,
    notes:   $notes,
    pub_date: $pub,
    platforms: {
      "darwin-aarch64": { signature: $sig, url: $url },
      "darwin-x86_64":  { signature: $sig, url: $url }
    }
  }' > "$OUT"

echo "Wrote $OUT"
```

- [ ] **Step 3: Make executable + test**

Run:
```bash
chmod +x scripts/extract-release-notes.sh scripts/generate-manifest.sh
./scripts/extract-release-notes.sh 0.2.0 | head -5
```
Expected: the first few lines of the `## [0.2.0] - 2026-04-18` section body (starting with "First public release.").

Run:
```bash
./scripts/generate-manifest.sh 0.2.0 "dGVzdC1zaWctdGVzdC1zaWc="
jq . dist/release/latest.json
```
Expected: JSON with `.version` = `"0.2.0"`, both platform keys present, `signature` = the dummy string.

Clean up the dummy manifest: `rm -rf dist/release`.

- [ ] **Step 4: Commit**

```bash
git add scripts/extract-release-notes.sh scripts/generate-manifest.sh
git commit -m "chore(scripts): changelog extraction + updater manifest generator"
```

---

## Task 18: `scripts/release.sh` — preflight only

**Files:**
- Create: `scripts/release.sh`

This task ships the script's skeleton and the full preflight function. Build/sign/notarize/publish stages are added in Tasks 19–21.

- [ ] **Step 1: Write the skeleton**

Create `scripts/release.sh`:

```bash
#!/usr/bin/env bash
# End-to-end macOS release driver for Hearth.
# See docs/superpowers/specs/2026-04-18-tauri-release-design.md for design.
#
# Usage:
#   ./scripts/release.sh               # full release
#   ./scripts/release.sh --dry-run     # stop before tag push + gh release create
#   ./scripts/release.sh --skip-tests  # skip preflight test runs
#   ./scripts/release.sh --verbose     # set -x
#
# Environment comes from .env.release.
set -euo pipefail

DRY_RUN=0
SKIP_TESTS=0
for arg in "$@"; do
  case "$arg" in
    --dry-run)     DRY_RUN=1 ;;
    --skip-tests)  SKIP_TESTS=1 ;;
    --verbose)     set -x ;;
    *) echo "Unknown flag: $arg" >&2; exit 64 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -f .env.release ]]; then
  echo "Missing .env.release (copy .env.release.example and fill in)." >&2
  exit 66
fi
# shellcheck disable=SC1091
set -a; . ./.env.release; set +a

log() { printf '\033[1;36m[release]\033[0m %s\n' "$*"; }
die() { printf '\033[1;31m[release]\033[0m %s\n' "$*" >&2; exit 1; }

preflight() {
  log "Preflight…"

  # Clean git + on main
  [[ -z "$(git status --porcelain)" ]] || die "git working tree not clean."
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
  [[ "$BRANCH" == "main" ]] || die "not on main (current: $BRANCH)."

  # Version sync across 3 manifests
  VER_PKG="$(jq -r .version package.json)"
  VER_TAURI="$(jq -r .version src-tauri/tauri.conf.json)"
  VER_CARGO="$(grep -m1 '^version' src-tauri/Cargo.toml | sed -E 's/.*"(.*)".*/\1/')"
  [[ "$VER_PKG" == "$VER_TAURI" && "$VER_PKG" == "$VER_CARGO" ]] \
    || die "version drift: package=$VER_PKG tauri=$VER_TAURI cargo=$VER_CARGO"
  VERSION="$VER_PKG"
  TAG="v$VERSION"
  export VERSION TAG

  # Tag must not exist upstream
  if git ls-remote --tags origin "refs/tags/$TAG" | grep -q .; then
    die "tag $TAG already exists on origin."
  fi

  # CHANGELOG section exists
  grep -q "^## \[$VERSION\]" CHANGELOG.md \
    || die "CHANGELOG.md has no '## [$VERSION]' section."

  # .env.release fields
  for v in APPLE_API_KEY_PATH APPLE_API_KEY_ID APPLE_API_ISSUER \
           TAURI_SIGNING_PRIVATE_KEY TAURI_SIGNING_PRIVATE_KEY_PASSWORD GH_REPO; do
    [[ -n "${!v:-}" ]] || die ".env.release missing $v"
  done
  [[ -f "$APPLE_API_KEY_PATH" ]] || die "APPLE_API_KEY_PATH not a file: $APPLE_API_KEY_PATH"
  [[ -f "$TAURI_SIGNING_PRIVATE_KEY" ]] \
    || die "TAURI_SIGNING_PRIVATE_KEY not a file: $TAURI_SIGNING_PRIVATE_KEY"

  # Tooling
  command -v gh >/dev/null         || die "gh CLI not installed."
  gh auth status >/dev/null        || die "gh not authenticated."
  command -v jq >/dev/null         || die "jq not installed."
  xcrun --find notarytool >/dev/null || die "xcrun notarytool missing; install Xcode CLT."
  xcrun --find stapler    >/dev/null || die "xcrun stapler missing; install Xcode CLT."
  security find-identity -v -p codesigning | grep -q "Developer ID Application: jaehyun jang (2UANJX7ATM)" \
    || die "Developer ID signing identity not in keychain."
  rustup target list --installed | grep -q aarch64-apple-darwin \
    || die "rustup target aarch64-apple-darwin not installed."
  rustup target list --installed | grep -q x86_64-apple-darwin \
    || die "rustup target x86_64-apple-darwin not installed."

  # Tests
  if [[ "$SKIP_TESTS" -eq 0 ]]; then
    log "cargo test…"
    (cd src-tauri && cargo test --quiet)
    log "npm test…"
    npm test --silent
  else
    log "skipping tests (--skip-tests)"
  fi

  log "Preflight OK — version=$VERSION tag=$TAG"
}

main() {
  preflight
  log "(build/sign/notarize/publish stages are added in subsequent tasks)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: stopping before tag/release."
  fi
}

main "$@"
```

- [ ] **Step 2: Make executable**

Run: `chmod +x scripts/release.sh`

- [ ] **Step 3: Dry-run the preflight**

Ensure you have a populated `.env.release` (copy from `.env.release.example` and fill `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Then:

```bash
./scripts/release.sh --dry-run
```

Expected final lines:
```
[release] Preflight OK — version=0.2.0 tag=v0.2.0
[release] (build/sign/notarize/publish stages are added in subsequent tasks)
[release] dry-run: stopping before tag/release.
```

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "chore(scripts): release.sh skeleton + preflight"
```

---

## Task 19: `release.sh` — build + codesign verify

**Files:**
- Modify: `scripts/release.sh` (extend `main`)

- [ ] **Step 1: Add the build function**

In `scripts/release.sh`, **before** the `main()` definition, add:

```bash
build_and_verify() {
  log "npm ci…"
  npm ci --silent

  log "cargo fetch…"
  (cd src-tauri && cargo fetch --quiet)

  log "tauri build (universal-apple-darwin)…"
  npm run tauri -- build --target universal-apple-darwin

  DMG_DIR="src-tauri/target/universal-apple-darwin/release/bundle/dmg"
  MACOS_DIR="src-tauri/target/universal-apple-darwin/release/bundle/macos"
  DMG="$(ls "$DMG_DIR"/Hearth_*_universal.dmg 2>/dev/null | head -1)"
  APP="$MACOS_DIR/Hearth.app"
  TARBALL="$MACOS_DIR/Hearth.app.tar.gz"
  SIG_FILE="$MACOS_DIR/Hearth.app.tar.gz.sig"
  [[ -f "$DMG" ]]     || die "DMG not produced: $DMG_DIR"
  [[ -d "$APP" ]]     || die "Hearth.app not produced: $APP"
  [[ -f "$TARBALL" ]] || die "updater tarball not produced: $TARBALL"
  [[ -f "$SIG_FILE" ]] || die "updater signature not produced: $SIG_FILE"
  export DMG APP TARBALL SIG_FILE

  log "codesign --verify…"
  codesign --verify --deep --strict --verbose=2 "$APP" \
    || die "codesign verify failed on $APP"

  log "Built: $DMG"
}
```

- [ ] **Step 2: Wire into `main`**

Replace the current `main` body:

```bash
main() {
  preflight
  log "(build/sign/notarize/publish stages are added in subsequent tasks)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: stopping before tag/release."
  fi
}
```

with:

```bash
main() {
  preflight
  build_and_verify
  log "(notarize/staple/publish stages added in subsequent tasks)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: stopping before notarize."
  fi
}
```

- [ ] **Step 3: Dry-run the build**

Run: `./scripts/release.sh --dry-run`
Expected (after several minutes for `cargo` + `tauri build`):
```
[release] Built: src-tauri/target/universal-apple-darwin/release/bundle/dmg/Hearth_0.2.0_universal.dmg
[release] (notarize/staple/publish stages added in subsequent tasks)
[release] dry-run: stopping before notarize.
```

And confirm the DMG and `.app.tar.gz` exist:
```bash
ls src-tauri/target/universal-apple-darwin/release/bundle/dmg/
ls src-tauri/target/universal-apple-darwin/release/bundle/macos/
```

If `tauri build` fails with a codesign error, check:
- `security find-identity -v -p codesigning` shows the Developer ID identity
- `tauri.conf.json > bundle.macOS.signingIdentity` exactly matches the keychain CN
- entitlements.plist is readable

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "chore(scripts): release.sh build + codesign verify stage"
```

---

## Task 20: `release.sh` — notarize + staple + updater signature

**Files:**
- Modify: `scripts/release.sh`

- [ ] **Step 1: Add notarize function**

Append, before `main`:

```bash
notarize_and_staple() {
  log "notarytool submit (may take 2–10 min)…"
  SUBMIT_JSON="$(xcrun notarytool submit "$DMG" \
    --key "$APPLE_API_KEY_PATH" \
    --key-id "$APPLE_API_KEY_ID" \
    --issuer "$APPLE_API_ISSUER" \
    --wait \
    --output-format json)"
  echo "$SUBMIT_JSON" | jq .
  SUBMIT_ID="$(echo "$SUBMIT_JSON" | jq -r .id)"
  STATUS="$(echo "$SUBMIT_JSON" | jq -r .status)"
  if [[ "$STATUS" != "Accepted" ]]; then
    log "notarization failed ($STATUS). Fetching log…"
    xcrun notarytool log "$SUBMIT_ID" \
      --key "$APPLE_API_KEY_PATH" \
      --key-id "$APPLE_API_KEY_ID" \
      --issuer "$APPLE_API_ISSUER" || true
    die "notarization not Accepted."
  fi
  export SUBMIT_ID

  log "stapler staple DMG…"
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"

  log "stapler staple .app (updater tarball must carry the staple)…"
  xcrun stapler staple "$APP"
  xcrun stapler validate "$APP"

  log "Repacking Hearth.app.tar.gz with stapled .app…"
  (cd "$(dirname "$APP")" && tar -czf Hearth.app.tar.gz Hearth.app)

  log "Signing updater tarball with Tauri Ed25519 key…"
  TAURI_SIGNING_PRIVATE_KEY="$TAURI_SIGNING_PRIVATE_KEY" \
  TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" \
  npx tauri signer sign "$TARBALL" > /dev/null

  # tauri signer writes <file>.sig next to the input; capture the content.
  SIGNATURE="$(cat "$SIG_FILE")"
  [[ -n "$SIGNATURE" ]] || die "updater signature empty."
  export SIGNATURE
  log "Updater signature ready (len=${#SIGNATURE})."
}
```

- [ ] **Step 2: Wire into `main`**

Update `main`:

```bash
main() {
  preflight
  build_and_verify
  notarize_and_staple
  log "(manifest + gh release + post-verify added in the next task)"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: stopping before tag/release."
  fi
}
```

- [ ] **Step 3: Dry-run notarize**

**⚠️ Warning:** this step submits a real notarization request to Apple. It's non-destructive (Apple stores the submission either way), but it's visible in Apple's audit logs. Only run when ready.

Run: `./scripts/release.sh --dry-run`
Expected (~5–15 min, most time spent awaiting Apple):
```
[release] stapler staple .app (updater tarball must carry the staple)…
The staple and validate action worked!
[release] Repacking Hearth.app.tar.gz with stapled .app…
[release] Signing updater tarball with Tauri Ed25519 key…
[release] Updater signature ready (len=…)
[release] (manifest + gh release + post-verify added in the next task)
[release] dry-run: stopping before tag/release.
```

Validate manually:
```bash
xcrun stapler validate src-tauri/target/universal-apple-darwin/release/bundle/dmg/Hearth_0.2.0_universal.dmg
spctl --assess --type execute --verbose src-tauri/target/universal-apple-darwin/release/bundle/macos/Hearth.app
```
Expected: "The validate action worked!" and `accepted / source=Notarized Developer ID`.

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "chore(scripts): release.sh notarize + staple + updater signature"
```

---

## Task 21: `release.sh` — manifest + gh release + post-verify

**Files:**
- Modify: `scripts/release.sh`

- [ ] **Step 1: Add final functions**

Append, before `main`:

```bash
write_manifest_and_notes() {
  log "Writing dist/release/latest.json…"
  "$ROOT/scripts/generate-manifest.sh" "$VERSION" "$SIGNATURE"

  log "Extracting release notes…"
  mkdir -p dist/release
  "$ROOT/scripts/extract-release-notes.sh" "$VERSION" > dist/release/notes.md

  # Append user-facing install/update footer.
  cat >> dist/release/notes.md <<'EOF'

---

**설치 (macOS)**

`Hearth_*_universal.dmg` 를 받아서 Applications 로 드래그하세요. 첫 실행만 우클릭 → "열기" 로 Gatekeeper 를 1회 통과하면 됩니다 (공증된 빌드라 "알 수 없는 개발자" 경고는 없습니다).

**업데이트**

앱이 켜져 있으면 자동으로 새 버전을 확인합니다. 업데이트 토스트에서 "지금 재시작" 을 누르면 2–3초 내에 새 버전으로 교체됩니다.
EOF
}

publish_and_verify() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    log "dry-run: skipping git tag + gh release create."
    log "artifacts prepared:"
    ls -lh "$DMG" "$TARBALL" "$SIG_FILE" dist/release/latest.json dist/release/notes.md
    return 0
  fi

  log "Creating signed git tag $TAG…"
  git tag -s "$TAG" -m "Hearth $VERSION"
  git push origin "$TAG"

  log "gh release create…"
  gh release create "$TAG" \
    --repo "$GH_REPO" \
    --title "Hearth $VERSION" \
    --notes-file dist/release/notes.md \
    "$DMG" \
    "$TARBALL" \
    "$SIG_FILE" \
    "dist/release/latest.json"

  log "Post-verify: latest.json version round-trip…"
  REMOTE_VER="$(curl -sL "https://github.com/$GH_REPO/releases/latest/download/latest.json" | jq -r .version)"
  [[ "$REMOTE_VER" == "$VERSION" ]] \
    || die "permalink mismatch: expected $VERSION got $REMOTE_VER"

  log "Release published:"
  log "  https://github.com/$GH_REPO/releases/tag/$TAG"
}
```

- [ ] **Step 2: Finalize `main`**

Replace `main` with:

```bash
main() {
  preflight
  build_and_verify
  notarize_and_staple
  write_manifest_and_notes
  publish_and_verify

  log "Done."
  log "  version:    $VERSION"
  log "  tag:        $TAG"
  log "  dmg:        $DMG"
  log "  notarize:   ${SUBMIT_ID:-(n/a)}"
  if [[ "$DRY_RUN" -eq 0 ]]; then
    log "  release:    https://github.com/$GH_REPO/releases/tag/$TAG"
    log "  manifest:   https://github.com/$GH_REPO/releases/latest/download/latest.json"
  fi
}
```

- [ ] **Step 3: Dry-run the whole thing**

Run: `./scripts/release.sh --dry-run`
Expected:
- Builds, notarizes, staples, writes `dist/release/latest.json` + `dist/release/notes.md`
- Final block lists artifacts without publishing
- No git tag was pushed (`git tag -l` shows no `v0.2.0`)
- No GitHub release was created (`gh release list --repo NewTurn2017/hearth` is empty)

Inspect manifest:
```bash
jq . dist/release/latest.json
head -40 dist/release/notes.md
```

- [ ] **Step 4: Commit**

```bash
git add scripts/release.sh
git commit -m "chore(scripts): release.sh manifest + gh release + post-verify"
```

---

## Task 22: Tier 3 updater round-trip dry-run

This is a **manual** ceremony; no code changes, but it must pass before the real v0.2.0 release. Follow `docs/releasing.md`'s "Tier 3" section.

**Files:** none modified (work happens on a throwaway branch that is discarded).

- [ ] **Step 1: Create a throwaway branch**

```bash
git checkout -b tmp/updater-dryrun
```

- [ ] **Step 2: Build a fake `0.1.9.app`**

```bash
./scripts/bump-version.sh 0.1.9
npm run tauri -- build --target universal-apple-darwin
```

(No signing on this fake is fine — the goal is only to host the updater binding.)

- [ ] **Step 3: Stand up a private test manifest**

Create `/tmp/fake-latest.json` that advertises a v0.2.0 update pointing at the v0.2.0 tarball you built during Task 20's dry-run:

```json
{
  "version": "0.2.0",
  "notes": "test",
  "pub_date": "2026-04-18T12:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<paste signature from Hearth.app.tar.gz.sig>",
      "url": "file:///Users/genie/dev/tools/hearth/src-tauri/target/universal-apple-darwin/release/bundle/macos/Hearth.app.tar.gz"
    },
    "darwin-x86_64": {
      "signature": "<same>",
      "url": "<same>"
    }
  }
}
```

Host it locally: `python3 -m http.server 8765 --directory /tmp` in another terminal.

- [ ] **Step 4: Temporarily point the fake build at the test manifest**

Edit `src-tauri/tauri.conf.json` locally:
- `plugins.updater.endpoints = ["http://127.0.0.1:8765/fake-latest.json"]`

Rebuild: `npm run tauri -- build --target universal-apple-darwin`

- [ ] **Step 5: Run the fake app**

Copy the produced `.app` bundle into a guest user's `~/Applications/` (to reproduce a first-run Gatekeeper path as closely as practical; this is a dev build so Gatekeeper will warn — that's OK, the point is only the updater flow).

Launch. Wait ~30 s. **Expected:** toast "새 버전 0.2.0 준비됨" with two buttons.

- [ ] **Step 6: Verify "지금 재시작" works**

Click **지금 재시작**. **Expected:** within 2–3 seconds the app relaunches; window title is "Hearth"; SQLite DB (any test data created while fake-0.1.9 was running) persists.

- [ ] **Step 7: Verify "나중에" works**

From a fresh launch, click **나중에**. Quit + relaunch. Wait ~30 s. **Expected:** toast does **not** re-appear for the same version.

Inspect: `defaults read -g` or the app's localStorage via DevTools — `updater.dismissedVersion` should be `0.2.0`.

- [ ] **Step 8: Discard the throwaway branch**

```bash
kill %1                 # stop the python http.server
git checkout main
git branch -D tmp/updater-dryrun
```

Revert any stray uncommitted changes in `src-tauri/tauri.conf.json` to make sure `endpoints` points at the real permalink and `version` is `0.2.0`:

```bash
git status
git checkout -- src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
jq -r .version package.json
jq -r '.plugins.updater.endpoints[0]' src-tauri/tauri.conf.json
```

Expected:
```
0.2.0
https://github.com/NewTurn2017/hearth/releases/latest/download/latest.json
```

**No commit.** This task produces no repository artifacts.

---

## Task 23: Cut v0.2.0 (real release)

This is the real, irreversible release. Preconditions:
- All prior tasks complete and committed on `main`.
- Tier 3 dry-run (Task 22) passed.
- `docs/releasing.md` open for reference.

**Files:** none modified (the release is a tag + GH Release, not a source edit).

- [ ] **Step 1: Final confirmation**

```bash
git log --oneline main ^HEAD~10   # sanity check recent history
git status                        # clean
jq -r .version package.json       # 0.2.0
gh release list --repo NewTurn2017/hearth   # empty
```

- [ ] **Step 2: Run the release script**

```bash
./scripts/release.sh
```

Expected elapsed: 10–20 min. The script streams notarization status. On success, the last output includes `release: https://github.com/NewTurn2017/hearth/releases/tag/v0.2.0`.

- [ ] **Step 3: Verify the release**

```bash
gh release view v0.2.0 --repo NewTurn2017/hearth
```

Expected fields:
- Title `Hearth 0.2.0`
- 4 assets: `Hearth_0.2.0_universal.dmg`, `Hearth.app.tar.gz`, `Hearth.app.tar.gz.sig`, `latest.json`
- Body contains the CHANGELOG section + install/update footer

```bash
curl -sL https://github.com/NewTurn2017/hearth/releases/latest/download/latest.json | jq .
```

Expected: JSON with `version: "0.2.0"`, non-empty `signature`, both `darwin-aarch64` and `darwin-x86_64` entries.

- [ ] **Step 4: No commit**

The tag + release **are** the artifact. There is no post-release commit required.

---

## Task 24: Tier 2 smoke test

Exercise the released v0.2.0 end-to-end on a freshly-installed environment. Follow `docs/releasing.md`'s "Tier 2" section verbatim.

**Files:** none modified.

- [ ] **Step 1: Log into a macOS guest or secondary user account**

(The goal is to approximate a first-time user's Mac where Hearth has never run and Gatekeeper has never seen it.)

- [ ] **Step 2: Download DMG**

Open `https://github.com/NewTurn2017/hearth/releases/latest` in Safari. Download `Hearth_0.2.0_universal.dmg`.

- [ ] **Step 3: Install**

Double-click the DMG. Drag Hearth.app to the Applications alias. Eject DMG.

- [ ] **Step 4: First launch**

Finder → Applications → right-click `Hearth.app` → **Open**.

**Expected:** macOS may show "Hearth was downloaded from the Internet..." once, offering **Open**. Click Open.

**Not expected:** an "unknown developer" warning (if it appears, notarization failed to staple — yank + roll forward).

- [ ] **Step 5: Run through the smoke checklist**

- [ ] Main window renders (sidebar, Projects/Calendar/Memos tabs)
- [ ] `⌘K` palette opens
- [ ] Create a project → quit → relaunch → project persists
- [ ] Settings (🛠 icon) → AI tab → paste an OpenAI key → `⌘K` → type `프로젝트 목록 보여줘` → AI responds
- [ ] Settings → AI tab → provider `local` + MLX running locally on port 18080 → `⌘K` → same prompt → responds
- [ ] Settings → 백업 tab → "지금 백업" → a `.db` file appears in the configured backup folder
- [ ] `~/Library/Application Support/com.newturn2017.hearth/data.db` exists (via Terminal → `ls ~/Library/Application\ Support/com.newturn2017.hearth/`)

- [ ] **Step 6: If anything failed**

Yank immediately:

```bash
gh release delete v0.2.0 --yes --repo NewTurn2017/hearth
git push --delete origin v0.2.0
```

Fix the issue on a branch, re-bump to `0.2.1`, and re-run Tasks 23–24. Do **not** rescue a broken v0.2.0 in place (users who already downloaded the broken DMG are stuck until they manually reinstall).

- [ ] **Step 7: If everything passed**

Announce the release however you like (README already advertises the latest release badge). Add a short "release-0.2.0 monitoring" reminder for yourself — watch `gh issue list --repo NewTurn2017/hearth --label release-0.2.0` for 7 days.

---

## Self-Review (done after writing the plan)

### Spec coverage audit

Run through every Spec A section and point to the implementing task:

| Spec section | Implementing task(s) |
|--------------|---------------------|
| Metadata & Manifest Cleanup — Cargo.toml rename + metadata + plugin deps | 1, 2, 3 |
| Metadata & Manifest Cleanup — tauri.conf.json (version, CSP, bundle, macOS, updater) | 6, 7 |
| Metadata & Manifest Cleanup — package.json | 4 |
| Metadata & Manifest Cleanup — entitlements.plist | 5 |
| Metadata & Manifest Cleanup — CHANGELOG + README | 12, 13 |
| macOS Signing & Notarization — signing identity in config | 6 |
| macOS Signing & Notarization — universal-apple-darwin target | 19 (build), prereqs |
| macOS Signing & Notarization — signing flow (tauri build + codesign verify) | 19 |
| macOS Signing & Notarization — notarization (notarytool API key) | 20 |
| macOS Signing & Notarization — stapling | 20 |
| macOS Signing & Notarization — credential storage principles | 15 (.env.release.example), Task 7 step 4 (offline backup) |
| Auto-updater Architecture — Ed25519 keypair + pubkey embed | 7 |
| Auto-updater Architecture — Rust plugin wiring | 3 (deps), 8 (plugin registration + capabilities) |
| Auto-updater Architecture — frontend useAppUpdater | 9 (Toast extension), 10 (hook), 11 (wire) |
| Auto-updater Architecture — latest.json manifest | 17 (generator), 20 (signature), 21 (write) |
| Auto-updater Architecture — endpoint permalink | 6 (config) |
| Auto-updater Architecture — failure modes | 10 (hook tests cover offline + dismiss), 11 (smoke) |
| Release Script — preflight | 18 |
| Release Script — execution stages | 19, 20, 21 |
| Release Script — modes (--dry-run, --skip-tests, --verbose) | 18 |
| Release Script — idempotency | 21 (publish_and_verify refuses existing tag implicitly via `gh`; Task 18 checks tag absence) |
| Release Script — final output | 21 |
| GitHub Releases Layout — tag/release | 21 |
| GitHub Releases Layout — assets | 21 |
| GitHub Releases Layout — notes body + footer | 21 |
| GitHub Releases Layout — permalinks | 6 (config), 21 (post-verify) |
| Testing — Tier 1 (scripted verifies) | 18 (preflight cargo/npm test), 19 (codesign verify), 20 (stapler validate) |
| Testing — Tier 2 (public smoke) | 24 |
| Testing — Tier 3 (updater round-trip) | 22 |
| Testing — antitests | partial: Task 10 covers offline + version mismatch dismissal; signature tampering is implicitly covered by Tauri (plugin-level) and by Task 22 step 6 (mutating a byte would reject the update). Not separately executed. |
| Rollout — implementation order | matches Tasks 1–24 |
| Rollout — Release Day checklist | 14 (docs/releasing.md), 23 (execution) |
| Rollout — Rollback strategy | 14 (releasing.md), 24 step 6 (yank procedure) |
| Rollout — Post-release monitoring | 24 step 7 |
| Rollout — User communication | 13 (README badge + install footer) |

No gaps identified.

### Placeholder scan

- `REPLACE_WITH_PUBKEY_IN_TASK_7` — deliberate, resolved in Task 7 Step 2. Lives through exactly one commit (Task 6 writes it, Task 7 replaces it).
- `<paste signature from Hearth.app.tar.gz.sig>` / `<same>` in Task 22 Step 3 — intentional human substitution during a manual dry-run. Not a plan gap.
- No other TBDs / TODOs / FIXMEs.

### Type consistency

- `ToastApi.info(message, { sticky?, actions? })` — introduced in Task 9, consumed in Tasks 10 and 11. Matches.
- `useAppUpdater()` — no args, no return; mounted in Task 11 exactly as declared in Task 10.
- `update.downloadAndInstall()` / `update.version` / `update.available` — match `@tauri-apps/plugin-updater` public API at v2.
- `relaunch()` — from `@tauri-apps/plugin-process`, matches capabilities `process:allow-restart`.
- Env var names `APPLE_API_KEY_PATH` / `_KEY_ID` / `_ISSUER` / `TAURI_SIGNING_PRIVATE_KEY` / `_PASSWORD` / `GH_REPO` — identical across `.env.release.example` (Task 15), `release.sh` preflight (Task 18), `notarize_and_staple` (Task 20), and `publish_and_verify` (Task 21). ✓
- `$DMG`, `$APP`, `$TARBALL`, `$SIG_FILE`, `$VERSION`, `$TAG`, `$SIGNATURE`, `$SUBMIT_ID` — exported in the stage that produces them, consumed by later stages; no renames.

No consistency issues found.
