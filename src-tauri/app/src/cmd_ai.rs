// OpenAI-only AI agent.
//
// MLX was removed in 0.3.0. The local backend hard-coded
// /Users/genie/dev/side/supergemma-bench/start-mlx.sh which never worked for
// anyone except the original developer and leaked that path into notarized
// release bundles.
//
// This file implements a standard OpenAI tool-calling agent.
// `ai_tools::specs()` advertises the tool catalog; the model emits
// `tool_calls`; we dispatch per-tool by `ToolKind`:
//
//   • Read         → execute immediately, feed result back into the loop
//   • Mutation     → pause and return Pending; client raises confirm modal;
//                    on approval, `ai_confirm` executes the call and resumes
//   • ClientIntent → collect and return with the final reply so React can
//                    dispatch (filter / focus UI navigation)
//
// The loop continues until the model stops requesting tools (Final) or hits
// MAX_STEPS (safety cap against runaways).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::Manager;

use crate::ai_tools::{self, ToolCall, ToolKind};
use crate::cmd_settings;

/// OpenAI model used for every request. Intentionally hard-coded — the
/// settings UI does not expose a picker, so changing targets is a single
/// constant bump (cheap, tool-capable).
const OPENAI_MODEL: &str = "gpt-5.4-mini";

/// Per-turn completion budget for the OpenAI path. Reasoning-family models
/// (GPT-5.x) bill this cap against *both* internal reasoning tokens and the
/// visible output, so the budget has to be generous enough to survive a
/// single reasoning pass plus a tool-call emission — empirically 2048 gets
/// truncated mid-reasoning on non-trivial prompts and returns an empty
/// `content`. 8192 is the sweet spot for our tool-calling loop.
const OPENAI_MAX_COMPLETION_TOKENS: u32 = 8192;

/// OpenAI chat-completions endpoint. Kept as a constant so tests / future
/// proxy setups can swap it without hunting.
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

// ---------- Agent types ----------

/// Chat message in OpenAI shape. `content` can be absent on assistant turns
/// that only issue tool_calls, and `tool_calls`/`tool_call_id` are carried
/// verbatim to preserve round-tripping. `Value` is used for tool_calls so we
/// don't reshape the server's string-encoded arguments payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Agent-loop outcome. Either the model settled (`Final`) or it asked for a
/// mutation we need the user to approve before continuing (`Pending`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AgentResult {
    /// Terminal state — model emitted reply text without a mutation request.
    /// `client_intents` collects any navigation/UI-state tool calls the model
    /// made during the run so React can dispatch them after rendering.
    Final {
        reply: String,
        client_intents: Vec<ToolCall>,
    },
    /// Paused awaiting confirmation for `call`. `history` is the full message
    /// log (including the pending tool-call turn) so the client can pass it
    /// back to `ai_confirm` unchanged.
    Pending {
        call: ToolCall,
        label: String,
        history: Vec<ChatMessage>,
    },
}

// ---------- Commands: chat ----------

#[tauri::command]
pub async fn ai_chat(
    app: tauri::AppHandle,
    messages: Vec<ChatMessage>,
) -> Result<AgentResult, String> {
    run_agent(&app, messages).await
}

/// Resume after a pending mutation was approved on the client. We execute the
/// tool, append the result, and run the loop again — the model may respond
/// with final prose, or chain into another tool call.
#[tauri::command]
pub async fn ai_confirm(
    app: tauri::AppHandle,
    history: Vec<ChatMessage>,
    call: ToolCall,
) -> Result<AgentResult, String> {
    let result = ai_tools::execute(&app, &call)
        .map_err(|e| format!("{} 실행 실패: {}", call.name, e))?;
    let mut history = history;
    history.push(tool_result_message(&call, &result));
    run_agent(&app, history).await
}

// ---------- Agent loop ----------

const MAX_STEPS: usize = 8;

