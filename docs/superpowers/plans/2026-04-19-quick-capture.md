# Quick Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a global-shortcut triggered one-line memo overlay (Quick Capture) so users can brain-dump into Hearth without switching to the app first.

**Architecture:** New Rust module `cmd_quick_capture` registers a Tauri global shortcut (default `CommandOrControl+Shift+H`, configurable via settings KV `shortcut.quick_capture`). A second Tauri window (label `quick-capture`) is pre-built hidden and toggled by the shortcut; its frontend is a minimal route served from the same bundle that invokes the existing `create_memo` command and hides itself on submit or blur. The main window listens for a `memo:quick-captured` event to emit a toast (reusing `memo:focus` for click-to-scroll).

**Tech Stack:** Tauri v2, `tauri-plugin-global-shortcut@2`, `@tauri-apps/plugin-global-shortcut`, React 19, Vitest, Rust `rusqlite`.

**Reference spec:** `docs/superpowers/specs/2026-04-19-quick-capture-design.md`

---

## File Structure

**Create:**
- `src-tauri/src/cmd_quick_capture.rs` — shortcut KV, accelerator normalization, window show/hide/toggle, tauri commands.
- `src/windows/QuickCapture.tsx` — overlay window component.
- `src/components/settings/ShortcutRecorder.tsx` — key combo capture primitive.
- `src/hooks/useQuickCaptureShortcut.ts` — reactive hook over the settings value.
- `src/__tests__/QuickCapture.test.tsx`
- `src/components/settings/__tests__/ShortcutRecorder.test.tsx`
- `src-tauri/src/cmd_quick_capture_tests.rs` (test module inside `cmd_quick_capture.rs` via `#[cfg(test)]`)

**Modify:**
- `src-tauri/Cargo.toml` — add `tauri-plugin-global-shortcut`.
- `src-tauri/capabilities/default.json` — add `global-shortcut:default` (or specific allows).
- `src-tauri/src/lib.rs` — register module, plugin, invoke handlers, setup-hook shortcut registration + window prebuild.
- `package.json` — add `@tauri-apps/plugin-global-shortcut`.
- `src/main.tsx` — branch render on `?window=quick-capture`.
- `src/api.ts` — bindings for new commands.
- `src/components/SettingsGeneralSection.tsx` — add Quick Capture section.
- `src/components/Layout.tsx` — listen for `memo:quick-captured`, show toast, forward click to `memo:focus`.
- `src-tauri/tauri.conf.json` — version bump to `0.5.0` (also `Cargo.toml`, `package.json`).
- `CHANGELOG.md` — `[0.5.0]` entry.
- `README.md` — Features list update.

---

## Task 1: Add dependencies and capability

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Add Rust plugin crate**

Edit `src-tauri/Cargo.toml`. In `[dependencies]`, add after `tauri-plugin-notification`:

```toml
tauri-plugin-global-shortcut = "2"
```

- [ ] **Step 2: Add JS plugin package**

```bash
cd /Users/genie/dev/tools/hearth && npm install @tauri-apps/plugin-global-shortcut@^2
```

Expected: `package.json` lists `@tauri-apps/plugin-global-shortcut` under dependencies, npm reports no errors.

- [ ] **Step 3: Grant capability**

Read current capability file first to follow existing formatting. Add these permission strings to the `permissions` array of the `default` capability (file: `src-tauri/capabilities/default.json`):

```json
"global-shortcut:allow-register",
"global-shortcut:allow-unregister",
"global-shortcut:allow-is-registered"
```

If the file does not contain a `default` capability at that path, search for the equivalently-named file under `src-tauri/capabilities/` (e.g. `main.json`) and add them there instead.

- [ ] **Step 4: Verify build wires**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```

Expected: compiles clean. Warnings about unused deps are OK — they go away once Task 5 wires the plugin.

- [ ] **Step 5: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json src-tauri/capabilities/
git commit -m "chore(deps): add tauri-plugin-global-shortcut for Quick Capture"
```

---

## Task 2: Accelerator normalization helper (TDD)

**Files:**
- Create: `src-tauri/src/cmd_quick_capture.rs`
- Modify: `src-tauri/src/lib.rs` (module declaration only — no wiring yet)

- [ ] **Step 1: Create module skeleton and register**

Create `src-tauri/src/cmd_quick_capture.rs`:

