// MLX-backed AI agent.
//
// The previous implementation asked Gemma to emit a custom
// `{reply, actions[]}` JSON via `response_format: json_schema`. That made the
// model responsible for formatting AND intent at once, and it could not loop
// (read data, reason, act) — a confirmation like "생성하시겠습니까?" left the
// palette stuck with no way to progress.
//
// This file implements a standard OpenAI-style tool-calling agent instead.
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
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Manager, State};

use crate::ai_tools::{self, ToolCall, ToolKind};
use crate::cmd_settings::{self, AiSettingsFull};

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

/// Per-turn token budget for the local MLX path. Gemma-class models count
/// only visible tokens here, so the classic 2048 is plenty.
const MLX_MAX_TOKENS: u32 = 2048;

/// OpenAI chat-completions endpoint. Kept as a constant so tests / future
/// proxy setups can swap it without hunting.
const OPENAI_CHAT_URL: &str = "https://api.openai.com/v1/chat/completions";

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
    /// Cached ID of the currently-loaded MLX model (the one specified via
    /// `--model` on the listener process). Required for chat requests — MLX's
    /// `/v1/models` lists every cached model, so "default" or the wrong ID
    /// causes MLX to try pulling from HF and fail with 401.
    pub model_id: Mutex<Option<String>>,
    /// `true` iff this app spawned the MLX process (vs. adopting one that was
    /// already listening — e.g. started by Lumo). Drives app-close cleanup:
    /// we only kill MLX on window-destroyed when we started it ourselves, so
    /// sibling apps sharing the port aren't affected.
    pub started_by_us: AtomicBool,
}

impl AiManager {
    pub fn new(script_path: String) -> Self {
        Self {
            state: Mutex::new(AiServerState::Idle),
            child: Mutex::new(None),
            script_path: Mutex::new(script_path),
            last_try: Mutex::new(None),
            model_id: Mutex::new(None),
            started_by_us: AtomicBool::new(false),
        }
    }
}

// ---------- Commands: lifecycle ----------

// Use 18080 to avoid clashes with OrbStack/other services on 8080.
const PORT: u16 = 18080;

fn health_url() -> String {
    format!("http://127.0.0.1:{}/v1/models", PORT)
}

fn chat_url() -> String {
    format!("http://127.0.0.1:{}/v1/chat/completions", PORT)
}

/// Returns the currently-loaded MLX model ID when the port answers like an
/// OpenAI/MLX `/v1/models` endpoint, otherwise None.
///
/// Plain `send().is_ok()` would be too loose — any stray HTTP service on the
/// port (OrbStack, Jenkins, etc.) would return 2xx/4xx and fool us.
///
/// We *must* know the loaded model ID so chat requests can target it directly.
/// `/v1/models` lists every cached model in HF, so we resolve the real one by
/// peeking at the listener process's `--model` argument — this works whether
/// our app spawned the server or an adopted instance (e.g. Lumo) did.
async fn probe_alive(client: &reqwest::Client) -> Option<String> {
    let resp = client.get(health_url()).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    if body.get("object").and_then(|v| v.as_str()) != Some("list") {
        return None;
    }
    if let Some(m) = detect_loaded_model(PORT) {
        return Some(m);
    }
    // Last-resort fallback: the first cached model. May trigger a reload in
    // MLX but at least won't 401 with an unknown ID.
    body["data"][0]["id"].as_str().map(String::from)
}

// macOS absolute paths — GUI apps often launch with a restricted PATH that
// excludes `/usr/sbin`, so `Command::new("lsof")` would silently fail-to-spawn
// and `find_port_listener_pid` would return None, silently breaking kill.
const LSOF_BIN: &str = "/usr/sbin/lsof";
const PS_BIN: &str = "/bin/ps";
const PGREP_BIN: &str = "/usr/bin/pgrep";
const KILL_BIN: &str = "/bin/kill";

/// Returns the PID of the process listening on `port`, if any.
///
/// Tries three strategies in order, because a Tauri GUI process on macOS often
/// can't list other processes' network sockets (TCC / entitlements), making
/// `lsof` silently empty. The process table itself is always visible.
///   1. Read `.mlx.pid` next to the launch script AND verify it still points
///      at an `mlx_lm.server` process. PIDs are recycled — a stale file can
///      hold an unrelated app's PID; if we kill that blindly we clobber the
///      wrong program.
///   2. `pgrep -f "mlx_lm.server ... --port <PORT>"` — scans `ps`-visible
///      process commands, unaffected by socket-layer TCC. This is the branch
///      that rescues stop when a sibling (Lumo, another run) owns the server.
///   3. `lsof` as a last resort (helps on setups where pgrep isn't present).
fn find_port_listener_pid(port: u16) -> Option<String> {
    if let Some(pid) = read_launcher_pid_file() {
        if is_pid_alive(&pid) && pid_is_mlx_server(&pid) {
            return Some(pid);
        }
    }
    pgrep_mlx_server(port).or_else(|| lsof_port_listener(port))
}

