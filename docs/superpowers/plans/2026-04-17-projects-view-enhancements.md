# Projects View Enhancements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the four features in `docs/superpowers/specs/2026-04-17-projects-view-enhancements-design.md` — cross-priority drag-and-drop, `Cmd+=/-/0` UI zoom, a card-grid project view with a double-click detail dialog, and memo grouping with global `#N` badges plus supporting AI tools.

**Architecture:** Additive changes on top of the existing schema. Backend adds four new Tauri commands (`get_ui_scale`, `set_ui_scale`, `update_memo_by_number`, `delete_memo_by_number`), extends the existing `create_memo` AI tool with a `project_name` parameter, and exposes two new AI tool specs for memo editing by `#N`. Frontend introduces a Vitest harness, a `useUiScale` hook, a detailed two-column `ProjectCard`, a new `ProjectDetailDialog`, a unified `DndContext` for cross-priority drag, and memo grouping with `#N` badges in `MemoBoard`.

**Tech Stack:** React 19, TypeScript 5.8, Tailwind 4, dnd-kit 6.3 + sortable 10.0, Tauri 2, rusqlite 0.34, Vitest (new).

---

## File Structure

**New files**

```
src/hooks/useUiScale.ts                 Cmd+=/-/0 keyboard hook + settings round-trip
src/hooks/__tests__/useUiScale.test.ts  Vitest
src/components/ProjectFormFields.tsx    Shared fields for New/Edit dialogs
src/components/ProjectDetailDialog.tsx  Edit form + scoped memo CRUD panel
src/components/EmptyDropZone.tsx        useDroppable wrapper for empty priority/memo groups
src/lib/dragTargets.ts                  deriveTarget(over, projects) pure helper
src/lib/__tests__/dragTargets.test.ts   Vitest
src/lib/memoSequence.ts                 globalSequence(memos) pure helper
src/lib/__tests__/memoSequence.test.ts  Vitest
src/__tests__/smoke.test.ts             Vitest smoke test (setup verification)
vitest.config.ts                        Vitest config (jsdom, alias)
```

**Modified files**

```
package.json                           + vitest, @testing-library, jsdom deps + "test" script
src-tauri/src/cmd_settings.rs          + get_ui_scale / set_ui_scale + K_UI_SCALE constant
src-tauri/src/cmd_memos.rs             + update_memo_by_number / delete_memo_by_number
src-tauri/src/lib.rs                   + 4 new command registrations
src-tauri/src/ai_tools.rs              + project_name param on create_memo, + 2 new tool specs
src-tauri/tests/memo_by_number.rs      NEW Rust integration test
src/api.ts                             + 4 new bindings
src/App.tsx                            + useUiScale() call at mount
src/command/buildSystemPrompt.ts       + [메모] section explaining #N semantics
src/components/NewProjectDialog.tsx    use ProjectFormFields
src/components/ProjectList.tsx         unified DndContext + cross-priority drag + detail dialog wiring
src/components/ProjectCard.tsx         two-column detailed layout + double-click handler
src/components/MemoBoard.tsx           grouped rendering + #N badges + cross-group drag
src/components/MemoCard.tsx            sequenceNumber prop + badge render
```

Each task below produces one focused commit.

---

## Task 1: Vitest harness setup

**Files:**
- Create: `/Users/genie/dev/tools/hearth/vitest.config.ts`
- Create: `/Users/genie/dev/tools/hearth/src/__tests__/smoke.test.ts`
- Modify: `/Users/genie/dev/tools/hearth/package.json`

- [ ] **Step 1: Install dev dependencies**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm install --save-dev vitest@^3 jsdom@^25 @testing-library/react@^16 @testing-library/jest-dom@^6
```
Expected: dependencies added to `devDependencies`, `node_modules` populated, no errors.

- [ ] **Step 2: Create Vitest config**

Create `/Users/genie/dev/tools/hearth/vitest.config.ts`:

```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    include: ["src/**/*.test.{ts,tsx}"],
    setupFiles: [],
  },
});
```

- [ ] **Step 3: Add `test` script to package.json**

Edit `/Users/genie/dev/tools/hearth/package.json` to add a `test` entry under `scripts`:

```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 4: Write smoke test**

Create `/Users/genie/dev/tools/hearth/src/__tests__/smoke.test.ts`:

```ts
import { describe, it, expect } from "vitest";

describe("vitest harness", () => {
  it("runs", () => {
    expect(1 + 1).toBe(2);
  });
});
```

- [ ] **Step 5: Run smoke test**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test
```
Expected: `1 passed`. If Vite cannot resolve `@vitejs/plugin-react`, the project already depends on it via `vite.config.ts`; verify it's in `devDependencies` and reinstall.

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add package.json package-lock.json vitest.config.ts src/__tests__/smoke.test.ts && git commit -m "test: bootstrap vitest harness with jsdom"
```

---

## Task 2: Backend `get_ui_scale` / `set_ui_scale` commands

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_settings.rs`
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`

- [ ] **Step 1: Add constant and commands in cmd_settings.rs**

Append below the existing `K_OPENAI_KEY` constant (near line 28) and after `save_ai_settings`:

```rust
const K_UI_SCALE: &str = "ui.scale";

#[tauri::command]
pub fn get_ui_scale(state: State<'_, crate::AppState>) -> Result<f64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw = read(&db, K_UI_SCALE)?;
    if raw.is_empty() {
        return Ok(1.0);
    }
    raw.parse::<f64>().map_err(|e| format!("invalid ui.scale value: {e}"))
}

#[tauri::command]
pub fn set_ui_scale(
    state: State<'_, crate::AppState>,
    scale: f64,
) -> Result<(), String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!("invalid scale: {scale}"));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    write(&db, K_UI_SCALE, &scale.to_string())
}
```

If `read` / `write` are not already `pub(crate)`, leave them private — the new commands are in the same module.

- [ ] **Step 2: Register commands**

Edit `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`. Find the `generate_handler!` block (around lines 55–85) and add `cmd_settings::get_ui_scale` and `cmd_settings::set_ui_scale` to the comma-separated list.

- [ ] **Step 3: Compile**

Run:
```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```
Expected: PASS, no warnings about the two new functions.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/cmd_settings.rs src-tauri/src/lib.rs && git commit -m "feat(settings): add get_ui_scale / set_ui_scale commands"
```

---

## Task 3: Backend `update_memo_by_number` / `delete_memo_by_number` commands

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_memos.rs`
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`
- Test: `/Users/genie/dev/tools/hearth/src-tauri/tests/memo_by_number.rs`

- [ ] **Step 1: Write failing Rust integration test**

Create `/Users/genie/dev/tools/hearth/src-tauri/tests/memo_by_number.rs`:

```rust
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE projects (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            priority TEXT NOT NULL DEFAULT 'P4',
            number INTEGER,
            name TEXT NOT NULL,
            category TEXT,
            path TEXT,
            evaluation TEXT,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        CREATE TABLE memos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL DEFAULT '',
            color TEXT NOT NULL DEFAULT 'yellow',
            project_id INTEGER REFERENCES projects(id) ON DELETE SET NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );
        INSERT INTO memos (content, sort_order) VALUES
            ('first', 0), ('second', 1), ('third', 2);
        "#,
    ).unwrap();
    conn
}