async fn run_agent(
    app: &tauri::AppHandle,
    mut messages: Vec<ChatMessage>,
) -> Result<AgentResult, String> {
    // Resolve the OpenAI API key. A missing key returns a friendly error
    // instead of trying a local server — AI is optional; the rest of the
    // app continues working without it.
    let settings = cmd_settings::load_full(&app.state::<crate::AppState>())?;
    let api_key = settings.openai_api_key.ok_or_else(|| {
        "OpenAI API 키가 설정되지 않았습니다. 설정 → AI 탭에서 키를 입력해 주세요.".to_string()
    })?;

    let client = reqwest::Client::new();

    let tools_json: Vec<Value> = ai_tools::specs()
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

    let mut client_intents: Vec<ToolCall> = vec![];

    for _ in 0..MAX_STEPS {
        // GPT-5 family reasoning models (gpt-5.4-mini included) reject the
        // classic `max_tokens` + custom `temperature` combo:
        //   • `max_tokens` is deprecated — the API requires `max_completion_tokens`
        //     and returns 400 Bad Request otherwise.
        //   • `temperature` must be the default (1); any other value 400s with
        //     "Unsupported parameter".
        let body = json!({
            "model": OPENAI_MODEL,
            "messages": messages,
            "tools": tools_json,
            "max_completion_tokens": OPENAI_MAX_COMPLETION_TOKENS,
        });

        let resp = client
            .post(OPENAI_CHAT_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("AI request failed: {}", e))?;

        // OpenAI returns structured errors as 4xx JSON; surface the message
        // instead of silently falling through into the tool-call parser with
        // an empty `choices` array.
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            let pretty: Option<String> = serde_json::from_str::<Value>(&body_text)
                .ok()
                .and_then(|v| v["error"]["message"].as_str().map(String::from));
            return Err(format!(
                "AI request returned {}: {}",
                status,
                pretty.unwrap_or(body_text)
            ));
        }
        let data: Value = resp
            .json()
            .await
            .map_err(|e| format!("parse AI response failed: {}", e))?;

        let msg = &data["choices"][0]["message"];
        let content = msg["content"].as_str().unwrap_or("").to_string();
        let tool_calls_raw = msg["tool_calls"].as_array().cloned();

        // Echo the assistant turn back into history so the model sees its own
        // tool-call request on the next iteration.
        messages.push(ChatMessage {
            role: "assistant".into(),
            content: if content.is_empty() { None } else { Some(content.clone()) },
            name: None,
            tool_calls: tool_calls_raw.clone(),
            tool_call_id: None,
        });

        let calls = match tool_calls_raw {
            Some(c) if !c.is_empty() => c,
            _ => {
                return Ok(AgentResult::Final {
                    reply: content,
                    client_intents,
                });
            }
        };

        for (i, raw) in calls.iter().enumerate() {
            let parsed = match parse_tool_call(raw) {
                Some(c) => c,
                None => {
                    messages.push(ChatMessage {
                        role: "tool".into(),
                        content: Some(
                            json!({ "error": "malformed tool_call" }).to_string(),
                        ),
                        name: None,
                        tool_calls: None,
                        tool_call_id: raw["id"].as_str().map(String::from),
                    });
                    continue;
                }
            };

            match ai_tools::kind_of(&parsed.name) {
                Some(ToolKind::Read) => {
                    let result = ai_tools::execute(app, &parsed)
                        .unwrap_or_else(|e| json!({ "error": e }));
                    messages.push(tool_result_message(&parsed, &result));
                }
                Some(ToolKind::Mutation) => {
                    // OpenAI requires every tool_call_id in an assistant message to
                    // have a matching tool result before the next request. When the
                    // model batches multiple calls and we pause on a Mutation, add
                    // placeholder results for all remaining calls in this batch so
                    // the resumed request doesn't 400.
                    for remaining in &calls[i + 1..] {
                        if let Some(id) = remaining["id"].as_str() {
                            messages.push(ChatMessage {
                                role: "tool".into(),
                                content: Some(
                                    json!({ "skipped": "preceding mutation awaiting confirmation" }).to_string(),
                                ),
                                name: None,
                                tool_calls: None,
                                tool_call_id: Some(id.to_string()),
                            });
                        }
                    }
                    let label = describe_call(&parsed);
                    return Ok(AgentResult::Pending {
                        call: parsed,
                        label,
                        history: messages,
                    });
                }
                Some(ToolKind::ClientIntent) => {
                    // The client will dispatch this after rendering; feed the
                    // model a "dispatched" ack so it can move on.
                    let ack = json!({ "ok": true, "dispatched": parsed.name });
                    messages.push(tool_result_message(&parsed, &ack));
                    client_intents.push(parsed);
                }
                None => {
                    messages.push(tool_result_message(
                        &parsed,
                        &json!({ "error": format!("unknown tool: {}", parsed.name) }),
                    ));
                }
            }
        }
    }

    Err(format!(
        "AI agent exceeded {} steps without settling",
        MAX_STEPS
    ))
}