```rust
//! Quick Capture — global shortcut and hidden overlay window.
//!
//! The shortcut combo is persisted in the `settings` KV under
//! `shortcut.quick_capture`. The UI sends combos in display form (e.g.
//! "Cmd+Shift+H"); we normalize to Tauri's accelerator format
//! ("CommandOrControl+Shift+H") before register.

pub(crate) const K_SHORTCUT: &str = "shortcut.quick_capture";
pub(crate) const K_SHORTCUT_LAST_ERROR: &str = "shortcut.quick_capture.last_error";
pub(crate) const DEFAULT_COMBO: &str = "CommandOrControl+Shift+H";
pub(crate) const WINDOW_LABEL: &str = "quick-capture";

/// Normalize a user-facing combo string to Tauri's accelerator format.
/// Accepts "Cmd+Shift+H", "Ctrl+Shift+H", "CommandOrControl+Shift+H".
/// Returns Err if no base (non-modifier) key is present or if an unknown
/// token appears.
pub fn normalize_accelerator(input: &str) -> Result<String, String> {
    let mut mods: Vec<String> = Vec::new();
    let mut base: Option<String> = None;
    for raw in input.split('+') {
        let tok = raw.trim();
        if tok.is_empty() {
            return Err(format!("Invalid accelerator: empty token in {input:?}"));
        }
        match tok.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "meta" | "super" | "commandorcontrol"
            | "ctrl" | "control" => push_mod(&mut mods, "CommandOrControl"),
            "alt" | "option" | "opt" => push_mod(&mut mods, "Alt"),
            "shift" => push_mod(&mut mods, "Shift"),
            _ => {
                if base.is_some() {
                    return Err(format!("Invalid accelerator: multiple base keys in {input:?}"));
                }
                base = Some(canonical_base(tok)?);
            }
        }
    }
    let base = base.ok_or_else(|| format!("Invalid accelerator: no base key in {input:?}"))?;
    let mut parts = mods;
    parts.push(base);
    Ok(parts.join("+"))
}

fn push_mod(mods: &mut Vec<String>, name: &str) {
    let owned = name.to_string();
    if !mods.iter().any(|m| m == &owned) {
        mods.push(owned);
    }
}

fn canonical_base(tok: &str) -> Result<String, String> {
    let t = tok.trim();
    // Letters: upper-case single char A-Z.
    if t.len() == 1 {
        let c = t.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Ok(c.to_ascii_uppercase().to_string());
        }
        if c.is_ascii_digit() {
            return Ok(t.to_string());
        }
    }
    // Function keys F1..F24, or named keys passed through.
    let upper = t.to_ascii_uppercase();
    if upper.starts_with('F') && upper[1..].chars().all(|c| c.is_ascii_digit()) {
        return Ok(upper);
    }
    match upper.as_str() {
        "SPACE" | "ENTER" | "ESCAPE" | "TAB" | "BACKSPACE" | "DELETE"
        | "ARROWUP" | "ARROWDOWN" | "ARROWLEFT" | "ARROWRIGHT" => Ok(upper),
        _ => Err(format!("Invalid accelerator: unsupported key {t:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_cmd_shift_letter() {
        assert_eq!(
            normalize_accelerator("Cmd+Shift+H").unwrap(),
            "CommandOrControl+Shift+H"
        );
        assert_eq!(
            normalize_accelerator("ctrl+shift+h").unwrap(),
            "CommandOrControl+Shift+H"
        );
        assert_eq!(
            normalize_accelerator("CommandOrControl+Shift+H").unwrap(),
            "CommandOrControl+Shift+H"
        );
    }

    #[test]
    fn normalize_duplicates_collapse() {
        assert_eq!(
            normalize_accelerator("Cmd+Ctrl+H").unwrap(),
            "CommandOrControl+H"
        );
    }

    #[test]
    fn normalize_function_key() {
        assert_eq!(normalize_accelerator("Alt+F5").unwrap(), "Alt+F5");
    }

    #[test]
    fn normalize_rejects_no_base() {
        assert!(normalize_accelerator("Cmd+Shift").is_err());
    }

    #[test]
    fn normalize_rejects_unknown_key() {
        assert!(normalize_accelerator("Cmd+Weird").is_err());
    }

    #[test]
    fn normalize_rejects_empty_token() {
        assert!(normalize_accelerator("Cmd++H").is_err());
    }
}
```

Edit `src-tauri/src/lib.rs`, add `mod cmd_quick_capture;` in the module list (alphabetical placement between `cmd_projects` and `cmd_schedules`).

- [ ] **Step 2: Run tests to verify all pass**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test cmd_quick_capture
```

Expected: 6 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/cmd_quick_capture.rs src-tauri/src/lib.rs
git commit -m "feat(quick-capture): accelerator normalization"
```

---

## Task 3: Settings accessors and tauri commands (no wiring yet)

