# Tool Calling Integration Tests — Design

## Purpose

Verify that the OpenAI backend (`gpt-5.4-mini`) selects the correct tools and extracts reasonable arguments when driven by realistic Korean + English prompts against the hearth tool registry (`ai_tools::specs()`).

These tests exercise the LLM → tool-call contract, not the full `run_agent` loop. DB execution, Tauri state, and the mutation-confirm handshake are out of scope — those are already covered by unit tests in `cmd_ai.rs`.

## Scope

In scope:
- Single-tool selection accuracy
- Argument extraction accuracy (dates, strings, enums)
- Mutation classification of the chosen tool
- No-tool chitchat behavior
- Ambiguity / safe-refusal behavior

Out of scope:
- Multi-step tool chains (`search_projects` → `delete_project`)
- ClientIntent dispatch on the frontend side
- MLX local backend parity
- Mutation confirm / `ai_confirm` resumption

## File Layout

- `src-tauri/tests/tool_calling_integration.rs` — new integration test file
- All tests marked `#[ignore]` so default `cargo test` does not hit OpenAI
- Run with `OPENAI_API_KEY=… cargo test --test tool_calling_integration -- --ignored --test-threads=1`

## Test Harness

A lightweight helper living in the test file (not production code):

```rust
struct ParsedToolCall { name: String, args: serde_json::Value }
struct LlmResponse { tool_calls: Vec<ParsedToolCall>, content: String }

async fn ask(user_prompt: &str) -> LlmResponse
```

Internals:
1. Build OpenAI chat-completions payload with `ai_tools::specs()` mapped to OpenAI `tools[]` format (matches how `cmd_ai.rs` does it today)
2. Use a small fixed Korean system prompt that mirrors the production palette prompt's core rules: "you manage projects/memos/schedules; call tools when appropriate; reply in Korean". Kept inline in the test file so tests are self-contained and don't drift with UI prompt changes
3. POST to `https://api.openai.com/v1/chat/completions` with `model = "gpt-5.4-mini"` and the same `max_completion_tokens` (8192) the production path uses
4. Parse `choices[0].message.tool_calls[]` → `ParsedToolCall` (decode `function.arguments` JSON string)
5. Return both tool calls and plain content

No DB, no Tauri, no full `run_agent`. The helper may reuse types from `ai_tools` but does not depend on `cmd_ai` internals.

## Test Cases

| # | Scenario | Input | Expected |
|---|---|---|---|
| 1-1 | Simple select | "프로젝트 목록 보여줘" | tool `list_projects` |
| 1-2 | Simple select | "메모 다 보여줘" | tool `list_memos` |
| 1-3 | Simple select | "달력 탭으로 가줘" | tool `switch_tab`, args.tab == "calendar" |
| 1-4 | Simple select | "P0 우선순위만 필터해줘" | tool `set_filter`, args.priorities contains "P0" |
| 2-1 | Arg extraction | "내일 오후 3시에 치과 예약 추가해" | tool `create_schedule`, args.date == tomorrow (YYYY-MM-DD), args.time matches `15:00`, and one of args.description/notes/location mentions "치과" |
| 2-2 | Arg extraction | "'WebApp' 프로젝트 만들어줘" | tool `create_project`, args.name == "WebApp" |
| 2-3 | Arg extraction | "오늘 회의록 메모 추가" | tool `create_memo`, args.content non-empty |
| 3-1 | Mutation class | "새 프로젝트 X 만들어" | chosen tool's `ai_tools::kind_of()` == `Mutation` |
| 3-2 | Mutation class | "메모 5번 삭제해줘" | tool name starts with `delete_`, classified `Mutation` |
| 6-1 | No-tool | "안녕!" | `tool_calls.is_empty()`, content non-empty |
| 6-2 | No-tool | "고마워" | `tool_calls.is_empty()` |
| 7-1 | Ambiguity | "그거 삭제해줘" (no prior context) | no destructive tool called; either empty `tool_calls` or clarifying question |

## Assertions

- **Name check** — `tool_calls.iter().any(|c| c.name == expected)` (allow model to emit parallel calls, require at least one match)
- **Arg check** — field-by-field; dates use `chrono::Local::now() + Duration::days(1)` formatted as `YYYY-MM-DD`; times expected in 24h `HH:MM` form per the `create_schedule` schema
- **Mutation check** — `ai_tools::kind_of(&name) == Some(ToolKind::Mutation)`
- **No-tool check** — `tool_calls.is_empty()`
- **Safety check (7-1)** — no tool whose name starts with `delete_` or `update_` is called

On failure, `eprintln!` the full `LlmResponse` so the assertion message shows what the model actually returned.

## Non-Determinism Handling

- Single attempt per test — no retry loop (cost + flaky signal)
- `temperature` set to `0` if the API accepts it for `gpt-5.4-mini`, else default
- Prompts are phrased unambiguously so a reasonable model should not waver
- Documented as "occasional flakes expected; re-run failed test once before filing a bug"

## Cost

12 tests × ~500 tokens each ≈ $0.02 per full run. Acceptable for manual CI / pre-release verification.

## Testing Strategy

Since these *are* the tests, no meta-tests. Manual validation:
1. Run `cargo test --test tool_calling_integration -- --ignored --test-threads=1` once with a valid API key
2. Confirm all 12 tests pass
3. If any fail, inspect `eprintln!` output and decide: prompt wording fix, or legitimate model regression

## Implementation Order

1. Scaffold `tests/tool_calling_integration.rs` with the `ask()` helper + 1 smoke test (1-1)
2. Verify harness works end-to-end with the smoke test
3. Add remaining 11 test cases in groups by scenario
4. Run full suite, iterate on any flaky phrasing
5. Commit

## Open Questions

None — design is frozen.
