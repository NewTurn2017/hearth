//! Integration tests for OpenAI tool-calling against the hearth tool
//! registry. All tests are `#[ignore]` because they hit the real OpenAI API
//! (~$0.02 per full run) and require `OPENAI_API_KEY`.
//!
//! Run with:
//!   OPENAI_API_KEY=sk-... cargo test --test tool_calling_integration \
//!       -- --ignored --test-threads=1 --nocapture
//!
//! Design: docs/superpowers/specs/2026-04-16-tool-calling-integration-tests-design.md

use chrono::{Duration, Local};
use serde_json::{json, Value};
use hearth_lib::ai_tools::{kind_of, specs, ToolKind};

const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";
const OPENAI_MODEL: &str = "gpt-5.4-mini";
const OPENAI_MAX_COMPLETION_TOKENS: u32 = 8192;

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
    let call = r.first_with("switch_tab").unwrap_or_else(|| {
        panic!(
            "expected switch_tab, got {:?} (content: {})",
            r.tool_calls, r.content
        )
    });
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
    let call = r.first_with("set_filter").unwrap_or_else(|| {
        panic!(
            "expected set_filter, got {:?} (content: {})",
            r.tool_calls, r.content
        )
    });
    let priorities = call.args["priorities"]
        .as_array()
        .unwrap_or_else(|| panic!("expected priorities array, got args {:?}", call.args));
    assert!(
        priorities.iter().any(|v| v.as_str() == Some("P0")),
        "expected priorities to contain P0, got {:?}",
        priorities
    );
}

// ---------- Scenario 2: argument extraction ----------

#[tokio::test]
#[ignore]
async fn t2_1_create_schedule_tomorrow_3pm() {
    let r = ask("내일 오후 3시에 치과 예약 추가해줘").await;
    let call = r.first_with("create_schedule").unwrap_or_else(|| {
        panic!(
            "expected create_schedule, got {:?} (content: {})",
            r.tool_calls, r.content
        )
    });

    let expected_date = (Local::now() + Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
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
    let call = r.first_with("create_project").unwrap_or_else(|| {
        panic!(
            "expected create_project, got {:?} (content: {})",
            r.tool_calls, r.content
        )
    });
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
    let call = r.first_with("create_memo").unwrap_or_else(|| {
        panic!(
            "expected create_memo, got {:?} (content: {})",
            r.tool_calls, r.content
        )
    });
    let content = call.args["content"].as_str().unwrap_or("");
    assert!(
        !content.trim().is_empty(),
        "expected non-empty content, got args {:?}",
        call.args
    );
}

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