**Files:**
- Modify: `src-tauri/src/cmd_quick_capture.rs`

- [ ] **Step 1: Add settings read/write helpers and commands**

Append to `src-tauri/src/cmd_quick_capture.rs`:

```rust
use crate::cmd_settings;
use crate::AppState;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// Read the saved combo, falling back to the default. Always returns a value
/// already normalized to Tauri accelerator form.
pub(crate) fn read_combo(db: &rusqlite::Connection) -> Result<String, String> {
    let raw = cmd_settings::read(db, K_SHORTCUT)?;
    let combo = if raw.trim().is_empty() { DEFAULT_COMBO.to_string() } else { raw };
    normalize_accelerator(&combo)
}

#[tauri::command]
pub fn get_quick_capture_shortcut(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    read_combo(&db)
}

#[tauri::command]
pub fn get_quick_capture_shortcut_error(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::read(&db, K_SHORTCUT_LAST_ERROR)
}

#[tauri::command]
pub fn rebind_quick_capture_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    combo: String,
) -> Result<String, String> {
    let normalized = normalize_accelerator(&combo)?;
    // Unregister old, register new; on failure restore old.
    let old = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        read_combo(&db)?
    };
    let gs = app.global_shortcut();
    let old_sc: Shortcut = old.parse().map_err(|e| format!("parse old shortcut: {e}"))?;
    let new_sc: Shortcut = normalized
        .parse()
        .map_err(|e| format!("parse new shortcut: {e}"))?;

    let _ = gs.unregister(old_sc);
    if let Err(e) = gs.register(new_sc) {
        // Try to restore the previous binding so the user isn't stranded.
        let _ = gs.register(old_sc);
        return Err(format!("register shortcut failed: {e}"));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::write(&db, K_SHORTCUT, &normalized)?;
    cmd_settings::write(&db, K_SHORTCUT_LAST_ERROR, "")?;
    drop(db);

    let _ = app.emit("quick-capture-shortcut:changed", &normalized);
    Ok(normalized)
}
```

Make `cmd_settings::read` / `write` visible. They are already `pub(crate)` — confirm before building.

- [ ] **Step 2: Build**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```

Expected: compiles. If `read`/`write` aren't `pub(crate)`, raise visibility minimally in `cmd_settings.rs`.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/cmd_quick_capture.rs src-tauri/src/cmd_settings.rs
git commit -m "feat(quick-capture): settings-backed shortcut rebind command"
```

---

## Task 4: Window show/hide/toggle commands

**Files:**
- Modify: `src-tauri/src/cmd_quick_capture.rs`

- [ ] **Step 1: Append window lifecycle functions**

Append:

```rust
pub(crate) fn ensure_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(w) = app.get_webview_window(WINDOW_LABEL) {
        return Ok(w);
    }
    let url = WebviewUrl::App("index.html?window=quick-capture".into());
    let window = WebviewWindowBuilder::new(app, WINDOW_LABEL, url)
        .title("Quick Capture")
        .inner_size(560.0, 80.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(window)
}

fn position_top_center(w: &tauri::WebviewWindow) -> Result<(), String> {
    // Place at roughly 1/4 down from the top on the primary monitor.
    let monitor = w
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no primary monitor".to_string())?;
    let size = monitor.size();
    let win_size = w.outer_size().map_err(|e| e.to_string())?;
    let x = (size.width as i32 - win_size.width as i32) / 2;
    let y = (size.height as i32 / 4).max(80);
    w.set_position(tauri::PhysicalPosition { x, y })
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn show_quick_capture_window(app: AppHandle) -> Result<(), String> {
    let w = ensure_window(&app)?;
    position_top_center(&w)?;
    w.show().map_err(|e| e.to_string())?;
    w.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn hide_quick_capture_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(WINDOW_LABEL) {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn toggle_quick_capture_window(app: AppHandle) -> Result<(), String> {
    let w = ensure_window(&app)?;
    let visible = w.is_visible().unwrap_or(false);
    if visible {
        w.hide().map_err(|e| e.to_string())
    } else {
        position_top_center(&w)?;
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn resize_quick_capture_window(app: AppHandle, height: u32) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(WINDOW_LABEL) {
        let clamped = height.clamp(80, 200);
        w.set_size(tauri::LogicalSize::new(560.0, clamped as f64))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Build**

```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/cmd_quick_capture.rs
git commit -m "feat(quick-capture): overlay window show/hide/toggle"
```

---

## Task 5: Wire plugin, invoke handlers, and setup-hook registration

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add plugin to builder**

In `src-tauri/src/lib.rs`, insert after `.plugin(tauri_plugin_notification::init())`:

```rust
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
```

- [ ] **Step 2: Add setup-hook registration**

In the `.setup(|app| { ... })` block, at the very end (after the DB-recovered emit), append:

```rust
            // Quick Capture: pre-build the overlay window (hidden) and
            // register the user's global shortcut from settings.
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
                let app_handle = app.handle();
                let _ = crate::cmd_quick_capture::ensure_window(app_handle);

                let combo = {
                    let state: tauri::State<'_, AppState> = app_handle.state();
                    let db = state.db.lock().unwrap();
                    crate::cmd_quick_capture::read_combo(&db).unwrap_or_else(|_| {
                        crate::cmd_quick_capture::DEFAULT_COMBO.to_string()
                    })
                };

                let gs = app_handle.global_shortcut();
                let handle_for_cb = app_handle.clone();
                let register_result = combo
                    .parse::<Shortcut>()
                    .map_err(|e| e.to_string())
                    .and_then(|sc| {
                        gs.on_shortcut(sc, move |_app, _sc, event| {
                            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                                let _ = crate::cmd_quick_capture::toggle_quick_capture_window(
                                    handle_for_cb.clone(),
                                );
                            }
                        })
                        .map_err(|e| e.to_string())
                    });

                let state: tauri::State<'_, AppState> = app_handle.state();
                let db = state.db.lock().unwrap();
                match &register_result {
                    Ok(()) => {
                        let _ = crate::cmd_settings::write(
                            &db,
                            crate::cmd_quick_capture::K_SHORTCUT_LAST_ERROR,
                            "",
                        );
                    }
                    Err(e) => {
                        let _ = crate::cmd_settings::write(
                            &db,
                            crate::cmd_quick_capture::K_SHORTCUT_LAST_ERROR,
                            e,
                        );
                    }
                }
            }