/// True when `ps -o command=` for `pid` contains `mlx_lm.server`. Guards the
/// pid-file path against PID recycling after the launcher's process is gone.
fn pid_is_mlx_server(pid: &str) -> bool {
    let out = match Command::new(PS_BIN)
        .args(["-o", "command=", "-p", pid])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };
    String::from_utf8_lossy(&out.stdout).contains("mlx_lm.server")
}

/// Locate the running MLX listener for `port` via the process table. Matches
/// any command line containing both `mlx_lm.server` and `--port <port>`, so we
/// don't depend on any particular launcher's naming.
fn pgrep_mlx_server(port: u16) -> Option<String> {
    let pattern = format!("mlx_lm\\.server.*--port {}", port);
    let out = Command::new(PGREP_BIN)
        .args(["-f", &pattern])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
}

/// Read `.mlx.pid` from the directory containing the launch script. The
/// script writes `echo $! > "$PID_FILE"` after spawning the server.
fn read_launcher_pid_file() -> Option<String> {
    // The launch script path is stable per AiManager::new; resolving it via
    // filesystem keeps this helper independent of manager state so refresh
    // and kill paths can both use it.
    let script = "/Users/genie/dev/side/supergemma-bench/start-mlx.sh";
    let pid_file = std::path::Path::new(script).parent()?.join(".mlx.pid");
    let raw = std::fs::read_to_string(&pid_file).ok()?;
    let pid = raw.trim();
    if pid.is_empty() {
        return None;
    }
    Some(pid.to_string())
}

fn is_pid_alive(pid: &str) -> bool {
    // `kill -0 pid` sends no signal but returns 0 iff the PID exists and we
    // have permission to signal it. Cheap liveness check.
    Command::new(KILL_BIN)
        .arg("-0")
        .arg(pid)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn lsof_port_listener(port: u16) -> Option<String> {
    let lsof = Command::new(LSOF_BIN)
        .args([
            "-iTCP",
            &format!(":{}", port),
            "-sTCP:LISTEN",
            "-P",
            "-n",
            "-F",
            "p",
        ])
        .output()
        .ok()?;
    if !lsof.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&lsof.stdout);
    stdout
        .lines()
        .find(|l| l.starts_with('p'))
        .map(|l| l.trim_start_matches('p').to_string())
}

/// Find the process listening on `port` and extract its `--model` argument
/// from the command line. Used to infer the active MLX model.
fn detect_loaded_model(port: u16) -> Option<String> {
    let pid = find_port_listener_pid(port)?;

    let ps = Command::new(PS_BIN)
        .args(["-o", "command=", "-p", &pid])
        .output()
        .ok()?;
    if !ps.status.success() {
        return None;
    }
    let cmd = String::from_utf8_lossy(&ps.stdout);
    let mut parts = cmd.split_whitespace();
    while let Some(p) = parts.next() {
        if p == "--model" {
            return parts.next().map(String::from);
        }
    }
    None
}

/// Update the cached state based on the current liveness probe. Returns the
/// model ID if alive.
async fn refresh_state(mgr: &AiManager, client: &reqwest::Client) -> Option<String> {
    let model = probe_alive(client).await;
    if let Some(m) = &model {
        if let Ok(mut guard) = mgr.model_id.lock() {
            *guard = Some(m.clone());
        }
        if let Ok(mut state) = mgr.state.lock() {
            *state = AiServerState::Running { port: PORT };
        }
    }
    model
}