fn tool_result_message(call: &ToolCall, result: &Value) -> ChatMessage {
    ChatMessage {
        role: "tool".into(),
        content: Some(result.to_string()),
        name: Some(call.name.clone()),
        tool_calls: None,
        tool_call_id: Some(call.id.clone()),
    }
}

/// Parse a raw `choices[0].message.tool_calls[i]` into our typed form. The
/// OpenAI convention encodes `function.arguments` as a JSON *string*; we
/// decode it here so downstream executors can read fields by name.
fn parse_tool_call(raw: &Value) -> Option<ToolCall> {
    let id = raw["id"].as_str()?.to_string();
    let name = raw["function"]["name"].as_str()?.to_string();
    let args_str = raw["function"]["arguments"].as_str().unwrap_or("{}");
    let arguments: Value =
        serde_json::from_str(args_str).unwrap_or(Value::Object(Default::default()));
    Some(ToolCall {
        id,
        name,
        arguments,
    })
}

/// Human-readable summary of a pending mutation for the confirm modal. Kept
/// intentionally terse — the user already sees the AI's prose reply, so this
/// only needs to disambiguate *which* action they're approving.
fn describe_call(call: &ToolCall) -> String {
    let a = &call.arguments;
    let s = |k: &str| a.get(k).and_then(|v| v.as_str()).unwrap_or("?").to_string();
    let i = |k: &str| a.get(k).and_then(|v| v.as_i64()).unwrap_or(-1);
    // Truncate memo content so the confirm label stays one-line — long notes
    // otherwise blow past the dialog width.
    let preview = |k: &str, limit: usize| {
        let full = s(k);
        if full.chars().count() > limit {
            let head: String = full.chars().take(limit).collect();
            format!("{}…", head)
        } else {
            full
        }
    };
    match call.name.as_str() {
        // Mirror the executor's P2 default so the confirm label doesn't show
        // "(?)" when the model omits priority.
        "create_project" => {
            let pri = a
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("P2");
            format!("프로젝트 '{}' ({}) 생성", s("name"), pri)
        }
        "update_project" => format!("프로젝트 #{} 수정", i("id")),
        "delete_project" => format!("프로젝트 #{} 삭제", i("id")),

        "create_memo" => format!("메모 추가: '{}'", preview("content", 40)),
        "update_memo" => format!("메모 #{} 수정", i("id")),
        "delete_memo" => format!("메모 #{} 삭제", i("id")),

        "create_schedule" => {
            let when = match a.get("time").and_then(|v| v.as_str()) {
                Some(t) if !t.is_empty() => format!("{} {}", s("date"), t),
                _ => s("date"),
            };
            let desc = a.get("description").and_then(|v| v.as_str()).unwrap_or("");
            if desc.is_empty() {
                format!("일정 {} 등록", when)
            } else {
                format!("일정 {} · {} 등록", when, desc)
            }
        }
        "update_schedule" => format!("일정 #{} 수정", i("id")),
        "delete_schedule" => format!("일정 #{} 삭제", i("id")),

        _ => call.name.replace('_', " "),
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_tool_call_shape() {
        let raw = json!({
            "id": "call_abc",
            "type": "function",
            "function": {
                "name": "create_project",
                "arguments": "{\"name\":\"pickat\",\"priority\":\"P0\"}"
            }
        });
        let parsed = parse_tool_call(&raw).expect("parse");
        assert_eq!(parsed.id, "call_abc");
        assert_eq!(parsed.name, "create_project");
        assert_eq!(parsed.arguments["name"], json!("pickat"));
        assert_eq!(parsed.arguments["priority"], json!("P0"));
    }

    #[test]
    fn parse_tool_call_tolerates_missing_arguments() {
        let raw = json!({
            "id": "x",
            "function": { "name": "list_projects" }
        });
        let parsed = parse_tool_call(&raw).expect("parse");
        assert_eq!(parsed.name, "list_projects");
        assert!(parsed.arguments.as_object().unwrap().is_empty());
    }

    #[test]
    fn parse_tool_call_rejects_without_id_or_name() {
        assert!(parse_tool_call(&json!({"function": {"name": "x"}})).is_none());
        assert!(parse_tool_call(&json!({"id": "x", "function": {}})).is_none());
    }

    #[test]
    fn describe_known_mutations() {
        let c = ToolCall {
            id: "i".into(),
            name: "create_project".into(),
            arguments: json!({ "name": "pickat", "priority": "P0" }),
        };
        assert_eq!(describe_call(&c), "프로젝트 'pickat' (P0) 생성");

        let d = ToolCall {
            id: "i".into(),
            name: "delete_project".into(),
            arguments: json!({ "id": 7 }),
        };
        assert_eq!(describe_call(&d), "프로젝트 #7 삭제");
    }

    #[test]
    fn describe_defaults_priority_to_p2_when_omitted() {
        // The confirm label must mirror the executor's P2 fallback.
        let c = ToolCall {
            id: "i".into(),
            name: "create_project".into(),
            arguments: json!({ "name": "pickat" }),
        };
        assert_eq!(describe_call(&c), "프로젝트 'pickat' (P2) 생성");
    }

    #[test]
    fn describe_falls_back_to_spaced_name() {
        let c = ToolCall {
            id: "i".into(),
            name: "focus_project".into(),
            arguments: json!({}),
        };
        assert_eq!(describe_call(&c), "focus project");
    }

    #[test]
    fn describe_memo_mutations() {
        let create = ToolCall {
            id: "i".into(),
            name: "create_memo".into(),
            arguments: json!({ "content": "회의록 정리" }),
        };
        assert_eq!(describe_call(&create), "메모 추가: '회의록 정리'");

        let del = ToolCall {
            id: "i".into(),
            name: "delete_memo".into(),
            arguments: json!({ "id": 12 }),
        };
        assert_eq!(describe_call(&del), "메모 #12 삭제");
    }

    #[test]
    fn describe_memo_truncates_long_content() {
        // The confirm dialog is one-line; anything > 40 chars should get the
        // ellipsis treatment so the label doesn't overflow.
        let long = "가".repeat(60);
        let c = ToolCall {
            id: "i".into(),
            name: "create_memo".into(),
            arguments: json!({ "content": long }),
        };
        let label = describe_call(&c);
        assert!(label.ends_with("…'"), "got: {}", label);
    }

    #[test]
    fn describe_schedule_mutations() {
        let with_time = ToolCall {
            id: "i".into(),
            name: "create_schedule".into(),
            arguments: json!({
                "date": "2026-04-20",
                "time": "15:00",
                "description": "디자인 리뷰"
            }),
        };
        assert_eq!(
            describe_call(&with_time),
            "일정 2026-04-20 15:00 · 디자인 리뷰 등록"
        );

        let date_only = ToolCall {
            id: "i".into(),
            name: "create_schedule".into(),
            arguments: json!({ "date": "2026-04-20" }),
        };
        assert_eq!(describe_call(&date_only), "일정 2026-04-20 등록");

        let del = ToolCall {
            id: "i".into(),
            name: "delete_schedule".into(),
            arguments: json!({ "id": 7 }),
        };
        assert_eq!(describe_call(&del), "일정 #7 삭제");
    }
}
