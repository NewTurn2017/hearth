# Tool Calling Integration Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 12 integration tests against OpenAI `gpt-5.4-mini` that verify the model picks correct tools and extracts sane arguments given realistic Korean prompts against the hearth tool registry.

**Architecture:** One new integration test file at `src-tauri/tests/tool_calling_integration.rs`. A self-contained `ask()` helper builds the same OpenAI request shape `run_agent` uses (tools from `ai_tools::specs()`, bearer auth, `max_completion_tokens=8192`, no `temperature` override), POSTs to `/v1/chat/completions`, parses `tool_calls[]`, returns them to assertions. All tests `#[ignore]` so default `cargo test` stays offline.

**Tech Stack:** Rust, `reqwest` (already a dep), `tokio` (already `features = ["full"]`), `serde_json`, `chrono`. Reuses `tauri_app_lib::ai_tools::{specs, kind_of, ToolKind}`.

**Spec:** `docs/superpowers/specs/2026-04-16-tool-calling-integration-tests-design.md`

---

## File Structure

- **Modify** `src-tauri/src/lib.rs:1` — change `mod ai_tools;` to `pub mod ai_tools;` so integration tests can reach `specs()` and `kind_of()`.
- **Create** `src-tauri/tests/tool_calling_integration.rs` — single file housing helpers + 12 tests.

No other production code changes.

---

### Task 1: Expose `ai_tools` module to integration tests

**Files:**
- Modify: `src-tauri/src/lib.rs:1`

- [ ] **Step 1: Change the module declaration**

In `src-tauri/src/lib.rs`, change line 1 from:

```rust
mod ai_tools;
```

to:

```rust
pub mod ai_tools;
```

- [ ] **Step 2: Verify the workspace still builds**

Run: `cd src-tauri && cargo check`
Expected: clean build, no new warnings. `ai_tools` is now a public module of `tauri_app_lib`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "chore(ai): expose ai_tools module for integration tests"
```

---

### Task 2: Scaffold test file with `ask()` helper + smoke test (1-1)

**Files:**
- Create: `src-tauri/tests/tool_calling_integration.rs`

- [ ] **Step 1: Create the file with the helper and the first test**

Write `src-tauri/tests/tool_calling_integration.rs` with the following exact contents:

```rust
//! Integration tests for OpenAI tool-calling against the hearth tool
//! registry. All tests are `#[ignore]` because they hit the real OpenAI API
//! ($~0.02 per full run) and require `OPENAI_API_KEY`.
//!
//! Run with:
//!   OPENAI_API_KEY=sk-... cargo test --test tool_calling_integration \
//!       -- --ignored --test-threads=1
//!
//! Design: docs/superpowers/specs/2026-04-16-tool-calling-integration-tests-design.md

use chrono::{Duration, Local};
use serde_json::{json, Value};
use tauri_app_lib::ai_tools::{kind_of, specs, ToolKind};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const OPENAI_MODEL: &str = "gpt-5.4-mini";
const OPENAI_MAX_COMPLETION_TOKENS: u32 = 8192;

/// Minimal system prompt inlined so these tests don't drift with UI prompt
/// changes. Mirrors the production palette prompt's core rules.
const SYSTEM_PROMPT: &str = "\
당신은 Hearth의 AI 어시스턴트입니다. 사용자의 프로젝트, 메모, 일정을 \
관리합니다. 사용자의 요청을 처리할 때 적절한 tool을 호출하세요. 단순한 인사, \
감사, 잡담이나 정보가 부족한 모호한 요청에는 tool을 호출하지 말고 한국어로 \
답하세요. 기본 응답 언어는 한국어입니다.";

#[derive(Debug)]
struct ParsedToolCall {
    name: String,
    args: Value,
}

#[derive(Debug)]
struct LlmResponse {
    tool_calls: Vec<ParsedToolCall>,
    content: String,
}

impl LlmResponse {
    fn has_tool(&self, name: &str) -> bool {
        self.tool_calls.iter().any(|c| c.name == name)
    }

    fn first_with(&self, name: &str) -> Option<&ParsedToolCall> {
        self.tool_calls.iter().find(|c| c.name == name)
    }
}

async fn ask(user_prompt: &str) -> LlmResponse {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY required for integration tests");

    let tools_json: Vec<Value> = specs()
        .into_iter()
        .map(|s| {
            json!({
                "type": "function",
                "function": {
                    "name": s.name,
                    "description": s.description,
                    "parameters": s.parameters,
                }
            })
        })
        .collect();

    let body = json!({
        "model": OPENAI_MODEL,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": user_prompt }
        ],
        "tools": tools_json,
        "max_completion_tokens": OPENAI_MAX_COMPLETION_TOKENS,
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(OPENAI_CHAT_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .expect("OpenAI request failed");

    let status = resp.status();
    let text = resp.text().await.expect("read body failed");
    assert!(status.is_success(), "OpenAI {}: {}", status, text);

    let data: Value = serde_json::from_str(&text).expect("parse body failed");
    let msg = &data["choices"][0]["message"];
    let content = msg["content"].as_str().unwrap_or("").to_string();

    let tool_calls = msg["tool_calls"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| {
                    let name = c["function"]["name"].as_str()?.to_string();
                    let args_str = c["function"]["arguments"].as_str().unwrap_or("{}");
                    let args: Value = serde_json::from_str(args_str)
                        .unwrap_or(Value::Object(Default::default()));
                    Some(ParsedToolCall { name, args })
                })
                .collect()
        })
        .unwrap_or_default();

    LlmResponse { tool_calls, content }
}

