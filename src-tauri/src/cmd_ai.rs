use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Mutex;
use tauri::State;

pub struct AiState {
    pub pid: Mutex<Option<u32>>,
    pub script_path: Mutex<String>,
}

#[derive(Serialize)]
pub struct AiServerStatus {
    pub running: bool,
    pub port: u16,
}

#[tauri::command]
pub async fn start_ai_server(ai_state: State<'_, AiState>) -> Result<AiServerStatus, String> {
    let script = ai_state.script_path.lock().map_err(|e| e.to_string())?.clone();

    let client = reqwest::Client::new();
    if client
        .get("http://127.0.0.1:8080/v1/models")
        .send()
        .await
        .is_ok()
    {
        return Ok(AiServerStatus {
            running: true,
            port: 8080,
        });
    }

    let child = Command::new("bash")
        .arg(&script)
        .spawn()
        .map_err(|e| format!("Failed to start AI server: {}", e))?;

    *ai_state.pid.lock().map_err(|e| e.to_string())? = Some(child.id());

    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if client
            .get("http://127.0.0.1:8080/v1/models")
            .send()
            .await
            .is_ok()
        {
            return Ok(AiServerStatus {
                running: true,
                port: 8080,
            });
        }
    }

    Err("AI server failed to start within 120s".into())
}

#[tauri::command]
pub async fn stop_ai_server(ai_state: State<'_, AiState>) -> Result<(), String> {
    Command::new("pkill")
        .args(["-f", "mlx_lm.server"])
        .output()
        .ok();

    *ai_state.pid.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

#[tauri::command]
pub async fn ai_server_status(ai_state: State<'_, AiState>) -> Result<AiServerStatus, String> {
    let client = reqwest::Client::new();
    let running = client
        .get("http://127.0.0.1:8080/v1/models")
        .send()
        .await
        .is_ok();

    if !running {
        *ai_state.pid.lock().map_err(|e| e.to_string())? = None;
    }

    Ok(AiServerStatus {
        running,
        port: 8080,
    })
}

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[tauri::command]
pub async fn ai_chat(messages: Vec<ChatMessage>) -> Result<ChatResponse, String> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "default",
        "messages": messages.iter().map(|m| {
            serde_json::json!({
                "role": m.role,
                "content": m.content,
            })
        }).collect::<Vec<_>>(),
        "max_tokens": 4096,
        "temperature": 0.7,
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
        .map_err(|e| format!("Failed to parse AI response: {}", e))?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let tool_calls = extract_tool_calls(&content);

    Ok(ChatResponse {
        content: if tool_calls.is_some() {
            content
                .lines()
                .filter(|l| !l.trim().starts_with('{') && !l.trim().starts_with('}'))
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        } else {
            content
        },
        tool_calls,
    })
}

fn extract_tool_calls(content: &str) -> Option<Vec<ToolCall>> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.contains("\"action\"") {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(action) = val.get("action").and_then(|a| a.as_str()) {
                    return Some(vec![ToolCall {
                        name: action.to_string(),
                        arguments: val,
                    }]);
                }
            }
        }
    }
    None
}
