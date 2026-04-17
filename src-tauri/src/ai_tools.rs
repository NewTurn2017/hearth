// Tool registry for the AI agent loop.
//
// Why this file exists: the previous `cmd_ai.rs` forced MLX to emit a custom
// `{reply, actions[]}` JSON shape via `response_format: json_schema`. That
// duplicates the OpenAI tool-calling protocol. MLX (+ gemma-4) emits proper
// `tool_calls` when we advertise real `tools`, so we route everything through
// that standard mechanism. The agent loop in `cmd_ai.rs` calls `specs()` to
// hand MLX the tool list and `execute()` to run a returned call.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

/// Classification that drives the agent loop's behavior for each tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolKind {
    /// Side-effect-free (queries). The agent executes instantly and feeds the
    /// result back so the model can reason over it in the same turn.
    Read,
    /// Mutates persistent state. The loop pauses; the UI raises a confirmation
    /// modal; on approval the server executes and resumes the loop.
    Mutation,
    /// Not a backend operation — changes client UI state (filters, focus).
    /// Passed through verbatim in the final result so React can dispatch it.
    ClientIntent,
}

/// Tool specification advertised to MLX in `tools: [...]`.
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
    pub kind: ToolKind,
}

/// Parsed tool call produced by the model.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Arguments — already JSON-decoded from the model's stringified payload.
    pub arguments: Value,
}