#[test]
fn resolve_number_offset_maps_to_correct_memo() {
    let conn = setup_db();
    // number=2 → 2nd memo (sort_order=1) = 'second'
    let (id, content): (i64, String) = conn
        .query_row(
            "SELECT id, content FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [1_i64],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
    assert_eq!(content, "second");
    assert_eq!(id, 2);
}

#[test]
fn resolve_number_out_of_range_returns_err() {
    let conn = setup_db();
    let result: rusqlite::Result<(i64, String)> = conn.query_row(
        "SELECT id, content FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
        [10_i64],
        |r| Ok((r.get(0)?, r.get(1)?)),
    );
    assert!(matches!(result, Err(rusqlite::Error::QueryReturnedNoRows)));
}
```

This test locks down the OFFSET semantics the production code will rely on.

- [ ] **Step 2: Run test to verify it passes immediately**

Run:
```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo test --test memo_by_number
```
Expected: 2 passed. (Both tests assert SQLite semantics — they pass on setup.)

- [ ] **Step 3: Add backend functions in cmd_memos.rs**

Append to `/Users/genie/dev/tools/hearth/src-tauri/src/cmd_memos.rs`:

```rust
#[tauri::command]
pub fn update_memo_by_number(
    state: State<'_, crate::AppState>,
    number: i64,
    fields: UpdateMemoInput,
) -> Result<Memo, String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let id: i64 = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT id FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [number - 1],
            |r| r.get(0),
        )
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))?
    };
    update_memo(state, id, fields)
}

#[tauri::command]
pub fn delete_memo_by_number(
    state: State<'_, crate::AppState>,
    number: i64,
) -> Result<(), String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    let id: i64 = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT id FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [number - 1],
            |r| r.get(0),
        )
        .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))?
    };
    delete_memo(state, id)
}
```

The dropped `db` lock before calling `update_memo` / `delete_memo` is intentional — both callees re-acquire the lock.

- [ ] **Step 4: Register commands in lib.rs**

Add `cmd_memos::update_memo_by_number` and `cmd_memos::delete_memo_by_number` to the `generate_handler!` macro invocation in `/Users/genie/dev/tools/hearth/src-tauri/src/lib.rs`.

- [ ] **Step 5: Verify compile**

Run:
```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```
Expected: PASS with no warnings.

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/cmd_memos.rs src-tauri/src/lib.rs src-tauri/tests/memo_by_number.rs && git commit -m "feat(memos): add update/delete_memo_by_number commands"
```

---

## Task 4: Extend AI tools for memo `#N` operations

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src-tauri/src/ai_tools.rs`

- [ ] **Step 1: Extend `create_memo` tool spec**

Find the existing `create_memo` `ToolSpec` in the `specs()` function (around lines 425–460 in `ai_tools.rs`). Replace its `parameters` JSON Schema so that the properties object becomes:

```rust
"properties": {
    "content": { "type": "string", "description": "메모 내용" },
    "color": {
        "type": "string",
        "enum": ["yellow", "pink", "green", "blue"],
        "description": "메모 색상 (선택)"
    },
    "project_id": {
        "type": "integer",
        "description": "연결할 프로젝트 ID (선택). project_name 보다 우선."
    },
    "project_name": {
        "type": "string",
        "description": "연결할 프로젝트 이름 부분 일치 (선택). project_id 가 없을 때만 사용됨."
    }
},
"required": ["content"]
```

Update the `description` field of that tool spec to mention: `"project_name 제공 시 LIKE 매칭으로 project_id 자동 해석. 못 찾으면 기타로 저장."`.

- [ ] **Step 2: Extend `create_memo` handler**

In the same file, locate the handler branch inside `execute()` matching `"create_memo"` (around lines 455–491). Insert a resolution step after extracting `content` and before the INSERT:

```rust
let project_id: Option<i64> = if let Some(pid) = args.get("project_id").and_then(|v| v.as_i64()) {
    Some(pid)
} else if let Some(name) = args.get("project_name").and_then(|v| v.as_str()) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        None
    } else {
        let pattern = format!("%{}%", trimmed);
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.query_row(
            "SELECT id FROM projects WHERE name LIKE ? ORDER BY \
             CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 \
                           WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 ELSE 5 END, \
             sort_order LIMIT 1",
            [&pattern],
            |r| r.get::<_, i64>(0),
        ).ok()
    }
} else {
    None
};
```

Drop the lock before the later INSERT if the existing code re-locks. If the existing handler extracts `project_id` inline in the INSERT, replace that reference with the new `project_id` binding above. Confirm the final INSERT uses `project_id` (may be `NULL`) and returns the created row.

Match the existing error/return style — do NOT introduce a new return type.

- [ ] **Step 3: Add `update_memo_by_number` tool spec**

Append a new `ToolSpec` entry to the `specs()` vector (immediately after the updated `create_memo` spec):

```rust
ToolSpec {
    name: "update_memo_by_number".into(),
    description: "#N 뱃지 번호로 메모 내용 수정. 번호는 사용자 화면의 현재 #N. \
                  범위 밖이면 '#N 메모를 찾을 수 없음' 오류.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "number": { "type": "integer", "description": "메모 뱃지 번호 (1부터)" },
            "content": { "type": "string", "description": "새 내용" }
        },
        "required": ["number", "content"]
    }),
    kind: ToolKind::Mutation,
},
```

- [ ] **Step 4: Add `delete_memo_by_number` tool spec**

Append immediately after the update spec:

```rust
ToolSpec {
    name: "delete_memo_by_number".into(),
    description: "#N 뱃지 번호로 메모 삭제.".into(),
    parameters: serde_json::json!({
        "type": "object",
        "properties": {
            "number": { "type": "integer", "description": "메모 뱃지 번호 (1부터)" }
        },
        "required": ["number"]
    }),
    kind: ToolKind::Mutation,
},
```

- [ ] **Step 5: Add handler branches in `execute()`**

In the `execute()` match block (around lines 249–267 and onward), add two new arms:

```rust
"update_memo_by_number" => {
    let number = args.get("number").and_then(|v| v.as_i64())
        .ok_or_else(|| "number is required".to_string())?;
    let content = args.get("content").and_then(|v| v.as_str())
        .ok_or_else(|| "content is required".to_string())?
        .to_string();
    let id: i64 = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if number < 1 {
            return Err(format!("#{} 메모를 찾을 수 없음", number));
        }
        db.query_row(
            "SELECT id FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [number - 1],
            |r| r.get(0),
        ).map_err(|_| format!("#{} 메모를 찾을 수 없음", number))?
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE memos SET content = ?, updated_at = datetime('now') WHERE id = ?",
        rusqlite::params![content, id],
    ).map_err(|e| e.to_string())?;
    serde_json::to_value(serde_json::json!({ "id": id, "updated": true }))
        .map_err(|e| e.to_string())
}
"delete_memo_by_number" => {
    let number = args.get("number").and_then(|v| v.as_i64())
        .ok_or_else(|| "number is required".to_string())?;
    let id: i64 = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        if number < 1 {
            return Err(format!("#{} 메모를 찾을 수 없음", number));
        }
        db.query_row(
            "SELECT id FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?",
            [number - 1],
            |r| r.get(0),
        ).map_err(|_| format!("#{} 메모를 찾을 수 없음", number))?
    };
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM memos WHERE id = ?", [id])
        .map_err(|e| e.to_string())?;
    serde_json::to_value(serde_json::json!({ "id": id, "deleted": true }))
        .map_err(|e| e.to_string())
}
```

Match existing return-type conventions in that file. If `execute` already returns `Result<serde_json::Value, String>`, the above matches.

- [ ] **Step 6: Compile**

Run:
```bash
cd /Users/genie/dev/tools/hearth/src-tauri && cargo check
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src-tauri/src/ai_tools.rs && git commit -m "feat(ai): memo #N operations + project_name resolution in create_memo"
```

---

## Task 5: Frontend API bindings for new commands

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/api.ts`

- [ ] **Step 1: Add four bindings**

Append to the end of `src/api.ts`:

```ts
export async function getUiScale(): Promise<number> {
  return invoke<number>("get_ui_scale");
}

export async function setUiScale(scale: number): Promise<void> {
  await invoke("set_ui_scale", { scale });
}

export async function updateMemoByNumber(
  number: number,
  fields: { content?: string; color?: string; project_id?: number | null },
): Promise<Memo> {
  return invoke<Memo>("update_memo_by_number", { number, fields });
}

export async function deleteMemoByNumber(number: number): Promise<void> {
  await invoke("delete_memo_by_number", { number });
}
```