#[tauri::command]
pub async fn start_ai_server(
    app: AppHandle,
    mgr: State<'_, AiManager>,
) -> Result<AiServerState, String> {
    // If the user chose OpenAI, there is no local server to start. Report
    // "running" with a sentinel port so the UI skips the loading dialog.
    if current_provider(&app) == "openai" {
        let state = AiServerState::Running { port: 0 };
        *mgr.state.lock().map_err(|e| e.to_string())? = state.clone();
        return Ok(state);
    }

    let client = reqwest::Client::new();

    // Already running (possibly external instance like Lumo)? Adopt — and
    // leave `started_by_us = false` so app-close doesn't kill a sibling's
    // MLX process.
    if refresh_state(&mgr, &client).await.is_some() {
        return Ok(AiServerState::Running { port: PORT });
    }

    // Mark starting.
    {
        *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Starting;
        *mgr.last_try.lock().map_err(|e| e.to_string())? = Some(Instant::now());
    }

    let script = mgr.script_path.lock().map_err(|e| e.to_string())?.clone();
    let child = Command::new("bash")
        .arg(&script)
        .env("MLX_PORT", PORT.to_string())
        .spawn()
        .map_err(|e| {
            let err = format!("spawn failed: {}", e);
            *mgr.state.lock().unwrap() = AiServerState::Failed { error: err.clone() };
            err
        })?;

    *mgr.child.lock().map_err(|e| e.to_string())? = Some(child);
    // We own this MLX process now — app-close cleanup should kill it.
    mgr.started_by_us.store(true, Ordering::SeqCst);

    // Poll up to 120s.
    for _ in 0..120 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if refresh_state(&mgr, &client).await.is_some() {
            return Ok(AiServerState::Running { port: PORT });
        }
    }

    let err = "AI server failed to start within 120s".to_string();
    *mgr.state.lock().map_err(|e| e.to_string())? =
        AiServerState::Failed { error: err.clone() };
    Err(err)
}

#[tauri::command]
pub async fn ai_server_status(
    app: AppHandle,
    mgr: State<'_, AiManager>,
) -> Result<AiServerState, String> {
    // OpenAI mode has no server lifecycle — always report running so the
    // palette skips its "starting" dialog. We still stamp the cached state so
    // subsequent reads stay consistent.
    if current_provider(&app) == "openai" {
        let state = AiServerState::Running { port: 0 };
        *mgr.state.lock().map_err(|e| e.to_string())? = state.clone();
        return Ok(state);
    }

    let client = reqwest::Client::new();
    if refresh_state(&mgr, &client).await.is_some() {
        return Ok(AiServerState::Running { port: PORT });
    }
    let mut state = mgr.state.lock().map_err(|e| e.to_string())?;
    if matches!(*state, AiServerState::Running { .. }) {
        *state = AiServerState::Idle;
    }
    Ok(state.clone())
}

/// Cheap read-only lookup of the current provider. Swallows DB errors to
/// "local" so a broken settings table never breaks the AI status probe.
fn current_provider(app: &AppHandle) -> String {
    let state = app.state::<crate::AppState>();
    cmd_settings::load_full(&state)
        .map(|s| s.provider)
        .unwrap_or_else(|_| "local".to_string())
}

#[tauri::command]
pub async fn stop_ai_server(mgr: State<'_, AiManager>) -> Result<(), String> {
    // Explicit user intent — kill regardless of who started the listener.
    kill_child(&mgr);
    kill_mlx_listener(PORT).await;
    mgr.started_by_us.store(false, Ordering::SeqCst);
    *mgr.state.lock().map_err(|e| e.to_string())? = AiServerState::Idle;
    Ok(())
}

/// SIGTERM → 700ms grace → SIGKILL on the process listening at `port`.
/// Used by explicit stop and by app-close cleanup (when we own the process).
/// The bash launcher detaches the MLX server via `nohup ... &`, so just
/// reaping the bash child leaves the actual Python server alive; we have to
/// target the listener by port.
async fn kill_mlx_listener(port: u16) {
    let Some(pid) = find_port_listener_pid(port) else {
        return;
    };
    let _ = Command::new(KILL_BIN).arg("-15").arg(&pid).status();
    tokio::time::sleep(std::time::Duration::from_millis(700)).await;
    if is_pid_alive(&pid) {
        let _ = Command::new(KILL_BIN).arg("-9").arg(&pid).status();
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
}

/// App-close cleanup: kill the MLX listener only if this app started it.
/// Adopted siblings (Lumo etc.) are left running. Blocking call — runs on
/// the `on_window_event` thread, so we cannot await; we do a best-effort
/// SIGTERM then SIGKILL in a tight loop.
pub fn kill_mlx_if_ours(mgr: &AiManager) {
    if !mgr.started_by_us.swap(false, Ordering::SeqCst) {
        return;
    }
    let Some(pid) = find_port_listener_pid(PORT) else {
        return;
    };
    let _ = Command::new(KILL_BIN).arg("-15").arg(&pid).status();
    for _ in 0..7 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if find_port_listener_pid(PORT).is_none() {
            return;
        }
    }
    let _ = Command::new(KILL_BIN).arg("-9").arg(&pid).status();
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

// ---------- Agent types ----------

/// Chat message in OpenAI shape. `content` can be absent on assistant turns
/// that only issue tool_calls, and `tool_calls`/`tool_call_id` are carried
/// verbatim to preserve round-tripping (MLX echoes them back on subsequent
/// turns). `Value` is used for tool_calls so we don't reshape the server's
/// string-encoded arguments payload.
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
    app: AppHandle,
    mgr: State<'_, AiManager>,
    messages: Vec<ChatMessage>,
) -> Result<AgentResult, String> {
    run_agent(&app, &mgr, messages).await
}