```

(Verify the exact plugin API surface — `on_shortcut` vs `register`+separate listener — matches the installed plugin version. If `on_shortcut` is not available, use `register(sc)` followed by `.shortcut_event(move |event| { ... })` or the plugin's documented callback API. Consult `src-tauri/target/doc` or crate docs.)

- [ ] **Step 3: Register new commands in invoke_handler**

Add inside the `tauri::generate_handler![ ... ]` list (place them together after `cmd_settings::*`):

```rust
            cmd_quick_capture::get_quick_capture_shortcut,
            cmd_quick_capture::get_quick_capture_shortcut_error,
            cmd_quick_capture::rebind_quick_capture_shortcut,
            cmd_quick_capture::show_quick_capture_window,
            cmd_quick_capture::hide_quick_capture_window,
            cmd_quick_capture::toggle_quick_capture_window,
            cmd_quick_capture::resize_quick_capture_window,
```

- [ ] **Step 4: Build and run the app to smoke-test**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

Expected: app starts without crash. Pressing `⌃⇧H` does nothing yet (the frontend route in Task 7 will render the window content). The Rust log shows no errors from shortcut registration. Stop dev (`q` in terminal) after confirming.

- [ ] **Step 5: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/lib.rs
git commit -m "feat(quick-capture): wire plugin and register default shortcut"
```

---

## Task 6: Frontend API bindings

**Files:**
- Modify: `src/api.ts`

- [ ] **Step 1: Append bindings**

At the bottom of `src/api.ts`:

```ts
// Quick Capture
export const getQuickCaptureShortcut = () =>
  invoke<string>("get_quick_capture_shortcut");

export const getQuickCaptureShortcutError = () =>
  invoke<string>("get_quick_capture_shortcut_error");

export const rebindQuickCaptureShortcut = (combo: string) =>
  invoke<string>("rebind_quick_capture_shortcut", { combo });

export const showQuickCaptureWindow = () =>
  invoke<void>("show_quick_capture_window");

export const hideQuickCaptureWindow = () =>
  invoke<void>("hide_quick_capture_window");

export const resizeQuickCaptureWindow = (height: number) =>
  invoke<void>("resize_quick_capture_window", { height });
```

- [ ] **Step 2: Typecheck**

```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/api.ts
git commit -m "feat(quick-capture): frontend api bindings"
```

---

## Task 7: QuickCapture overlay component (TDD) and route split

**Files:**
- Create: `src/windows/QuickCapture.tsx`
- Create: `src/__tests__/QuickCapture.test.tsx`
- Modify: `src/main.tsx`

- [ ] **Step 1: Write failing test**

Create `src/__tests__/QuickCapture.test.tsx`:

```tsx
import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

vi.mock("../api", () => ({
  createMemo: vi.fn().mockResolvedValue({ id: 42 }),
  hideQuickCaptureWindow: vi.fn().mockResolvedValue(undefined),
  resizeQuickCaptureWindow: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn().mockResolvedValue(undefined),
  listen: vi.fn().mockResolvedValue(() => {}),
}));

import { QuickCapture } from "../windows/QuickCapture";
import * as api from "../api";

describe("QuickCapture", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("saves on Enter and hides", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "hello world{Enter}");
    expect(api.createMemo).toHaveBeenCalledWith({
      content: "hello world",
      color: "yellow",
    });
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("Esc closes without save", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "abc");
    await userEvent.keyboard("{Escape}");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("empty Enter is no-op save but hides", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox");
    await userEvent.type(input, "   {Enter}");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(api.hideQuickCaptureWindow).toHaveBeenCalled();
  });

  it("Shift+Enter inserts newline and does not submit", async () => {
    render(<QuickCapture />);
    const input = screen.getByRole("textbox") as HTMLTextAreaElement;
    await userEvent.type(input, "line1{Shift>}{Enter}{/Shift}line2");
    expect(api.createMemo).not.toHaveBeenCalled();
    expect(input.value).toBe("line1\nline2");
  });
});
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cd /Users/genie/dev/tools/hearth && npx vitest run src/__tests__/QuickCapture.test.tsx
```

Expected: FAIL — "Cannot find module '../windows/QuickCapture'".

- [ ] **Step 3: Implement QuickCapture**

Create `src/windows/QuickCapture.tsx`:

```tsx
import { useEffect, useRef, useState } from "react";
import { emit } from "@tauri-apps/api/event";
import {
  createMemo,
  hideQuickCaptureWindow,
  resizeQuickCaptureWindow,
} from "../api";

export function QuickCapture() {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const taRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    taRef.current?.focus();
  }, []);

  useEffect(() => {
    const onBlur = () => {
      if (!busy) void hideQuickCaptureWindow();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [busy]);

  useEffect(() => {
    const el = taRef.current;
    if (!el) return;
    el.style.height = "auto";
    const h = Math.min(Math.max(el.scrollHeight + 16, 80), 200);
    void resizeQuickCaptureWindow(h);
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  async function submit() {
    const content = value.trim();
    if (!content) {
      await hideQuickCaptureWindow();
      return;
    }
    setBusy(true);
    try {
      const memo = await createMemo({ content, color: "yellow" });
      await emit("memo:quick-captured", { memoId: memo.id });
      setValue("");
      await hideQuickCaptureWindow();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      void hideQuickCaptureWindow();
    }
  }

  return (
    <div className="h-full w-full p-3 bg-[var(--color-surface,_#1b1b1b)]/95 backdrop-blur rounded-xl shadow-2xl border border-white/10">
      <textarea
        ref={taRef}
        role="textbox"
        value={value}
        rows={1}
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={onKeyDown}
        placeholder="뇌에 있는 거 한 줄..."
        aria-label="Quick Capture"
        className="w-full resize-none bg-transparent outline-none text-[15px] text-white placeholder-white/40"
      />
      {error && (
        <div className="mt-2 text-xs text-red-400">저장 실패 — {error}</div>
      )}
    </div>
  );
}

export default QuickCapture;
```

Note: `emit` (Tauri global event bus) is used instead of `window.dispatchEvent` because each Tauri window has its own JS `window` object. The main window listens via `listen("memo:quick-captured", ...)` in Task 10.

- [ ] **Step 4: Split main.tsx route**

Replace `src/main.tsx` with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { QuickCapture } from "./windows/QuickCapture";

const params = new URLSearchParams(window.location.search);
const root = ReactDOM.createRoot(
  document.getElementById("root") as HTMLElement
);

if (params.get("window") === "quick-capture") {
  root.render(
    <React.StrictMode>
      <QuickCapture />
    </React.StrictMode>
  );
} else {
  root.render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
}
```

- [ ] **Step 5: Re-run tests to verify pass**

```bash
cd /Users/genie/dev/tools/hearth && npx vitest run src/__tests__/QuickCapture.test.tsx
```

Expected: 4 tests pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/windows/QuickCapture.tsx src/__tests__/QuickCapture.test.tsx src/main.tsx
git commit -m "feat(quick-capture): overlay component + route split"
```

---

## Task 8: ShortcutRecorder component (TDD)

**Files:**
- Create: `src/components/settings/ShortcutRecorder.tsx`
- Create: `src/components/settings/__tests__/ShortcutRecorder.test.tsx`

- [ ] **Step 1: Write failing test**