Confirm `Memo` is already imported at the top of this file (it should be — `updateMemo` references it). If not, add it to the existing `import type` line.

- [ ] **Step 2: Type check**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/api.ts && git commit -m "feat(api): bindings for ui_scale + memo #N commands"
```

---

## Task 6: `useUiScale` hook + App.tsx mount

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/hooks/useUiScale.ts`
- Test: `/Users/genie/dev/tools/hearth/src/hooks/__tests__/useUiScale.test.ts`
- Modify: `/Users/genie/dev/tools/hearth/src/App.tsx`

- [ ] **Step 1: Write failing hook test**

Create `/Users/genie/dev/tools/hearth/src/hooks/__tests__/useUiScale.test.ts`:

```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { useUiScale, __STEPS_FOR_TEST } from "../useUiScale";

vi.mock("../../api", () => ({
  getUiScale: vi.fn().mockResolvedValue(1.0),
  setUiScale: vi.fn().mockResolvedValue(undefined),
}));

describe("useUiScale", () => {
  beforeEach(() => {
    document.documentElement.style.zoom = "";
  });

  it("uses the DEFAULT step when none is persisted", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    expect(result.current.scale).toBe(1.0);
  });

  it("bump(+1) moves one step up, clamped at the max", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => { result.current.bump(1); });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[2]);  // 1.15
    act(() => { result.current.bump(1); });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[3]);  // 1.3
    act(() => { result.current.bump(1); });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[3]);  // clamp
  });

  it("bump(-1) moves one step down, clamped at the min", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => { result.current.bump(-1); });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[0]);  // 0.85
    act(() => { result.current.bump(-1); });
    expect(result.current.scale).toBe(__STEPS_FOR_TEST[0]);  // clamp
  });

  it("reset() returns to DEFAULT", async () => {
    const { result } = renderHook(() => useUiScale());
    await act(async () => {});
    act(() => { result.current.bump(1); });
    act(() => { result.current.reset(); });
    expect(result.current.scale).toBe(1.0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/hooks/__tests__/useUiScale.test.ts
```
Expected: FAIL with `Cannot find module '../useUiScale'`.

- [ ] **Step 3: Implement the hook**

Create `/Users/genie/dev/tools/hearth/src/hooks/useUiScale.ts`:

```ts
import { useCallback, useEffect, useState } from "react";
import * as api from "../api";

const STEPS = [0.85, 1.0, 1.15, 1.3] as const;
const DEFAULT: number = 1.0;

export const __STEPS_FOR_TEST = STEPS;

export function useUiScale() {
  const [scale, setScale] = useState<number>(DEFAULT);

  const apply = useCallback((next: number) => {
    document.documentElement.style.zoom = String(next);
    setScale(next);
    api.setUiScale(next).catch(() => { /* fire-and-forget */ });
  }, []);

  const bump = useCallback((dir: 1 | -1) => {
    setScale((current) => {
      const idx = STEPS.indexOf(current as (typeof STEPS)[number]);
      const base = idx === -1 ? STEPS.indexOf(DEFAULT) : idx;
      const nextIdx = Math.max(0, Math.min(STEPS.length - 1, base + dir));
      const next = STEPS[nextIdx];
      document.documentElement.style.zoom = String(next);
      api.setUiScale(next).catch(() => {});
      return next;
    });
  }, []);

  const reset = useCallback(() => apply(DEFAULT), [apply]);

  useEffect(() => {
    api.getUiScale()
      .then((v) => apply(Number.isFinite(v) && v > 0 ? v : DEFAULT))
      .catch(() => apply(DEFAULT));
  }, [apply]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey)) return;
      if (e.key === "=" || e.key === "+") { e.preventDefault(); bump(1); }
      else if (e.key === "-") { e.preventDefault(); bump(-1); }
      else if (e.key === "0") { e.preventDefault(); reset(); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bump, reset]);

  return { scale, bump, reset };
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/hooks/__tests__/useUiScale.test.ts
```
Expected: 4 passed.

- [ ] **Step 5: Mount the hook in App.tsx**

Open `/Users/genie/dev/tools/hearth/src/App.tsx`. Inside the top-level `App` component (the one that wraps `Layout` + `ToastProvider`), call the hook:

```tsx
import { useUiScale } from "./hooks/useUiScale";

// ...existing imports

export default function App() {
  useUiScale();
  // ...existing JSX unchanged
}
```

If the top-level component has a different name, apply the same edit there. The return value is unused — the hook's effects are side-effectful.

- [ ] **Step 6: Type check + manual verify**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

Run the app:
```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
In the window, press `Cmd+=` three times, `Cmd+-` two times, `Cmd+0`. Confirm UI scales. Close the app, reopen — confirm the last scale persists.

- [ ] **Step 7: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/hooks/useUiScale.ts src/hooks/__tests__/useUiScale.test.ts src/App.tsx && git commit -m "feat(ui): Cmd+=/-/0 zoom with persisted scale"
```

---

## Task 7: Extract `ProjectFormFields` from `NewProjectDialog`

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/ProjectFormFields.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/NewProjectDialog.tsx`

- [ ] **Step 1: Create `ProjectFormFields` component**

Create `/Users/genie/dev/tools/hearth/src/components/ProjectFormFields.tsx`:

```tsx
import { Input } from "../ui/Input";
import { PRIORITIES, CATEGORIES, type Priority, type Category } from "../types";

export type ProjectFormState = {
  name: string;
  priority: Priority;
  category: Category | "";
  path: string;
  evaluation: string;
};