/// The full tool catalog advertised to the model.
pub fn specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "list_projects",
            description: "List all projects the user currently has, with priority and category.",
            parameters: json!({ "type": "object", "properties": {} }),
            kind: ToolKind::Read,
        },
        ToolSpec {
            name: "search_projects",
            description: "Search projects by substring in name/evaluation/category.",
            parameters: json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": { "type": "string" }
                }
            }),
            kind: ToolKind::Read,
        },
        ToolSpec {
            name: "create_project",
            description: "Create a new project. Only `name` is required. priority defaults to P2 when omitted (P0..P4). category (optional): Active|Side|Lab|Tools|Lecture. User can change priority/category later by clicking the chips on the card.",
            parameters: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string" },
                    "priority": { "type": "string", "enum": ["P0","P1","P2","P3","P4"] },
                    "category": { "type": "string", "enum": ["Active","Side","Lab","Tools","Lecture"] },
                    "path": { "type": "string" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "update_project",
            description: "Update fields on an existing project by id. Only include fields that should change.",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "integer" },
                    "name": { "type": "string" },
                    "priority": { "type": "string", "enum": ["P0","P1","P2","P3","P4"] },
                    "category": { "type": "string", "enum": ["Active","Side","Lab","Tools","Lecture"] },
                    "path": { "type": "string" },
                    "evaluation": { "type": "string" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "delete_project",
            description: "Delete a project permanently by id.",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": { "id": { "type": "integer" } }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "set_filter",
            description: "Filter the project list by priority and/or category. Client-side navigation.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "priorities": { "type": "array", "items": { "type": "string" } },
                    "categories": { "type": "array", "items": { "type": "string" } }
                }
            }),
            kind: ToolKind::ClientIntent,
        },

        // ----- memos -----
        ToolSpec {
            name: "list_memos",
            description: "List every sticky-note memo with its id, color, content, and optional linked project id.",
            parameters: json!({ "type": "object", "properties": {} }),
            kind: ToolKind::Read,
        },
        ToolSpec {
            name: "create_memo",
            description: "Create a new memo. Only `content` is required. color defaults to yellow. project_id links the memo to an existing project; if omitted, project_name is resolved by LIKE match on project name (priority-then-sort_order tie-break). Unresolved project_name → saved as 기타; tell the user in that case.",
            parameters: json!({
                "type": "object",
                "required": ["content"],
                "properties": {
                    "content": { "type": "string" },
                    "color": { "type": "string", "enum": ["yellow","pink","blue","green","purple"] },
                    "project_id": {
                        "type": "integer",
                        "description": "연결할 프로젝트 ID (선택). project_name 보다 우선."
                    },
                    "project_name": {
                        "type": "string",
                        "description": "연결할 프로젝트 이름 부분 일치 (LIKE, 선택). project_id 가 없을 때만 사용됨. 매칭 실패 시 기타로 저장."
                    }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "update_memo",
            description: "Update fields on an existing memo by id. Only include fields that should change. Passing project_id=0 is treated as 'detach from project' (sets NULL).",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "integer" },
                    "content": { "type": "string" },
                    "color": { "type": "string", "enum": ["yellow","pink","blue","green","purple"] },
                    "project_id": { "type": "integer" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "delete_memo",
            description: "Delete a memo permanently by id.",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": { "id": { "type": "integer" } }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "update_memo_by_number",
            description: "#N 뱃지 번호로 메모 내용 수정. 번호는 사용자 화면의 현재 #N(전역 sort_order 기준). 작업 전 list_memos로 최신 순서를 조회해 번호를 확정하세요. 범위 밖이면 '#N 메모를 찾을 수 없음' 오류.",
            parameters: json!({
                "type": "object",
                "required": ["number", "content"],
                "properties": {
                    "number": { "type": "integer", "description": "메모 뱃지 번호 (1부터)" },
                    "content": { "type": "string", "description": "새 내용" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "delete_memo_by_number",
            description: "#N 뱃지 번호로 메모 삭제. 작업 전 list_memos로 최신 순서를 조회해 번호를 확정하세요.",
            parameters: json!({
                "type": "object",
                "required": ["number"],
                "properties": {
                    "number": { "type": "integer", "description": "메모 뱃지 번호 (1부터)" }
                }
            }),
            kind: ToolKind::Mutation,
        },

        // ----- schedules (calendar events) -----
        ToolSpec {
            name: "list_schedules",
            description: "List schedule entries, optionally filtered to a single month (YYYY-MM). Without a month, returns everything ordered by date/time.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "month": { "type": "string", "description": "YYYY-MM (e.g. 2026-04)" }
                }
            }),
            kind: ToolKind::Read,
        },
        ToolSpec {
            name: "create_schedule",
            description: "Create a new calendar entry. `date` (YYYY-MM-DD) is required; `time` (HH:MM, 24h) and the text fields are optional.",
            parameters: json!({
                "type": "object",
                "required": ["date"],
                "properties": {
                    "date": { "type": "string", "description": "YYYY-MM-DD" },
                    "time": { "type": "string", "description": "HH:MM (24h)" },
                    "location": { "type": "string" },
                    "description": { "type": "string" },
                    "notes": { "type": "string" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "update_schedule",
            description: "Update fields on an existing schedule by id. Only include fields that should change — absent fields are preserved, not nulled.",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "integer" },
                    "date": { "type": "string", "description": "YYYY-MM-DD" },
                    "time": { "type": "string", "description": "HH:MM (24h)" },
                    "location": { "type": "string" },
                    "description": { "type": "string" },
                    "notes": { "type": "string" }
                }
            }),
            kind: ToolKind::Mutation,
        },
        ToolSpec {
            name: "delete_schedule",
            description: "Delete a schedule permanently by id.",
            parameters: json!({
                "type": "object",
                "required": ["id"],
                "properties": { "id": { "type": "integer" } }
            }),
            kind: ToolKind::Mutation,
        },

        // ----- navigation (client-side, dispatched by React) -----
        ToolSpec {
            name: "switch_tab",
            description: "Switch the main view to projects, calendar, or memos.",
            parameters: json!({
                "type": "object",
                "required": ["tab"],
                "properties": {
                    "tab": { "type": "string", "enum": ["projects","calendar","memos"] }
                }
            }),
            kind: ToolKind::ClientIntent,
        },
    ]
}

/// Fast lookup of a tool's kind by name. Returns None if the model hallucinated
/// a tool we don't support.
pub fn kind_of(name: &str) -> Option<ToolKind> {
    specs().into_iter().find(|s| s.name == name).map(|s| s.kind)
}

/// Execute a backend tool (Read or Mutation). Returns the JSON the model will
/// see. ClientIntent tools are NOT dispatched here — the agent loop hands them
/// back to the client verbatim.
pub fn execute(app: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    match call.name.as_str() {
        "list_projects" => list_projects(app),
        "search_projects" => search_projects(app, &call.arguments),
        "create_project" => create_project(app, &call.arguments),
        "update_project" => update_project(app, &call.arguments),
        "delete_project" => delete_project(app, &call.arguments),
        "list_memos" => list_memos(app),
        "create_memo" => create_memo(app, &call.arguments),
        "update_memo" => update_memo(app, &call.arguments),
        "delete_memo" => delete_memo(app, &call.arguments),
        "update_memo_by_number" => update_memo_by_number(app, &call.arguments),
        "delete_memo_by_number" => delete_memo_by_number(app, &call.arguments),
        "list_schedules" => list_schedules(app, &call.arguments),
        "create_schedule" => create_schedule(app, &call.arguments),
        "update_schedule" => update_schedule(app, &call.arguments),
        "delete_schedule" => delete_schedule(app, &call.arguments),
        other => Err(format!("unknown or non-backend tool: {}", other)),
    }
}

// ----- tool implementations -----
//
// We replicate the minimal SQL needed here rather than calling the
// `#[tauri::command]` handlers in cmd_projects.rs — those take `State<_>` which
// can't be synthesized from regular Rust. Each fn grabs `state` as a named
// local so the returned `MutexGuard` can borrow from it (a temporary from
// `app.state::<AppState>()` would be dropped before the guard).

fn list_projects(app: &AppHandle) -> Result<Value, String> {
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, priority, name, category FROM projects \
             ORDER BY CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 \
             WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 END, sort_order ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "priority": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "category": row.get::<_, Option<String>>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let items: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
    Ok(json!({ "count": items.len(), "projects": items }))
}

fn search_projects(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("missing query")?;
    let pattern = format!("%{}%", query);
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare(
            "SELECT id, priority, name, category FROM projects \
             WHERE name LIKE ?1 OR evaluation LIKE ?1 OR category LIKE ?1 \
             ORDER BY sort_order ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([&pattern], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "priority": row.get::<_, String>(1)?,
                "name": row.get::<_, String>(2)?,
                "category": row.get::<_, Option<String>>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let items: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
    Ok(json!({ "count": items.len(), "projects": items }))
}

fn create_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing name")?
        .trim();
    // Priority is optional at the agent boundary — Gemma often omits it for
    // casual "X 프로젝트 만들어줘" requests. Users can reclassify via the
    // chip picker on the card, so default to the middle (P2) and accept it.
    let priority = args
        .get("priority")
        .and_then(|v| v.as_str())
        .unwrap_or("P2");
    let category = args.get("category").and_then(|v| v.as_str());
    let path = args.get("path").and_then(|v| v.as_str());

    if name.is_empty() {
        return Err("name must not be empty".into());
    }
    if !matches!(priority, "P0" | "P1" | "P2" | "P3" | "P4") {
        return Err(format!("invalid priority: {}", priority));
    }

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM projects WHERE priority = ?1",
            [priority],
            |row| row.get(0),
        )
        .unwrap_or(0);
    db.execute(
        "INSERT INTO projects (name, priority, category, path, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name, priority, category, path, max_order + 1],
    )
    .map_err(|e| e.to_string())?;
    let id = db.last_insert_rowid();
    Ok(json!({
        "ok": true,
        "id": id,
        "name": name,
        "priority": priority,
    }))
}

fn update_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;
    let mut sets: Vec<&str> = vec![];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    for key in ["name", "priority", "category", "path", "evaluation"] {
        if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
            sets.push(match key {
                "name" => "name = ?",
                "priority" => "priority = ?",
                "category" => "category = ?",
                "path" => "path = ?",
                "evaluation" => "evaluation = ?",
                _ => unreachable!(),
            });
            params.push(Box::new(v.to_string()));
        }
    }
    if sets.is_empty() {
        return Err("no fields to update".into());
    }
    params.push(Box::new(id));
    let sql = format!(
        "UPDATE projects SET {}, updated_at = datetime('now') WHERE id = ?",
        sets.join(", ")
    );

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changed = db.execute(&sql, refs.as_slice()).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no project with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}