Create `src/components/settings/__tests__/ShortcutRecorder.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ShortcutRecorder } from "../ShortcutRecorder";

describe("ShortcutRecorder", () => {
  it("captures a modifier+letter combo and emits normalized string", async () => {
    const onSave = vi.fn();
    render(<ShortcutRecorder onSave={onSave} onCancel={() => {}} />);
    const area = screen.getByRole("button", { name: /녹화/ });
    area.focus();
    await userEvent.keyboard("{Meta>}{Shift>}h{/Shift}{/Meta}");
    await userEvent.click(screen.getByRole("button", { name: /확인/ }));
    expect(onSave).toHaveBeenCalledWith("Cmd+Shift+H");
  });

  it("ignores modifier-only input", async () => {
    const onSave = vi.fn();
    render(<ShortcutRecorder onSave={onSave} onCancel={() => {}} />);
    const area = screen.getByRole("button", { name: /녹화/ });
    area.focus();
    await userEvent.keyboard("{Shift}");
    const save = screen.getByRole("button", { name: /확인/ });
    expect(save).toBeDisabled();
  });
});
```

- [ ] **Step 2: Run tests to verify failure**

```bash
cd /Users/genie/dev/tools/hearth && npx vitest run src/components/settings/__tests__/ShortcutRecorder.test.tsx
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement ShortcutRecorder**

Create `src/components/settings/ShortcutRecorder.tsx`:

```tsx
import { useCallback, useState } from "react";
import { Button } from "../../ui/Button";

type Captured = {
  ctrl: boolean;
  meta: boolean;
  alt: boolean;
  shift: boolean;
  key: string | null; // uppercase letter / digit / function key / named
};

const EMPTY: Captured = {
  ctrl: false,
  meta: false,
  alt: false,
  shift: false,
  key: null,
};

function format(c: Captured): string | null {
  if (!c.key) return null;
  const parts: string[] = [];
  if (c.meta) parts.push("Cmd");
  else if (c.ctrl) parts.push("Ctrl");
  if (c.alt) parts.push("Alt");
  if (c.shift) parts.push("Shift");
  parts.push(c.key);
  return parts.join("+");
}