export function ProjectFormFields({
  value,
  onChange,
  disableName = false,
}: {
  value: ProjectFormState;
  onChange: (patch: Partial<ProjectFormState>) => void;
  disableName?: boolean;
}) {
  return (
    <div className="flex flex-col gap-3">
      <label className="text-small text-[var(--color-text-muted)]">
        이름
        <Input
          className="mt-1"
          value={value.name}
          disabled={disableName}
          onChange={(e) => onChange({ name: e.target.value })}
        />
      </label>

      <div className="grid grid-cols-2 gap-3">
        <label className="text-small text-[var(--color-text-muted)]">
          우선순위
          <select
            className="mt-1 w-full rounded-md bg-[var(--color-surface-2)] px-2 py-1 text-body"
            value={value.priority}
            onChange={(e) => onChange({ priority: e.target.value as Priority })}
          >
            {PRIORITIES.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </label>

        <label className="text-small text-[var(--color-text-muted)]">
          카테고리
          <select
            className="mt-1 w-full rounded-md bg-[var(--color-surface-2)] px-2 py-1 text-body"
            value={value.category}
            onChange={(e) => onChange({ category: e.target.value as Category | "" })}
          >
            <option value="">—</option>
            {CATEGORIES.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </select>
        </label>
      </div>

      <label className="text-small text-[var(--color-text-muted)]">
        경로
        <Input
          className="mt-1"
          value={value.path}
          onChange={(e) => onChange({ path: e.target.value })}
          placeholder="~/dev/…"
        />
      </label>

      <label className="text-small text-[var(--color-text-muted)]">
        평가
        <textarea
          className="mt-1 w-full min-h-[96px] rounded-md bg-[var(--color-surface-2)] px-2 py-1 text-body"
          value={value.evaluation}
          onChange={(e) => onChange({ evaluation: e.target.value })}
          placeholder="현재 진행 상황, 메모…"
        />
      </label>
    </div>
  );
}
```

If the codebase's existing `<Input>` component import path differs, adjust the import (look at `NewProjectDialog.tsx` for the real path).

- [ ] **Step 2: Refactor NewProjectDialog**

Open `/Users/genie/dev/tools/hearth/src/components/NewProjectDialog.tsx`. Replace the inline form fields with the extracted component:

```tsx
import { ProjectFormFields, type ProjectFormState } from "./ProjectFormFields";

// Inside the component, replace the existing useState calls for name/priority/category/path
// with one combined state:
const [form, setForm] = useState<ProjectFormState>({
  name: "",
  priority: "P4",
  category: "",
  path: "",
  evaluation: "",
});

// Replace the inline inputs with:
<ProjectFormFields
  value={form}
  onChange={(patch) => setForm((prev) => ({ ...prev, ...patch }))}
/>

// Update handleSubmit to read from form instead of individual variables:
await api.createProject({
  name: form.name.trim(),
  priority: form.priority,
  category: form.category === "" ? null : form.category,
  path: form.path.trim() || null,
});
```

Keep the existing validation (reject empty name), reset logic, and close-on-success behavior. `evaluation` is not sent on create but is present in state for dialog reuse.

- [ ] **Step 3: Type check and manual verify**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Open the New Project dialog via `Cmd+K` → "새 프로젝트" or TopBar. Confirm all fields render and submit still creates a project.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/ProjectFormFields.tsx src/components/NewProjectDialog.tsx && git commit -m "refactor(projects): extract ProjectFormFields for dialog reuse"
```

---

## Task 8: `ProjectCard` two-column detailed layout + double-click

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectCard.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectList.tsx`

- [ ] **Step 1: Add `onOpenDetail` prop and refactor layout**

Replace the rendered JSX in `/Users/genie/dev/tools/hearth/src/components/ProjectCard.tsx` so the card is a detailed block:

```tsx
type ProjectCardProps = {
  project: Project;
  onUpdate: (id: number, fields: UpdateProjectInput) => Promise<void> | void;
  onDelete: (id: number) => Promise<void> | void;
  onOpenGhostty: (project: Project) => void;
  onOpenFinder: (project: Project) => void;
  onOpenDetail: (project: Project) => void;  // NEW
};

export function ProjectCard({ project, onUpdate, onDelete, onOpenGhostty, onOpenFinder, onOpenDetail }: ProjectCardProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: project.id });
  // ... existing state (editing flags, popover open, etc.) unchanged

  const cardStyle: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  const stop = (e: React.MouseEvent) => e.stopPropagation();

  return (
    <div
      ref={setNodeRef}
      style={cardStyle}
      onDoubleClick={() => onOpenDetail(project)}
      className="group relative rounded-lg border border-[var(--color-surface-3)] bg-[var(--color-surface-1)] px-3 py-2.5 hover:border-[var(--color-brand-soft)] transition-colors cursor-default"
    >
      <div className="absolute top-2 right-2 flex gap-1 opacity-50 group-hover:opacity-100 transition-opacity" onClick={stop} onDoubleClick={stop}>
        {/* existing Ghostty / Folder / X icon buttons */}
      </div>

      <div className="flex items-start gap-2">
        <button
          {...attributes}
          {...listeners}
          aria-label="drag"
          className="cursor-grab text-[var(--color-text-dim)] hover:text-[var(--color-text)] mt-0.5"
          onClick={stop}
          onDoubleClick={stop}
        >
          ≡
        </button>
        <div className="flex-1 min-w-0">
          <div
            className="text-body font-semibold text-[var(--color-text-hi)] truncate"
            onClick={stop}
            onDoubleClick={stop}
          >
            {/* existing inline-edit name field — preserve the existing click→edit flow, just wrap the click handler in stopPropagation */}
          </div>
          <div className="flex gap-1.5 mt-1" onClick={stop} onDoubleClick={stop}>
            {/* existing priority/category popover buttons */}
          </div>
          <div
            className="mt-2 text-small text-[var(--color-text-muted)] line-clamp-3"
            onClick={stop}
            onDoubleClick={stop}
          >
            {/* existing evaluation inline-edit */}
          </div>
          {project.path && (
            <div className="mt-2 text-[11px] font-mono text-[var(--color-text-dim)] truncate">
              {project.path}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
```

Preserve every existing callback — the existing inline edit, popovers, and action buttons. The only structural changes: `onDoubleClick` at card root, `stopPropagation` on all interactive children, and the new path/evaluation block layout. The class `line-clamp-3` relies on Tailwind's line-clamp utility (already on v4, no plugin needed).

- [ ] **Step 2: Update ProjectList to pass the new prop**

Open `/Users/genie/dev/tools/hearth/src/components/ProjectList.tsx`. Add a new prop `onOpenDetail` to its signature, and plumb it through into every `<ProjectCard>` instance:

```tsx
type ProjectListProps = {
  // ...existing props
  onOpenDetail: (project: Project) => void;
};

// ...
<ProjectCard
  project={project}
  onUpdate={onUpdate}
  onDelete={onDelete}
  onOpenGhostty={onOpenGhostty}
  onOpenFinder={onOpenFinder}
  onOpenDetail={onOpenDetail}
/>
```

In the parent (`App.tsx` → `ProjectsTab`), add a placeholder that just logs for now — the real dialog wiring lands in Task 10:

```tsx
<ProjectList
  // ...existing props
  onOpenDetail={(p) => console.debug("open detail", p)}
/>
```

- [ ] **Step 3: Type check + manual verify card layout**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Confirm the list now renders detailed two-column-worthy cards (the 2-col grid itself comes in Task 11 with the unified DndContext — for now the card is taller with path/evaluation visible). Double-click a card → console shows `open detail { ... }`.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/ProjectCard.tsx src/components/ProjectList.tsx src/App.tsx && git commit -m "feat(projects): detailed card layout with double-click hook"
```

---

## Task 9: `ProjectDetailDialog` component

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/components/ProjectDetailDialog.tsx`

- [ ] **Step 1: Scaffold the dialog**

Create `/Users/genie/dev/tools/hearth/src/components/ProjectDetailDialog.tsx`:

```tsx
import { useEffect, useMemo, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { ProjectFormFields, type ProjectFormState } from "./ProjectFormFields";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import type { Project, Memo } from "../types";
import * as api from "../api";
import { useToast } from "../ui/Toast";

type Props = {
  open: boolean;
  project: Project | null;
  memos: Memo[];
  onClose: () => void;
  onProjectUpdated: () => void;
  onMemosChanged: () => void;
};

export function ProjectDetailDialog({
  open,
  project,
  memos,
  onClose,
  onProjectUpdated,
  onMemosChanged,
}: Props) {
  const [form, setForm] = useState<ProjectFormState>({
    name: "",
    priority: "P4",
    category: "",
    path: "",
    evaluation: "",
  });
  const [saving, setSaving] = useState(false);
  const [newMemoContent, setNewMemoContent] = useState("");
  const toast = useToast();

  const scopedMemos = useMemo(
    () => (project ? memos.filter((m) => m.project_id === project.id) : []),
    [memos, project],
  );

  useEffect(() => {
    if (!project) return;
    setForm({
      name: project.name,
      priority: project.priority,
      category: project.category ?? "",
      path: project.path ?? "",
      evaluation: project.evaluation ?? "",
    });
    setNewMemoContent("");
  }, [project]);

  if (!open || !project) return null;

  const handleSave = async () => {
    setSaving(true);
    try {
      await api.updateProject(project.id, {
        name: form.name.trim(),
        priority: form.priority,
        category: form.category === "" ? null : form.category,
        path: form.path.trim() || null,
        evaluation: form.evaluation.trim() || null,
      });
      onProjectUpdated();
      toast.success("프로젝트 저장됨");
      onClose();
    } catch (e) {
      toast.error(`저장 실패: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleAddMemo = async () => {
    const content = newMemoContent.trim();
    if (!content) return;
    try {
      await api.createMemo({ content, color: "yellow", project_id: project.id });
      setNewMemoContent("");
      onMemosChanged();
    } catch (e) {
      toast.error(`메모 생성 실패: ${e}`);
    }
  };

  const handleDeleteMemo = async (m: Memo) => {
    const yes = await ask("메모를 삭제할까요?", { title: "메모 삭제", kind: "warning" });
    if (!yes) return;
    try {
      await api.deleteMemo(m.id);
      onMemosChanged();
    } catch (e) {
      toast.error(`메모 삭제 실패: ${e}`);
    }
  };

  const handleInlineEdit = async (m: Memo, content: string) => {
    try {
      await api.updateMemo(m.id, { content });
      onMemosChanged();
    } catch (e) {
      toast.error(`메모 저장 실패: ${e}`);
    }
  };

  return (
    <Dialog open={open} onClose={onClose} title={project.name}>
      <div className="flex flex-col gap-6 min-w-[520px]">
        <section>
          <ProjectFormFields
            value={form}
            onChange={(patch) => setForm((prev) => ({ ...prev, ...patch }))}
          />
          <div className="mt-4 flex justify-end gap-2">
            <Button variant="ghost" onClick={onClose} disabled={saving}>취소</Button>
            <Button onClick={handleSave} disabled={saving || !form.name.trim()}>
              저장
            </Button>
          </div>
        </section>

        <section className="border-t border-[var(--color-surface-3)] pt-4">
          <div className="flex items-center justify-between mb-3">
            <div className="text-small text-[var(--color-text-muted)]">
              📝 연결 메모 ({scopedMemos.length})
            </div>
          </div>

          <div className="flex flex-col gap-2 max-h-[280px] overflow-y-auto pr-1">
            {scopedMemos.map((m) => (
              <MemoRow
                key={m.id}
                memo={m}
                onSave={(c) => handleInlineEdit(m, c)}
                onDelete={() => handleDeleteMemo(m)}
              />
            ))}
            {scopedMemos.length === 0 && (
              <div className="text-small text-[var(--color-text-dim)]">없음</div>
            )}
          </div>

          <div className="mt-3 flex gap-2">
            <textarea
              className="flex-1 rounded-md bg-[var(--color-surface-2)] px-2 py-1 text-small"
              rows={2}
              value={newMemoContent}
              onChange={(e) => setNewMemoContent(e.target.value)}
              placeholder="새 메모…"
            />
            <Button onClick={handleAddMemo} disabled={!newMemoContent.trim()}>
              추가
            </Button>
          </div>
        </section>
      </div>
    </Dialog>
  );
}

function MemoRow({ memo, onSave, onDelete }: { memo: Memo; onSave: (c: string) => void; onDelete: () => void }) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(memo.content);

  useEffect(() => { setDraft(memo.content); }, [memo.content]);

  return (
    <div className="flex items-start gap-2 rounded-md bg-[var(--color-surface-2)] px-2 py-1.5">
      <div className="flex-1">
        {editing ? (
          <textarea
            className="w-full rounded bg-[var(--color-surface-1)] px-1.5 py-1 text-small"
            autoFocus
            rows={2}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => { setEditing(false); if (draft !== memo.content) onSave(draft); }}
          />
        ) : (
          <button
            className="w-full text-left text-small text-[var(--color-text)] whitespace-pre-wrap"
            onClick={() => setEditing(true)}
          >
            {memo.content || <span className="text-[var(--color-text-dim)]">(비어 있음)</span>}
          </button>
        )}
      </div>
      <button
        className="text-small text-[var(--color-text-dim)] hover:text-[var(--color-text)]"
        onClick={onDelete}
      >
        ✕
      </button>
    </div>
  );
}
```

If `Dialog` / `Button` paths differ, correct imports against existing UI primitives (check `NewProjectDialog` for the canonical imports).

- [ ] **Step 2: Type check**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/ProjectDetailDialog.tsx && git commit -m "feat(projects): detail dialog with scoped memo CRUD"
```

---

## Task 10: Wire the detail dialog into the projects view

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/App.tsx` (or wherever `ProjectsTab` lives)
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectList.tsx` if necessary

- [ ] **Step 1: Add `detailProjectId` state**

In the component that owns `useProjects()` (most likely `ProjectsTab` inside `App.tsx`), add:

```tsx
import { useMemos } from "./hooks/useMemos";
import { ProjectDetailDialog } from "./components/ProjectDetailDialog";

// inside ProjectsTab:
const { memos, reload: reloadMemos } = useMemos();
const [detailProjectId, setDetailProjectId] = useState<number | null>(null);
const detailProject = useMemo(
  () => projects.find((p) => p.id === detailProjectId) ?? null,
  [projects, detailProjectId],
);
```

- [ ] **Step 2: Replace the placeholder `onOpenDetail` from Task 8**

```tsx
<ProjectList
  // ...existing props
  onOpenDetail={(p) => setDetailProjectId(p.id)}
/>
```

- [ ] **Step 3: Render the dialog**

Immediately after `<ProjectList … />` inside the same tab JSX:

```tsx
<ProjectDetailDialog
  open={detailProjectId !== null}
  project={detailProject}
  memos={memos}
  onClose={() => setDetailProjectId(null)}
  onProjectUpdated={() => { /* useProjects subscribes to projects:changed events already */ }}
  onMemosChanged={reloadMemos}
/>
```

- [ ] **Step 4: Handle project deletion while dialog is open**

Add an effect in `ProjectsTab` that closes the dialog if the target project disappears:

```tsx
useEffect(() => {
  if (detailProjectId !== null && !projects.some((p) => p.id === detailProjectId)) {
    setDetailProjectId(null);
  }
}, [projects, detailProjectId]);
```

- [ ] **Step 5: Type check + manual verify**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Double-click a project card → dialog opens with fields populated and scoped memos listed. Edit name → save → card reflects update. Add memo inside dialog → dialog memo list updates AND MemoBoard (if you switch tabs) shows the new memo.

- [ ] **Step 6: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/App.tsx && git commit -m "feat(projects): open detail dialog on card double-click"
```

---

## Task 11: Cross-priority drag-and-drop (unified DndContext + `deriveTarget`)

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/lib/dragTargets.ts`
- Test: `/Users/genie/dev/tools/hearth/src/lib/__tests__/dragTargets.test.ts`
- Create: `/Users/genie/dev/tools/hearth/src/components/EmptyDropZone.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/ProjectList.tsx`

- [ ] **Step 1: Write failing test for `deriveTarget`**

Create `/Users/genie/dev/tools/hearth/src/lib/__tests__/dragTargets.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { deriveTarget } from "../dragTargets";
import type { Project } from "../../types";

const mk = (id: number, priority: Project["priority"]): Project => ({
  id,
  priority,
  number: null,
  name: `p${id}`,
  category: null,
  path: null,
  evaluation: null,
  sort_order: 0,
  created_at: "",
  updated_at: "",
});

const projects: Project[] = [
  mk(1, "P0"), mk(2, "P0"),
  mk(3, "P2"),
];

describe("deriveTarget", () => {
  it("resolves a card id to its priority", () => {
    expect(deriveTarget(3, projects)).toEqual({ priority: "P2", overId: 3 });
  });

  it("resolves an empty-zone id to its priority with null overId", () => {
    expect(deriveTarget("priority-P4-empty", projects)).toEqual({
      priority: "P4", overId: null,
    });
  });

  it("returns null for an unknown id", () => {
    expect(deriveTarget(999, projects)).toBeNull();
    expect(deriveTarget("priority-PX-empty", projects)).toBeNull();
  });
});
```

- [ ] **Step 2: Run test to confirm it fails**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/lib/__tests__/dragTargets.test.ts
```
Expected: FAIL with `Cannot find module '../dragTargets'`.

- [ ] **Step 3: Implement `deriveTarget`**

Create `/Users/genie/dev/tools/hearth/src/lib/dragTargets.ts`:

```ts
import { PRIORITIES, type Priority, type Project } from "../types";

export type DragTarget = { priority: Priority; overId: number | null };

const EMPTY_ZONE = /^priority-(P[0-4])-empty$/;

export function deriveTarget(
  overId: string | number,
  projects: Project[],
): DragTarget | null {
  if (typeof overId === "string") {
    const m = EMPTY_ZONE.exec(overId);
    if (!m) return null;
    const priority = m[1] as Priority;
    if (!(PRIORITIES as readonly string[]).includes(priority)) return null;
    return { priority, overId: null };
  }
  const card = projects.find((p) => p.id === overId);
  if (!card) return null;
  return { priority: card.priority, overId };
}
```

- [ ] **Step 4: Run test to confirm it passes**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/lib/__tests__/dragTargets.test.ts
```
Expected: 3 passed.

- [ ] **Step 5: Create `EmptyDropZone` component**

Create `/Users/genie/dev/tools/hearth/src/components/EmptyDropZone.tsx`:

```tsx
import { useDroppable } from "@dnd-kit/core";

export function EmptyDropZone({ id, label }: { id: string; label: string }) {
  const { setNodeRef, isOver } = useDroppable({ id });
  return (
    <div
      ref={setNodeRef}
      className={
        "rounded-lg border-2 border-dashed px-3 py-4 text-small text-[var(--color-text-dim)] text-center transition-colors " +
        (isOver
          ? "border-[var(--color-brand)] bg-[var(--color-brand-soft)]"
          : "border-[var(--color-surface-3)]")
      }
    >
      {label}
    </div>
  );
}
```

- [ ] **Step 6: Replace per-group DndContext with unified one in ProjectList**

Open `/Users/genie/dev/tools/hearth/src/components/ProjectList.tsx`. Replace the `.map(([priority, items]))` block that renders five separate `DndContext`s with a single wrapper:

```tsx
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  closestCorners,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { useState } from "react";
import { deriveTarget } from "../lib/dragTargets";
import { EmptyDropZone } from "./EmptyDropZone";
import { PRIORITIES, type Priority, type Project } from "../types";
import { useToast } from "../ui/Toast";

// inside ProjectList:
const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));
const [activeId, setActiveId] = useState<number | null>(null);
const toast = useToast();

const projectById = (id: number | null) =>
  id === null ? undefined : projects.find((p) => p.id === id);

const idsOf = (priority: Priority) =>
  (groups.get(priority) ?? []).map((p) => p.id);

const handleDragStart = (e: DragStartEvent) => {
  if (typeof e.active.id === "number") setActiveId(e.active.id);
};

const handleDragEnd = async (e: DragEndEvent) => {
  setActiveId(null);
  const { active, over } = e;
  if (!over) return;
  const activeNumId = typeof active.id === "number" ? active.id : Number(active.id);
  const target = deriveTarget(over.id, projects);
  if (!target) return;
  const source = projectById(activeNumId);
  if (!source) return;

  if (source.priority === target.priority) {
    if (activeNumId === over.id) return;
    const groupIds = idsOf(target.priority);
    const from = groupIds.indexOf(activeNumId);
    const to = target.overId === null ? groupIds.length - 1 : groupIds.indexOf(target.overId);
    if (from < 0 || to < 0) return;
    const next = [...groupIds];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    try {
      await onReorder(target.priority, next);
    } catch (err) {
      toast.error(`순서 저장 실패: ${err}`);
    }
    return;
  }

  // Cross-group move: serialize three writes so the next refetch sees canonical state.
  const targetGroupIds = idsOf(target.priority);
  const sourceGroupIds = idsOf(source.priority);
  const insertAt = target.overId === null ? targetGroupIds.length : targetGroupIds.indexOf(target.overId);
  const nextTargetIds = [...targetGroupIds];
  nextTargetIds.splice(Math.max(0, insertAt), 0, activeNumId);
  const nextSourceIds = sourceGroupIds.filter((id) => id !== activeNumId);

  try {
    await onUpdate(activeNumId, { priority: target.priority });
    await onReorder(target.priority, nextTargetIds);
    await onReorder(source.priority, nextSourceIds);
  } catch (err) {
    toast.error(`우선순위 변경 실패: ${err}`);
  }
};

const activeProject = projectById(activeId);

return (
  <DndContext
    sensors={sensors}
    collisionDetection={closestCorners}
    onDragStart={handleDragStart}
    onDragEnd={handleDragEnd}
  >
    <div className="flex flex-col gap-4">
      {PRIORITIES.map((priority) => {
        const items = groups.get(priority) ?? [];
        return (
          <div key={priority}>
            <div className="mb-2 flex items-center gap-2 text-small text-[var(--color-text-muted)]">
              <span className={`w-2 h-2 rounded-full bg-[var(--color-${priority.toLowerCase()})]`} />
              {priority} ({items.length})
            </div>
            <SortableContext items={items.map((p) => p.id)} strategy={rectSortingStrategy}>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                {items.map((project) => (
                  <ProjectCard
                    key={project.id}
                    project={project}
                    onUpdate={onUpdate}
                    onDelete={onDelete}
                    onOpenGhostty={onOpenGhostty}
                    onOpenFinder={onOpenFinder}
                    onOpenDetail={onOpenDetail}
                  />
                ))}
                {items.length === 0 && (
                  <EmptyDropZone id={`priority-${priority}-empty`} label={`${priority} 비어 있음 · 드래그해서 추가`} />
                )}
              </div>
            </SortableContext>
          </div>
        );
      })}
    </div>

    <DragOverlay>
      {activeProject ? (
        <div className="rounded-lg border border-[var(--color-brand)] bg-[var(--color-surface-1)] px-3 py-2.5 shadow-[var(--shadow-e3)] text-body text-[var(--color-text-hi)]">
          {activeProject.name}
        </div>
      ) : null}
    </DragOverlay>
  </DndContext>
);
```

Remove the old `handleDragEnd(priority)` factory — it's replaced by the single `handleDragEnd`. Verify `onReorder` still has the signature `(priority, ids) => void` matching `useProjects.reorder`. `onUpdate` signature matches `updateProject`'s partial input.

- [ ] **Step 7: Type check**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

- [ ] **Step 8: Manual verification**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

Do each of the following and confirm DB persistence via the UI refresh:

1. Drag a P0 card inside P0 — reorders within group.
2. Drag a P0 card into the middle of P2 — card appears in P2 at that position, priority badge shows `P2`.
3. Drag a card into P4 (which is empty) using the "비어 있음" drop zone — lands there.
4. Reload app (`Cmd+R` inside devtools or close/reopen) — order persists.

- [ ] **Step 9: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/lib/dragTargets.ts src/lib/__tests__/dragTargets.test.ts src/components/EmptyDropZone.tsx src/components/ProjectList.tsx && git commit -m "feat(projects): cross-priority drag with unified DndContext"
```

---

## Task 12: MemoBoard grouping + global `#N` badges

**Files:**
- Create: `/Users/genie/dev/tools/hearth/src/lib/memoSequence.ts`
- Test: `/Users/genie/dev/tools/hearth/src/lib/__tests__/memoSequence.test.ts`
- Modify: `/Users/genie/dev/tools/hearth/src/components/MemoCard.tsx`
- Modify: `/Users/genie/dev/tools/hearth/src/components/MemoBoard.tsx`

- [ ] **Step 1: Write failing test for `globalSequence`**

Create `/Users/genie/dev/tools/hearth/src/lib/__tests__/memoSequence.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { globalSequence, groupMemosByProject } from "../memoSequence";
import type { Memo, Project } from "../../types";

const mkMemo = (id: number, sort_order: number, project_id: number | null = null): Memo => ({
  id, sort_order, project_id,
  content: `memo${id}`, color: "yellow",
  created_at: "", updated_at: "",
});

const mkProj = (id: number, priority: Project["priority"], sort_order = 0): Project => ({
  id, priority, number: null, name: `p${id}`, category: null, path: null, evaluation: null,
  sort_order, created_at: "", updated_at: "",
});

describe("globalSequence", () => {
  it("assigns 1..N by sort_order", () => {
    const memos = [mkMemo(10, 2), mkMemo(20, 0), mkMemo(30, 1)];
    const seq = globalSequence(memos);
    expect(seq.get(20)).toBe(1);
    expect(seq.get(30)).toBe(2);
    expect(seq.get(10)).toBe(3);
  });

  it("is stable when memos are already ordered", () => {
    const memos = [mkMemo(1, 0), mkMemo(2, 1), mkMemo(3, 2)];
    const seq = globalSequence(memos);
    expect([...seq.entries()]).toEqual([[1, 1], [2, 2], [3, 3]]);
  });
});

describe("groupMemosByProject", () => {
  it("groups by project priority then sort_order, with 기타 trailing", () => {
    const projects = [
      mkProj(1, "P2", 0),
      mkProj(2, "P0", 0),
      mkProj(3, "P0", 1),
    ];
    const memos = [
      mkMemo(10, 0, 1),
      mkMemo(11, 1, 2),
      mkMemo(12, 2, 3),
      mkMemo(13, 3, null),
    ];
    const groups = groupMemosByProject(memos, projects);
    expect(groups.map((g) => g.kind === "project" ? g.project.id : "etc")).toEqual([2, 3, 1, "etc"]);
    expect(groups[0].memos.map((m) => m.id)).toEqual([11]);
    expect(groups[3].memos.map((m) => m.id)).toEqual([13]);
  });

  it("omits empty groups", () => {
    const projects = [mkProj(1, "P0", 0), mkProj(2, "P1", 0)];
    const memos = [mkMemo(10, 0, 1)];
    const groups = groupMemosByProject(memos, projects);
    expect(groups).toHaveLength(1);
    expect(groups[0].kind).toBe("project");
  });
});
```

- [ ] **Step 2: Run test — should fail**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/lib/__tests__/memoSequence.test.ts
```
Expected: FAIL with `Cannot find module '../memoSequence'`.

- [ ] **Step 3: Implement helpers**

Create `/Users/genie/dev/tools/hearth/src/lib/memoSequence.ts`:

```ts
import type { Memo, Project, Priority } from "../types";

const PRIORITY_ORDER: Record<Priority, number> = {
  P0: 0, P1: 1, P2: 2, P3: 3, P4: 4,
};

export type MemoGroup =
  | { kind: "project"; project: Project; memos: Memo[] }
  | { kind: "etc"; memos: Memo[] };

export function globalSequence(memos: Memo[]): Map<number, number> {
  return new Map(
    [...memos]
      .sort((a, b) => a.sort_order - b.sort_order)
      .map((m, i) => [m.id, i + 1]),
  );
}

export function groupMemosByProject(memos: Memo[], projects: Project[]): MemoGroup[] {
  const byProject = new Map<number, Memo[]>();
  const etc: Memo[] = [];

  const sortedMemos = [...memos].sort((a, b) => a.sort_order - b.sort_order);
  for (const m of sortedMemos) {
    if (m.project_id === null || m.project_id === undefined) {
      etc.push(m);
    } else {
      const list = byProject.get(m.project_id) ?? [];
      list.push(m);
      byProject.set(m.project_id, list);
    }
  }

  const orderedProjects = [...projects].sort((a, b) => {
    const pd = PRIORITY_ORDER[a.priority] - PRIORITY_ORDER[b.priority];
    if (pd !== 0) return pd;
    return a.sort_order - b.sort_order;
  });

  const groups: MemoGroup[] = [];
  for (const p of orderedProjects) {
    const list = byProject.get(p.id);
    if (list && list.length > 0) {
      groups.push({ kind: "project", project: p, memos: list });
    }
  }
  if (etc.length > 0) groups.push({ kind: "etc", memos: etc });
  return groups;
}
```

- [ ] **Step 4: Run test — should pass**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npm test -- src/lib/__tests__/memoSequence.test.ts
```
Expected: 4 passed.

- [ ] **Step 5: Extend MemoCard with `sequenceNumber` prop**

Open `/Users/genie/dev/tools/hearth/src/components/MemoCard.tsx`. Add to the props type:

```tsx
type MemoCardProps = {
  // ...existing
  sequenceNumber: number;
};
```

In the rendered JSX, add the badge inside the root card element (after the drag handle, before the content):

```tsx
<span className="absolute top-1.5 right-2 rounded-full bg-black/25 text-white px-1.5 py-[1px] text-[10px] font-semibold">
  #{sequenceNumber}
</span>
```

Ensure the root element has `position: relative` — it already does (sticky-note visuals require it), but confirm by searching for `relative` in that component.

- [ ] **Step 6: Rewrite MemoBoard render with groups**

Open `/Users/genie/dev/tools/hearth/src/components/MemoBoard.tsx`. Replace the current flat render with grouped render:

```tsx
import { useMemo } from "react";
import { DndContext, closestCenter, DragOverlay, PointerSensor, useSensor, useSensors, type DragEndEvent, type DragStartEvent } from "@dnd-kit/core";
import { SortableContext, rectSortingStrategy } from "@dnd-kit/sortable";
import { MemoCard } from "./MemoCard";
import { EmptyDropZone } from "./EmptyDropZone";
import { globalSequence, groupMemosByProject } from "../lib/memoSequence";
import { useProjects } from "../hooks/useProjects";
import { useMemos } from "../hooks/useMemos";

export function MemoBoard() {
  const { memos, update, remove, reorder } = useMemos();
  const { projects } = useProjects();

  const groups = useMemo(() => groupMemosByProject(memos, projects), [memos, projects]);
  const seq = useMemo(() => globalSequence(memos), [memos]);

  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  // onDragEnd logic lands in Task 13 — for now, keep existing within-group reorder behavior.
  const handleDragEnd = async (_e: DragEndEvent) => {
    // Placeholder; replaced in Task 13.
  };

  return (
    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
      <div className="flex flex-col gap-5">
        {groups.map((g) => {
          const key = g.kind === "project" ? `proj-${g.project.id}` : "etc";
          const title = g.kind === "project"
            ? `${g.project.name} · ${g.project.priority}`
            : "기타 · 프로젝트 미연결";
          return (
            <section key={key}>
              <header className="mb-2 flex items-center gap-2 text-small text-[var(--color-text-muted)] border-b border-[var(--color-surface-3)] pb-1">
                <span>{title}</span>
                <span className="text-[var(--color-text-dim)]">({g.memos.length})</span>
              </header>
              <SortableContext items={g.memos.map((m) => m.id)} strategy={rectSortingStrategy}>
                <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-4 gap-2">
                  {g.memos.map((m) => (
                    <MemoCard
                      key={m.id}
                      memo={m}
                      projects={projects}
                      onUpdate={update}
                      onDelete={remove}
                      sequenceNumber={seq.get(m.id) ?? 0}
                    />
                  ))}
                </div>
              </SortableContext>
            </section>
          );
        })}

        {groups.length === 0 && (
          <div className="text-small text-[var(--color-text-dim)]">메모가 없습니다.</div>
        )}
      </div>

      <DragOverlay />
    </DndContext>
  );
}
```

`reorder` is imported but unused in this task — Task 13 wires it.

- [ ] **Step 7: Type check + manual verify**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Switch to the Memos tab. Confirm: memos group under project headers (P0 first, then P1…), "기타" group appears at the bottom for unassigned memos, each memo shows a `#N` badge in the top-right corner, badge numbers match global order when you sort by creation time.

- [ ] **Step 8: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/lib/memoSequence.ts src/lib/__tests__/memoSequence.test.ts src/components/MemoCard.tsx src/components/MemoBoard.tsx && git commit -m "feat(memos): project grouping + global #N badges"
```

---

## Task 13: Cross-group memo drag

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/components/MemoBoard.tsx`

- [ ] **Step 1: Implement full handleDragEnd**

Replace the placeholder `handleDragEnd` from Task 12 with:

```tsx
import { useMemo, useState } from "react";  // extend existing useMemo import
import * as api from "../api";
import { useToast } from "../ui/Toast";

// inside MemoBoard:
const toast = useToast();
const [activeId, setActiveId] = useState<number | null>(null);

const handleDragStart = (e: DragStartEvent) => {
  if (typeof e.active.id === "number") setActiveId(e.active.id);
};

const handleDragEnd = async (e: DragEndEvent) => {
  setActiveId(null);
  const { active, over } = e;
  if (!over || active.id === over.id) return;
  const activeIdNum = typeof active.id === "number" ? active.id : Number(active.id);
  const overIdNum = typeof over.id === "number" ? over.id : NaN;
  const sourceMemo = memos.find((m) => m.id === activeIdNum);
  const targetMemo = memos.find((m) => m.id === overIdNum);
  if (!sourceMemo || !targetMemo) return;

  const sameGroup = sourceMemo.project_id === targetMemo.project_id;

  // Build flat ordered id list by rebuilding groups after move.
  const nextMemos: typeof memos = memos.map((m) =>
    m.id === sourceMemo.id ? { ...m, project_id: targetMemo.project_id ?? null } : m,
  );

  // Reorder within the target group.
  const targetGroupIds = nextMemos
    .filter((m) => m.project_id === targetMemo.project_id)
    .sort((a, b) => a.sort_order - b.sort_order)
    .map((m) => m.id);
  const fromIdx = targetGroupIds.indexOf(sourceMemo.id);
  const toIdx = targetGroupIds.indexOf(targetMemo.id);
  if (fromIdx >= 0 && toIdx >= 0) {
    const [moved] = targetGroupIds.splice(fromIdx, 1);
    targetGroupIds.splice(toIdx, 0, moved);
  }

  // Rebuild full flat order: groups in their display order, each group using its new ordered ids.
  const groupsAfter = groupMemosByProject(nextMemos, projects);
  const fullIds: number[] = [];
  for (const g of groupsAfter) {
    const gIds = g.memos.map((m) => m.id);
    if (g.kind === "project" && g.project.id === (targetMemo.project_id ?? -999)) {
      fullIds.push(...targetGroupIds);
    } else if (g.kind === "etc" && targetMemo.project_id === null) {
      fullIds.push(...targetGroupIds);
    } else {
      fullIds.push(...gIds);
    }
  }

  try {
    if (!sameGroup) {
      await api.updateMemo(sourceMemo.id, { project_id: targetMemo.project_id ?? undefined });
    }
    await api.reorderMemos(fullIds);
  } catch (err) {
    toast.error(`메모 이동 실패: ${err}`);
  }
};
```

Wire the new handlers into the `<DndContext>` props:

```tsx
<DndContext
  sensors={sensors}
  collisionDetection={closestCenter}
  onDragStart={handleDragStart}
  onDragEnd={handleDragEnd}
>
```

Add a DragOverlay render of the active memo for visual continuity:

```tsx
<DragOverlay>
  {activeId !== null
    ? (() => {
        const m = memos.find((x) => x.id === activeId);
        return m ? (
          <div className="rounded-md bg-[var(--color-surface-1)] border border-[var(--color-brand)] px-3 py-2 text-small text-[var(--color-text-hi)] shadow-[var(--shadow-e3)]">
            {m.content.slice(0, 60)}
          </div>
        ) : null;
      })()
    : null}
</DragOverlay>
```

If `api.updateMemo` does not accept `project_id: undefined` to mean "unchanged" — inspect its signature in `src/api.ts`; if it requires `null` for unset, pass `targetMemo.project_id ?? null` instead.

- [ ] **Step 2: Type check + manual verify**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Drag a memo from one project group onto a memo in another project group → the memo re-parents, `#N` badges renumber. Drag within a group → reorders and badges shift accordingly.

- [ ] **Step 3: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/components/MemoBoard.tsx && git commit -m "feat(memos): cross-group drag re-parents + global reorder"
```

---

## Task 14: System prompt `[메모]` section

**Files:**
- Modify: `/Users/genie/dev/tools/hearth/src/command/buildSystemPrompt.ts`

- [ ] **Step 1: Add new section**

Open `/Users/genie/dev/tools/hearth/src/command/buildSystemPrompt.ts`. After the existing "변경" (Mutation) block and before Rule 1, insert:

```ts
lines.push("");
lines.push("[메모 처리 규칙]");
lines.push("- 메모는 프로젝트별 그룹 + 맨 아래 '기타' 그룹으로 표시됩니다.");
lines.push("- 각 메모는 전역 sort_order 기준 #1, #2 … 뱃지를 갖습니다.");
lines.push("- 새 메모 생성: create_memo(content, project_name?). project_name 은 이름 부분 일치 (LIKE) 로 해석되며, 매칭되지 않으면 '기타'로 저장됩니다. 기타로 저장된 경우 사용자에게 명시하세요.");
lines.push("- 메모 내용 수정: update_memo_by_number(number, content). number 는 현재 화면의 #N.");
lines.push("- 메모 삭제: delete_memo_by_number(number).");
lines.push("- #N 은 스냅샷 식별자이므로, 작업 전에 최신 메모 목록을 조회(list_memos)한 뒤 번호를 확정하세요.");
```

Keep the remainder of the function unchanged. If the file uses different variable names for the string buffer, adapt accordingly.

- [ ] **Step 2: Type check**

Run:
```bash
cd /Users/genie/dev/tools/hearth && npx tsc --noEmit
```
Expected: PASS.

- [ ] **Step 3: Manual verify**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```
Open `Cmd+K`, type `"WithGenieLMS 에 '심사 통과' 메모 추가"` → confirm → expect a `create_memo` tool call with `project_name: "WithGenieLMS"`, memo appears in that project group. Then type `"#5 메모 삭제"` → confirm → memo row removed.

- [ ] **Step 4: Commit**

```bash
cd /Users/genie/dev/tools/hearth && git add src/command/buildSystemPrompt.ts && git commit -m "feat(ai): system prompt documents memo #N semantics"
```

---

## Task 15: End-to-end manual verification

- [ ] **Step 1: Run final manual checklist (from spec §Testing)**

```bash
cd /Users/genie/dev/tools/hearth && npm run tauri dev
```

Exercise each case and record the outcome:

- [ ] Drag a P0 card into the middle of P2, confirm DB `priority` + `sort_order`:
  ```bash
  # Hearth keeps its SQLite under the Tauri app-data dir; bundle id lives in src-tauri/tauri.conf.json.
  DB=$(fd -p "data.db" ~/Library/Application\ Support | rg -i hearth | head -1)
  sqlite3 "$DB" "SELECT id, name, priority, sort_order FROM projects ORDER BY priority, sort_order;"
  ```
- [ ] `Cmd+=` × 3 → scale is `1.3` (clamped). `Cmd+-` × 2 → `1.0`. `Cmd+0` → `1.0`.
- [ ] Double-click card → edit name/evaluation in modal → save → card reflects change.
- [ ] Add three memos inside the modal → switch to Memos tab → they appear in the matching project group with `#N` badges.
- [ ] `"WithGenieLMS 에 '심사 통과' 메모 추가"` through `Cmd+K` → confirm → memo appears in WithGenieLMS group.
- [ ] `"#5 메모 삭제"` through `Cmd+K` → confirm → memo removed and subsequent badges renumber.
- [ ] Drag a memo from WithGenieLMS group onto a PickAtSoul memo → memo re-parents, `#N` stays consistent globally.
- [ ] Close + reopen the app — UI scale, project order, memo order all persist.

- [ ] **Step 2: Tag commit (optional)**

If all pass:

```bash
cd /Users/genie/dev/tools/hearth && git tag -a projects-view-enhancements-complete -m "All projects view enhancements tasks green"
```