fn delete_project(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let changed = db
        .execute("DELETE FROM projects WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no project with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}

// ----- memo implementations -----

/// Validated memo palette. Mirrors `MEMO_COLORS` on the TS side — keep in
/// sync when adding swatches, otherwise the AI can emit a color the UI
/// can't render.
const MEMO_COLORS: &[&str] = &["yellow", "pink", "blue", "green", "purple"];

fn list_memos(app: &AppHandle) -> Result<Value, String> {
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .prepare("SELECT id, content, color, project_id FROM memos ORDER BY sort_order ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                // Truncate in the tool response so long memos don't blow up
                // the context window — the UI still shows full content.
                "content": row.get::<_, String>(1)?.chars().take(200).collect::<String>(),
                "color": row.get::<_, String>(2)?,
                "project_id": row.get::<_, Option<i64>>(3)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    let items: Vec<Value> = rows.filter_map(|r| r.ok()).collect();
    Ok(json!({ "count": items.len(), "memos": items }))
}

fn create_memo(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("missing content")?;
    // `create_memo` accepts empty content at the UI level (blank sticky you
    // fill in later), but when the agent creates one it should always carry
    // text — otherwise the user has no way to tell what was just added.
    if content.trim().is_empty() {
        return Err("content must not be empty".into());
    }
    let color = args
        .get("color")
        .and_then(|v| v.as_str())
        .unwrap_or("yellow");
    if !MEMO_COLORS.contains(&color) {
        return Err(format!("invalid color: {}", color));
    }

    // project_id wins when present. Otherwise resolve project_name by
    // case-insensitive LIKE on the project name, tie-breaking by priority
    // (P0 → P4) then by the user's sort_order, so "WithGenie" matches the
    // top-of-stack project when multiple share the fragment. An unresolved
    // project_name is NOT an error — the memo lands in 기타 and the returned
    // project_id being null signals the miss to the agent.
    let explicit_id = args.get("project_id").and_then(|v| v.as_i64());
    let requested_name = args
        .get("project_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let project_id: Option<i64> = if let Some(pid) = explicit_id {
        Some(pid)
    } else if let Some(name) = requested_name {
        let pattern = format!("%{}%", name);
        db.query_row(
            "SELECT id FROM projects WHERE name LIKE ?1 ORDER BY \
             CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 \
                           WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 ELSE 5 END, \
             sort_order LIMIT 1",
            [&pattern],
            |r| r.get::<_, i64>(0),
        )
        .ok()
    } else {
        None
    };

    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM memos",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    db.execute(
        "INSERT INTO memos (content, color, project_id, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![content, color, project_id, max_order + 1],
    )
    .map_err(|e| e.to_string())?;
    let id = db.last_insert_rowid();
    let resolved_as_etc = requested_name.is_some() && explicit_id.is_none() && project_id.is_none();
    Ok(json!({
        "ok": true,
        "id": id,
        "color": color,
        "project_id": project_id,
        "resolved_as_etc": resolved_as_etc,
    }))
}

fn update_memo(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;
    let mut sets: Vec<&str> = vec![];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(content) = args.get("content").and_then(|v| v.as_str()) {
        sets.push("content = ?");
        params.push(Box::new(content.to_string()));
    }
    if let Some(color) = args.get("color").and_then(|v| v.as_str()) {
        if !MEMO_COLORS.contains(&color) {
            return Err(format!("invalid color: {}", color));
        }
        sets.push("color = ?");
        params.push(Box::new(color.to_string()));
    }
    // `project_id: 0` is the agent's idiom for "detach" — map to SQL NULL so
    // the memo shows up unlinked. Any positive id is taken as a concrete FK.
    if let Some(pid) = args.get("project_id").and_then(|v| v.as_i64()) {
        sets.push("project_id = ?");
        let value: Option<i64> = if pid > 0 { Some(pid) } else { None };
        params.push(Box::new(value));
    }
    if sets.is_empty() {
        return Err("no fields to update".into());
    }
    params.push(Box::new(id));
    let sql = format!(
        "UPDATE memos SET {}, updated_at = datetime('now') WHERE id = ?",
        sets.join(", ")
    );

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changed = db.execute(&sql, refs.as_slice()).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no memo with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}

fn delete_memo(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let changed = db
        .execute("DELETE FROM memos WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no memo with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}

/// Resolve `#N` → memo id via sort_order OFFSET, matching the UI badge.
/// Centralized so update/delete share one error path.
fn resolve_memo_by_number(
    db: &rusqlite::Connection,
    number: i64,
) -> Result<i64, String> {
    if number < 1 {
        return Err(format!("#{} 메모를 찾을 수 없음", number));
    }
    db.query_row(
        "SELECT id FROM memos ORDER BY sort_order LIMIT 1 OFFSET ?1",
        [number - 1],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|_| format!("#{} 메모를 찾을 수 없음", number))
}

fn update_memo_by_number(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_i64())
        .ok_or("missing number")?;
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or("missing content")?
        .to_string();

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = resolve_memo_by_number(&db, number)?;
    db.execute(
        "UPDATE memos SET content = ?1, updated_at = datetime('now') WHERE id = ?2",
        rusqlite::params![content, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "id": id, "number": number }))
}

fn delete_memo_by_number(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_i64())
        .ok_or("missing number")?;
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let id = resolve_memo_by_number(&db, number)?;
    db.execute("DELETE FROM memos WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(json!({ "ok": true, "id": id, "number": number }))
}

// ----- schedule implementations -----

/// Full calendar validation for YYYY-MM-DD via chrono. SQLite stores the
/// string verbatim, so once a row is persisted the app will try to parse it
/// in the frontend — values like "2026-13-45" pass a shape check but yield
/// Invalid Date in JS. Chrono rejects them at the boundary.
fn looks_like_date(s: &str) -> bool {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
}

/// Full calendar validation for YYYY-MM. We synthesize day=01 and defer to
/// chrono so bogus months (00, 13+) are rejected alongside obvious shape
/// errors.
fn looks_like_month(s: &str) -> bool {
    NaiveDate::parse_from_str(&format!("{}-01", s), "%Y-%m-%d").is_ok()
}

fn list_schedules(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let month = args.get("month").and_then(|v| v.as_str());
    if let Some(m) = month {
        if !looks_like_month(m) {
            return Err(format!("month must be YYYY-MM, got '{}'", m));
        }
    }

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let (sql, pattern) = match month {
        Some(m) => (
            "SELECT id, date, time, location, description FROM schedules \
             WHERE date LIKE ?1 ORDER BY date, time"
                .to_string(),
            Some(format!("{}%", m)),
        ),
        None => (
            "SELECT id, date, time, location, description FROM schedules \
             ORDER BY date, time"
                .to_string(),
            None,
        ),
    };

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let map_row = |row: &rusqlite::Row| -> rusqlite::Result<Value> {
        Ok(json!({
            "id": row.get::<_, i64>(0)?,
            "date": row.get::<_, String>(1)?,
            "time": row.get::<_, Option<String>>(2)?,
            "location": row.get::<_, Option<String>>(3)?,
            "description": row.get::<_, Option<String>>(4)?,
        }))
    };
    let rows = if let Some(pat) = pattern {
        stmt.query_map([pat], map_row).map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>()
    } else {
        stmt.query_map([], map_row).map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>()
    };
    Ok(json!({ "count": rows.len(), "schedules": rows }))
}

fn create_schedule(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let date = args
        .get("date")
        .and_then(|v| v.as_str())
        .ok_or("missing date")?;
    if !looks_like_date(date) {
        return Err(format!("date must be YYYY-MM-DD, got '{}'", date));
    }
    let time = args.get("time").and_then(|v| v.as_str());
    let location = args.get("location").and_then(|v| v.as_str());
    let description = args.get("description").and_then(|v| v.as_str());
    let notes = args.get("notes").and_then(|v| v.as_str());

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute(
        "INSERT INTO schedules (date, time, location, description, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![date, time, location, description, notes],
    )
    .map_err(|e| e.to_string())?;
    let id = db.last_insert_rowid();
    Ok(json!({ "ok": true, "id": id, "date": date }))
}

fn update_schedule(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;

    // Partial update: only touch columns the model explicitly named. The HTTP
    // `cmd_schedules::update_schedule` handler does full-row replacement (to
    // match the ScheduleModal's form-submit semantics), but the agent
    // boundary is different — the model typically wants "edit just the time"
    // and has no way to know it would null out the other columns without
    // round-tripping through list_schedules first. Partial semantics match
    // update_memo / update_project.
    let mut sets: Vec<&str> = vec![];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(d) = args.get("date").and_then(|v| v.as_str()) {
        if !looks_like_date(d) {
            return Err(format!("date must be YYYY-MM-DD, got '{}'", d));
        }
        sets.push("date = ?");
        params.push(Box::new(d.to_string()));
    }
    for key in ["time", "location", "description", "notes"] {
        if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
            sets.push(match key {
                "time" => "time = ?",
                "location" => "location = ?",
                "description" => "description = ?",
                "notes" => "notes = ?",
                _ => unreachable!(),
            });
            params.push(Box::new(v.to_string()));
        }
    }
    if sets.is_empty() {
        return Err("no fields to update".into());
    }
    params.push(Box::new(id));
    let sql = format!(
        "UPDATE schedules SET {}, updated_at = datetime('now') WHERE id = ?",
        sets.join(", ")
    );

    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let changed = db.execute(&sql, refs.as_slice()).map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no schedule with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}

fn delete_schedule(app: &AppHandle, args: &Value) -> Result<Value, String> {
    let id = args.get("id").and_then(|v| v.as_i64()).ok_or("missing id")?;
    let state = app.state::<crate::AppState>();
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let changed = db
        .execute("DELETE FROM schedules WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err(format!("no schedule with id {}", id));
    }
    Ok(json!({ "ok": true, "id": id }))
}