export function ShortcutRecorder({
  onSave,
  onCancel,
}: {
  onSave: (combo: string) => void;
  onCancel: () => void;
}) {
  const [cap, setCap] = useState<Captured>(EMPTY);

  const onKeyDown = useCallback((e: React.KeyboardEvent<HTMLButtonElement>) => {
    e.preventDefault();
    const k = e.key;
    if (k === "Escape") {
      onCancel();
      return;
    }
    const mods = {
      ctrl: e.ctrlKey,
      meta: e.metaKey,
      alt: e.altKey,
      shift: e.shiftKey,
    };
    if (["Control", "Meta", "Alt", "Shift"].includes(k)) {
      setCap((prev) => ({ ...prev, ...mods }));
      return;
    }
    let key: string | null = null;
    if (k.length === 1 && /[a-zA-Z0-9]/.test(k)) key = k.toUpperCase();
    else if (/^F\d{1,2}$/.test(k)) key = k;
    else if (k === " ") key = "Space";
    else if (["Enter", "Tab", "Backspace", "Delete", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(k))
      key = k;
    if (!key) return;
    setCap({ ...mods, key });
  }, [onCancel]);

  const display = format(cap) ?? "키를 눌러주세요...";
  const canSave = !!format(cap);

  return (
    <div className="flex flex-col gap-3">
      <button
        type="button"
        aria-label="단축키 녹화 영역 — 키를 누르세요"
        onKeyDown={onKeyDown}
        className="rounded border border-white/15 px-4 py-3 text-center text-sm bg-black/30 focus:outline-none focus:ring-2 focus:ring-amber-400"
      >
        {display}
      </button>
      <div className="flex gap-2 justify-end">
        <Button variant="secondary" size="sm" onClick={onCancel}>
          취소
        </Button>
        <Button
          size="sm"
          disabled={!canSave}
          onClick={() => {
            const c = format(cap);
            if (c) onSave(c);
          }}
        >
          확인
        </Button>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Re-run tests to verify pass**

```bash
cd /Users/genie/dev/tools/hearth && npx vitest run src/components/settings/__tests__/ShortcutRecorder.test.tsx
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/settings/ShortcutRecorder.tsx src/components/settings/__tests__/
git commit -m "feat(quick-capture): shortcut recorder primitive"
```

---

## Task 9: Settings integration + reactive hook

**Files:**
- Create: `src/hooks/useQuickCaptureShortcut.ts`
- Modify: `src/components/SettingsGeneralSection.tsx`

- [ ] **Step 1: Create hook**

Create `src/hooks/useQuickCaptureShortcut.ts`:

```ts
import { useCallback, useEffect, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  getQuickCaptureShortcut,
  getQuickCaptureShortcutError,
  rebindQuickCaptureShortcut,
} from "../api";

function pretty(combo: string): string {
  return combo
    .replace("CommandOrControl", navigator.userAgent.includes("Mac") ? "⌘" : "Ctrl")
    .replace("Shift", "⇧")
    .replace("Alt", "⌥")
    .replace(/\+/g, "")
    .toUpperCase();
}

export function useQuickCaptureShortcut() {
  const [combo, setCombo] = useState<string>("");
  const [error, setError] = useState<string>("");

  const reload = useCallback(async () => {
    const [c, e] = await Promise.all([
      getQuickCaptureShortcut(),
      getQuickCaptureShortcutError().catch(() => ""),
    ]);
    setCombo(c);
    setError(e);
  }, []);

  useEffect(() => {
    void reload();
    let off: UnlistenFn | null = null;
    listen<string>("quick-capture-shortcut:changed", (e) => {
      setCombo(e.payload);
      setError("");
    }).then((f) => (off = f));
    return () => {
      if (off) off();
    };
  }, [reload]);

  const rebind = useCallback(async (next: string) => {
    const saved = await rebindQuickCaptureShortcut(next);
    setCombo(saved);
    setError("");
    return saved;
  }, []);

  return { combo, display: combo ? pretty(combo) : "", error, rebind, reload };
}
```

- [ ] **Step 2: Integrate into SettingsGeneralSection**

In `src/components/SettingsGeneralSection.tsx`, add near the top:

```tsx
import { useState } from "react";
import { useQuickCaptureShortcut } from "../hooks/useQuickCaptureShortcut";
import { ShortcutRecorder } from "./settings/ShortcutRecorder";
```

Inside the component body (before `return`):

```tsx
  const { combo, display, error: shortcutError, rebind } = useQuickCaptureShortcut();
  const [recording, setRecording] = useState(false);
  const [rebindError, setRebindError] = useState<string | null>(null);
```

Inside the returned JSX, append as the last `<section>` in the top-level `<div className="flex flex-col gap-6">`:

```tsx
      <section>
        <h3 className="text-[13px] text-[var(--color-text-hi)] mb-2">
          Quick Capture
        </h3>
        <div className="flex items-center gap-3 text-[13px]">
          <span className="font-mono rounded bg-black/40 px-2 py-1">
            {display || "—"}
          </span>
          {!recording && (
            <Button size="sm" onClick={() => setRecording(true)}>
              변경
            </Button>
          )}
        </div>
        {recording && (
          <div className="mt-3">
            <ShortcutRecorder
              onCancel={() => {
                setRecording(false);
                setRebindError(null);
              }}
              onSave={async (next) => {
                try {
                  await rebind(next);
                  setRecording(false);
                  setRebindError(null);
                } catch (e) {
                  setRebindError(String(e));
                }
              }}
            />
          </div>
        )}
        {(shortcutError || rebindError) && (
          <p className="mt-2 text-xs text-red-400">
            {rebindError ?? `단축키 등록 실패: ${shortcutError}`}
          </p>
        )}
        <p className="mt-2 text-[11px] text-[var(--color-text-muted)]">
          어느 앱에서든 이 단축키로 한 줄 메모를 남길 수 있어요.
          Hearth가 완전히 종료되면 작동하지 않으니 "로그인 시 자동 실행"을 켜두는 걸 추천합니다.
          저장된 메모는 기본 노란색으로 메모 탭 상단에 쌓입니다. (combo: <code>{combo}</code>)
        </p>
      </section>
```

- [ ] **Step 3: Typecheck + unit tests**

```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit && npx vitest run
```

Expected: all pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/hooks/useQuickCaptureShortcut.ts src/components/SettingsGeneralSection.tsx
git commit -m "feat(quick-capture): settings UI for rebinding"
```

---

## Task 10: Main-window toast for `memo:quick-captured`

**Files:**
- Modify: `src/components/Layout.tsx`

- [ ] **Step 1: Read current Layout toast context to confirm shape**

```bash
cd /Users/genie/dev/tools/hearth && rg -n "useToast|success\(|info\(" src/components/Layout.tsx
```

Note the toast API shape.

- [ ] **Step 2: Add Tauri event listener**

At the top of `Layout.tsx`, add:

```ts
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
```

Inside the component body, add a `useEffect` alongside existing event wiring:

```tsx
  useEffect(() => {
    let off: UnlistenFn | null = null;
    listen<{ memoId: number }>("memo:quick-captured", (e) => {
      const id = e.payload?.memoId;
      toast.success("메모 추가됨", {});
      // also dispatch the highlight event so the MemoBoard scrolls+pulses
      window.dispatchEvent(
        new CustomEvent("memo:focus", { detail: { memoId: id } })
      );
      // also trigger memos reload via existing event bus
      window.dispatchEvent(new Event("memos:changed"));
    }).then((f) => (off = f));
    return () => {
      if (off) off();
    };
  }, [toast]);
```

(Replace `toast.success("메모 추가됨", {})` with whatever shape the current `useToast` exposes — e.g. `toast.success("메모 추가됨")`. Match existing calls in the file.)

If the Memo tab is not currently active, switching tabs should be the user's responsibility (click toast → future enhancement). MVP emits the focus event regardless — if the tab is visible the pulse runs; otherwise it's a no-op.

- [ ] **Step 3: Run frontend tests**

```bash
cd /Users/genie/dev/tools/hearth && npm test
```

Expected: existing + new tests all green.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/Layout.tsx
git commit -m "feat(quick-capture): toast + highlight on main window"
```

---

## Task 11: Manual QA + release chores

**Files:**
- Modify: `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json`, `CHANGELOG.md`, `README.md`

- [ ] **Step 1: Manual QA in dev**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

Walk through each item, stop dev when done:

1. From Safari (or any other app), press `⌃⇧H` → overlay appears top-center with focused input.
2. Type `quick capture smoke`, press Enter → overlay closes. Open Hearth main window → memo tab shows the new memo (yellow) + a toast "메모 추가됨" was visible + scroll pulse animated.
3. Press `⌃⇧H`, type nothing, press `Esc` → overlay closes, no memo saved.
4. Press `⌃⇧H` again, type `line1`, Shift+Enter, `line2`, Enter → memo has two lines.
5. Press `⌃⇧H` while overlay is open → overlay closes (toggle).
6. Click somewhere else while overlay is open → overlay auto-closes.
7. Settings → 일반 → Quick Capture: click 변경, record `Ctrl+Shift+J`, confirm. `⌃⇧H` no longer triggers, `⌃⇧J` does.
8. Try to rebind to `Cmd+Space` (Spotlight) → expect inline error.
9. Quit Hearth entirely via ⌘Q → `⌃⇧H` does nothing (documented limitation).

- [ ] **Step 2: Version bump**

Update to `0.5.0` in three files:
- `src-tauri/tauri.conf.json` → `"version": "0.5.0"`
- `src-tauri/Cargo.toml` → `version = "0.5.0"` in `[package]`
- `package.json` → `"version": "0.5.0"`

- [ ] **Step 3: CHANGELOG entry**

In `CHANGELOG.md`, add above the previous entry:

```markdown
## [0.5.0] - 2026-04-19

### Added
- **Quick Capture** — 전역 단축키(`⌃⇧H` 기본, 설정에서 리바인드 가능)로 앱을 포커스하지 않고도 한 줄 메모를 바로 저장. Shift+Enter로 여러 줄, Esc로 취소. 저장 시 메인 창에 토스트 + 메모 탭 하이라이트 펄스.
- 설정 → 일반에 Quick Capture 섹션 추가 (현재 단축키 뱃지 + 녹화 UI + 등록 실패 시 인라인 경고).
```

- [ ] **Step 4: README update**

In `README.md`, under `## Features`, add after the `⌘F` 전체 검색 line:

```markdown
- **Quick Capture** (`⌃⇧H`) — 다른 앱에서도 단축키 한 번이면 작은 오버레이가 떠서 한 줄 메모 저장. 설정에서 단축키 변경 가능
```

- [ ] **Step 5: Final test sweep**

```bash
cd /Users/genie/dev/tools/hearth && npm test && cd src-tauri && cargo test
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock package.json package-lock.json CHANGELOG.md README.md
git commit -m "feat: 0.5.0 — Quick Capture global shortcut overlay"
```

---

## Done Criteria

- `⌃⇧H` from any app opens the overlay; Enter saves a yellow memo and shows a toast + pulse in Hearth.
- Settings → 일반 → Quick Capture shows the current combo and can rebind it; failures appear inline.
- All unit tests (Rust `cargo test` + Vitest `npm test`) pass.
- Spec + backlog + plan + CHANGELOG + README reflect 0.5.0.
- No regressions to existing ⌘K / ⌘F / context menu / notifications.
