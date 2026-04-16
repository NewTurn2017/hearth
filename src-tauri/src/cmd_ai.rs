use serde::{Deserialize, Serialize};
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;

// ---------- State machine ----------

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AiServerState {
    Idle,
    Starting,
    Running { port: u16 },
    Failed { error: String },
}

pub struct AiManager {
    pub state: Mutex<AiServerState>,
    pub child: Mutex<Option<Child>>,
    pub script_path: Mutex<String>,
    pub last_try: Mutex<Option<Instant>>,
}

impl AiManager {
    pub fn new(script_path: String) -> Self {
        Self {
            state: Mutex::new(AiServerState::Idle),
            child: Mutex::new(None),
            script_path: Mutex::new(script_path),
            last_try: Mutex::new(None),
        }
    }
}

// ---------- Commands: lifecycle ----------

const PORT: u16 = 8080;
const HEALTH_URL: &str = "http://127.0.0.1:8080/v1/models";

async fn is_alive(client: &reqwest::Client) -> bool {
    client.get(HEALTH_URL).send().await.is_ok()
}

#[tauri::command]
pub async fn start_ai_server(mgr: State<'_, AiManager>) -> Result<AiServerState, String> {
    let client = reqwest::Client::new();

    // Already running (possibly external instance)? Adopt.
    if is_alive(&client).await {
        let s = AiServerState::Running { port: PORT };
        *mgr.state.lock().map_err(|e| e.to_string())? = s.clone();
        return Ok(s);
    }

    // Mark starting.
    {
        *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Starting;
        *mgr.last_try.lock().map_err(|e| e.to_string())? = Some(Instant::now());
    }

    let script = mgr.script_path.lock().map_err(|e| e.to_string())?.clone();
    let child = Command::new("bash")
        .arg(&script)
        .spawn()
        .map_err(|e| {
            let err = format!("spawn failed: {}", e);
            *mgr.state.lock().unwrap() = AiServerState::Failed { error: err.clone() };
            err
        })?;

    *mgr.child.lock().map_err(|e| e.to_string())? = Some(child);

    // Poll up to 120s.
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if is_alive(&client).await {
            let s = AiServerState::Running { port: PORT };
            *mgr.state.lock().map_err(|e| e.to_string())? = s.clone();
            return Ok(s);
        }
    }

    let err = "AI server failed to start within 120s".to_string();
    *mgr.state.lock().map_err(|e| e.to_string())? =
        AiServerState::Failed { error: err.clone() };
    Err(err)
}

#[tauri::command]
pub async fn ai_server_status(mgr: State<'_, AiManager>) -> Result<AiServerState, String> {
    let client = reqwest::Client::new();
    let alive = is_alive(&client).await;
    let mut state = mgr.state.lock().map_err(|e| e.to_string())?;
    if alive {
        *state = AiServerState::Running { port: PORT };
    } else if matches!(*state, AiServerState::Running { .. }) {
        *state = AiServerState::Idle;
    }
    Ok(state.clone())
}

#[tauri::command]
pub async fn stop_ai_server(mgr: State<'_, AiManager>) -> Result<(), String> {
    kill_child(&mgr);
    *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Idle;
    Ok(())
}

/// Targeted kill of the exact child we spawned. Called from window-destroyed hook
/// in `lib.rs` — does NOT use `pkill -f mlx_lm.server` (that would kill siblings).
pub fn kill_child(mgr: &AiManager) {
    if let Ok(mut guard) = mgr.child.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------- Commands: chat ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    Mutation,
    Navigation,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAction {
    #[serde(rename = "type")]
    pub kind: ActionType,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub reply: String,
    pub actions: Vec<AiAction>,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["reply", "actions"],
        "properties": {
            "reply": { "type": "string" },
            "actions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["type", "label"],
                    "properties": {
                        "type": { "enum": ["mutation", "navigation", "info"] },
                        "label": { "type": "string" },
                        "command": {
                            "enum": [
                                "create_project", "update_project", "delete_project",
                                "create_schedule", "update_schedule", "delete_schedule",
                                "create_memo", "update_memo", "delete_memo",
                                "set_filter", "focus_project"
                            ]
                        },
                        "args": { "type": "object" }
                    }
                }
            }
        }
    })
}

#[tauri::command]
pub async fn ai_chat(messages: Vec<ChatMessage>) -> Result<AiResponse, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "default",
        "messages": messages,
        "max_tokens": 2048,
        "temperature": 0.4,
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": "genie_response", "schema": schema(), "strict": true }
        }
    });

    let resp = client
        .post("http://127.0.0.1:8080/v1/chat/completions")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AI request failed: {}", e))?;

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse AI response failed: {}", e))?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    parse_ai_content(&content)
}

/// Parse assistant content into `AiResponse`. If the server ignored
/// `response_format`, the content may contain prose + json. We strip code
/// fences and try progressively looser parses.
pub fn parse_ai_content(content: &str) -> Result<AiResponse, String> {
    // Strip ```json ... ``` fences if present.
    let cleaned = content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // 1) Direct parse.
    if let Ok(r) = serde_json::from_str::<AiResponse>(cleaned) {
        return Ok(r);
    }

    // 2) Find the first balanced JSON object substring.
    if let Some(slice) = extract_first_json_object(cleaned) {
        if let Ok(r) = serde_json::from_str::<AiResponse>(slice) {
            return Ok(r);
        }
    }

    Err(format!("AI 응답 파싱 실패: {}", cleaned))
}

fn extract_first_json_object(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = r#"{"reply":"ok","actions":[]}"#;
        let r = parse_ai_content(raw).expect("parse");
        assert_eq!(r.reply, "ok");
        assert!(r.actions.is_empty());
    }

    #[test]
    fn parses_fenced_json_with_prose() {
        let raw = "여기 결과입니다:\n```json\n{\"reply\":\"hi\",\"actions\":[{\"type\":\"info\",\"label\":\"noop\"}]}\n```";
        let r = parse_ai_content(raw).expect("parse");
        assert_eq!(r.reply, "hi");
        assert_eq!(r.actions.len(), 1);
    }

    #[test]
    fn fails_on_garbage() {
        let raw = "sorry, I couldn't";
        assert!(parse_ai_content(raw).is_err());
    }
}