/// Resume after a pending mutation was approved on the client. We execute the
/// tool, append the result, and run the loop again — the model may respond
/// with final prose, or chain into another tool call.
#[tauri::command]
pub async fn ai_confirm(
    app: AppHandle,
    mgr: State<'_, AiManager>,
    history: Vec<ChatMessage>,
    call: ToolCall,
) -> Result<AgentResult, String> {
    let result = ai_tools::execute(&app, &call)
        .map_err(|e| format!("{} 실행 실패: {}", call.name, e))?;
    let mut history = history;
    history.push(tool_result_message(&call, &result));
    run_agent(&app, &mgr, history).await
}

// ---------- Agent loop ----------

const MAX_STEPS: usize = 8;

/// Resolved target for one chat request. Built per-run so switching provider
/// in settings takes effect on the next AI turn without requiring a restart.
struct Backend {
    url: String,
    /// When `Some`, sent as `Authorization: Bearer <token>`. Local MLX does
    /// not need auth; OpenAI does.
    bearer: Option<String>,
    model: String,
}

/// Look at the persisted settings and pick the right chat endpoint.
///
/// For local MLX we resolve the model name from the running process — MLX
/// requires the exact loaded model ID (`/v1/models` lists every cached
/// model, and a wrong ID triggers an HF pull that 401s). OpenAI uses the
/// hard-coded `OPENAI_MODEL` constant; settings no longer carry a model
/// field.
async fn resolve_backend(
    app: &AppHandle,
    mgr: &AiManager,
    client: &reqwest::Client,
) -> Result<Backend, String> {
    let settings: AiSettingsFull = cmd_settings::load_full(&app.state::<crate::AppState>())?;
    if settings.provider == "openai" {
        let key = settings.openai_api_key.ok_or_else(|| {
            "OpenAI API 키가 설정되지 않았습니다. 설정에서 키를 입력해 주세요.".to_string()
        })?;
        return Ok(Backend {
            url: OPENAI_CHAT_URL.to_string(),
            bearer: Some(key),
            model: OPENAI_MODEL.to_string(),
        });
    }

    // Local MLX — model ID comes from the server, never from settings.
    let cached = mgr.model_id.lock().map_err(|e| e.to_string())?.clone();
    let model = match cached {
        Some(m) => m,
        None => refresh_state(mgr, client)
            .await
            .ok_or_else(|| "AI 서버를 찾을 수 없습니다. 먼저 서버를 시작하세요.".to_string())?,
    };
    Ok(Backend {
        url: chat_url(),
        bearer: None,
        model,
    })
}

async fn run_agent(
    app: &AppHandle,
    mgr: &AiManager,
    mut messages: Vec<ChatMessage>,
) -> Result<AgentResult, String> {
    let client = reqwest::Client::new();
    let backend = resolve_backend(app, mgr, &client).await?;

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

    // GPT-5 family reasoning models (gpt-5.4-mini included) reject the
    // classic `max_tokens` + custom `temperature` combo:
    //   • `max_tokens` is deprecated — the API requires `max_completion_tokens`
    //     and returns 400 Bad Request otherwise.
    //   • `temperature` must be the default (1); any other value 400s with
    //     "Unsupported parameter".
    // Local MLX (Gemma, etc.) still expects the classic shape, so we branch
    // on `bearer` — only the OpenAI path sets one.
    let is_openai = backend.bearer.is_some();

    for _ in 0..MAX_STEPS {
        let mut body = json!({
            "model": backend.model,
            "messages": messages,
            "tools": tools_json,
        });
        if is_openai {
            body["max_completion_tokens"] = json!(OPENAI_MAX_COMPLETION_TOKENS);
        } else {
            body["max_tokens"] = json!(MLX_MAX_TOKENS);
            body["temperature"] = json!(0.2);
        }

        let mut req = client.post(&backend.url).json(&body);
        if let Some(ref bearer) = backend.bearer {
            req = req.header("Authorization", format!("Bearer {}", bearer));
        }
        let resp = req
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
        // Gemma often calls create_project with just a name. The preview must
        // mirror the executor's P2 fallback, not print "(?)".
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