// ---------- Scenario 1: simple tool selection ----------

#[tokio::test]
#[ignore]
async fn t1_1_list_projects() {
    let r = ask("프로젝트 목록 보여줘").await;
    assert!(
        r.has_tool("list_projects"),
        "expected list_projects, got {:?} (content: {})",
        r.tool_calls,
        r.content
    );
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo test --test tool_calling_integration --no-run`
Expected: compiles cleanly. (No tests run yet — `--no-run` just builds.)

- [ ] **Step 3: Run the smoke test**

Run: `cd src-tauri && OPENAI_API_KEY=$OPENAI_API_KEY cargo test --test tool_calling_integration t1_1_list_projects -- --ignored --nocapture`

Expected: `test t1_1_list_projects ... ok` with 1 passed.

If it fails because of missing key, stop and ask the user to export `OPENAI_API_KEY`. If it fails because the model chose a different tool, print the `r.tool_calls` dump and consider whether to rephrase the prompt before proceeding — but a prompt as clear as "프로젝트 목록 보여줘" should map cleanly.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/tool_calling_integration.rs
git commit -m "test(ai): integration harness + list_projects smoke test"
```

---

### Task 3: Add remaining 11 tests

**Files:**
- Modify: `src-tauri/tests/tool_calling_integration.rs` (append after existing test)

- [ ] **Step 1: Append remaining scenario-1 tests (1-2, 1-3, 1-4)**

Append after `t1_1_list_projects`:

```rust
#[tokio::test]
#[ignore]
async fn t1_2_list_memos() {
    let r = ask("메모 다 보여줘").await;
    assert!(
        r.has_tool("list_memos"),
        "expected list_memos, got {:?} (content: {})",
        r.tool_calls,
        r.content
    );
}

#[tokio::test]
#[ignore]
async fn t1_3_switch_tab_calendar() {
    let r = ask("달력 탭으로 가줘").await;
    let call = r
        .first_with("switch_tab")
        .unwrap_or_else(|| panic!("expected switch_tab, got {:?} (content: {})", r.tool_calls, r.content));
    assert_eq!(
        call.args["tab"].as_str(),
        Some("calendar"),
        "expected tab=calendar, got args {:?}",
        call.args
    );
}

#[tokio::test]
#[ignore]
async fn t1_4_set_filter_priority() {
    let r = ask("P0 우선순위만 필터해줘").await;
    let call = r
        .first_with("set_filter")
        .unwrap_or_else(|| panic!("expected set_filter, got {:?} (content: {})", r.tool_calls, r.content));
    let priorities = call.args["priorities"]
        .as_array()
        .unwrap_or_else(|| panic!("expected priorities array, got args {:?}", call.args));
    assert!(
        priorities.iter().any(|v| v.as_str() == Some("P0")),
        "expected priorities to contain P0, got {:?}",
        priorities
    );
}
```

- [ ] **Step 2: Append scenario-2 tests (2-1, 2-2, 2-3)**

Append below the scenario-1 block:

```rust
// ---------- Scenario 2: argument extraction ----------

#[tokio::test]
#[ignore]
async fn t2_1_create_schedule_tomorrow_3pm() {
    let r = ask("내일 오후 3시에 치과 예약 추가해줘").await;
    let call = r
        .first_with("create_schedule")
        .unwrap_or_else(|| panic!("expected create_schedule, got {:?} (content: {})", r.tool_calls, r.content));

    let expected_date = (Local::now() + Duration::days(1)).format("%Y-%m-%d").to_string();
    assert_eq!(
        call.args["date"].as_str(),
        Some(expected_date.as_str()),
        "expected date={}, got args {:?}",
        expected_date,
        call.args
    );

    let time = call.args["time"].as_str().unwrap_or("");
    assert!(
        time == "15:00" || time.starts_with("15:"),
        "expected time ~ 15:00 (24h), got {:?} in args {:?}",
        time,
        call.args
    );

    let text_fields = [
        call.args["description"].as_str().unwrap_or(""),
        call.args["notes"].as_str().unwrap_or(""),
        call.args["location"].as_str().unwrap_or(""),
    ];
    assert!(
        text_fields.iter().any(|s| s.contains("치과")),
        "expected 치과 in description/notes/location, got args {:?}",
        call.args
    );
}

#[tokio::test]
#[ignore]
async fn t2_2_create_project_named_webapp() {
    let r = ask("'WebApp' 이름으로 프로젝트 만들어줘").await;
    let call = r
        .first_with("create_project")
        .unwrap_or_else(|| panic!("expected create_project, got {:?} (content: {})", r.tool_calls, r.content));
    assert_eq!(
        call.args["name"].as_str(),
        Some("WebApp"),
        "expected name=WebApp, got args {:?}",
        call.args
    );
}

#[tokio::test]
#[ignore]
async fn t2_3_create_memo_content_nonempty() {
    let r = ask("오늘 회의록 메모 추가해줘").await;
    let call = r
        .first_with("create_memo")
        .unwrap_or_else(|| panic!("expected create_memo, got {:?} (content: {})", r.tool_calls, r.content));
    let content = call.args["content"].as_str().unwrap_or("");
    assert!(
        !content.trim().is_empty(),
        "expected non-empty content, got args {:?}",
        call.args
    );
}
```

- [ ] **Step 3: Append scenario-3 tests (3-1, 3-2)**

Append below scenario-2:

```rust
// ---------- Scenario 3: mutation classification ----------

#[tokio::test]
#[ignore]
async fn t3_1_new_project_is_mutation() {
    let r = ask("새 프로젝트 'Test123' 만들어").await;
    let first = r
        .tool_calls
        .first()
        .unwrap_or_else(|| panic!("expected a tool call, got none (content: {})", r.content));
    assert_eq!(
        kind_of(&first.name),
        Some(ToolKind::Mutation),
        "expected {} to be Mutation, got {:?}",
        first.name,
        kind_of(&first.name)
    );
}

#[tokio::test]
#[ignore]
async fn t3_2_delete_memo_is_mutation() {
    let r = ask("메모 id 5번 삭제해줘").await;
    let first = r
        .tool_calls
        .first()
        .unwrap_or_else(|| panic!("expected a tool call, got none (content: {})", r.content));
    assert!(
        first.name.starts_with("delete_"),
        "expected delete_* tool, got {} (args {:?})",
        first.name,
        first.args
    );
    assert_eq!(
        kind_of(&first.name),
        Some(ToolKind::Mutation),
        "expected Mutation classification for {}",
        first.name
    );
}
```

- [ ] **Step 4: Append scenario-6 + 7 tests (6-1, 6-2, 7-1)**

Append below scenario-3:

```rust
// ---------- Scenario 6: no-tool chitchat ----------

#[tokio::test]
#[ignore]
async fn t6_1_greeting_no_tool() {
    let r = ask("안녕!").await;
    assert!(
        r.tool_calls.is_empty(),
        "expected no tool calls for greeting, got {:?}",
        r.tool_calls
    );
    assert!(
        !r.content.trim().is_empty(),
        "expected a text reply for greeting"
    );
}

#[tokio::test]
#[ignore]
async fn t6_2_thanks_no_tool() {
    let r = ask("고마워").await;
    assert!(
        r.tool_calls.is_empty(),
        "expected no tool calls for thanks, got {:?}",
        r.tool_calls
    );
}

// ---------- Scenario 7: ambiguity safety ----------

#[tokio::test]
#[ignore]
async fn t7_1_vague_delete_no_destructive_call() {
    let r = ask("그거 삭제해줘").await;
    let destructive = r
        .tool_calls
        .iter()
        .any(|c| c.name.starts_with("delete_") || c.name.starts_with("update_"));
    assert!(
        !destructive,
        "expected model to NOT call delete_/update_ on vague request, got {:?} (content: {})",
        r.tool_calls,
        r.content
    );
}
```

- [ ] **Step 5: Verify the file still compiles**

Run: `cd src-tauri && cargo test --test tool_calling_integration --no-run`
Expected: clean build.

- [ ] **Step 6: Run the full suite**

Run: `cd src-tauri && OPENAI_API_KEY=$OPENAI_API_KEY cargo test --test tool_calling_integration -- --ignored --test-threads=1 --nocapture`

Expected: 12 passed, 0 failed. Total runtime ~20-40s, cost ~$0.02.

If a test fails, read the `eprintln`-style assertion message — it shows the full `tool_calls` array and `content` the model returned. Decide: (a) rephrase the prompt more unambiguously, (b) relax the assertion (e.g., accept a synonym tool), or (c) note a legitimate model regression and file it.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/tests/tool_calling_integration.rs
git commit -m "test(ai): cover arg extraction, mutation class, no-tool, ambiguity"
```

---

## Self-Review Checklist (for implementer)

Before declaring done:

1. All 12 tests present with `#[ignore]` attribute
2. No test references a tool name not in `ai_tools::specs()` output (spot-check: `list_projects`, `list_memos`, `switch_tab`, `set_filter`, `create_schedule`, `create_project`, `create_memo`, `delete_memo`)
3. Full suite passes twice in a row (catches flakes)
4. `cargo check` clean — no new warnings introduced
5. `mod ai_tools` in `lib.rs` is `pub mod ai_tools`
