use crate::models::Project;
use crate::AppState;
use serde::Deserialize;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct ProjectFilter {
    pub priorities: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
}

#[tauri::command]
pub fn get_projects(
    state: State<'_, AppState>,
    filter: Option<ProjectFilter>,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut sql = String::from(
        "SELECT id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at FROM projects"
    );
    let mut conditions: Vec<String> = vec![];
    let mut params: Vec<String> = vec![];

    if let Some(ref f) = filter {
        if let Some(ref priorities) = f.priorities {
            if !priorities.is_empty() {
                let placeholders: Vec<String> = priorities
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", params.len() + i + 1))
                    .collect();
                conditions.push(format!("priority IN ({})", placeholders.join(",")));
                params.extend(priorities.clone());
            }
        }
        if let Some(ref categories) = f.categories {
            if !categories.is_empty() {
                let placeholders: Vec<String> = categories
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", params.len() + i + 1))
                    .collect();
                conditions.push(format!("category IN ({})", placeholders.join(",")));
                params.extend(categories.clone());
            }
        }
    }

    if !conditions.is_empty() {
        sql.push_str(" WHERE ");
        sql.push_str(&conditions.join(" AND "));
    }
    sql.push_str(" ORDER BY CASE priority WHEN 'P0' THEN 0 WHEN 'P1' THEN 1 WHEN 'P2' THEN 2 WHEN 'P3' THEN 3 WHEN 'P4' THEN 4 END, sort_order ASC");

    let mut stmt = db.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(Project {
                id: row.get(0)?,
                priority: row.get(1)?,
                number: row.get(2)?,
                name: row.get(3)?,
                category: row.get(4)?,
                path: row.get(5)?,
                evaluation: row.get(6)?,
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectInput {
    pub name: Option<String>,
    pub priority: Option<String>,
    pub category: Option<String>,
    pub path: Option<String>,
    pub evaluation: Option<String>,
}

#[tauri::command]
pub fn update_project(
    state: State<'_, AppState>,
    id: i64,
    fields: UpdateProjectInput,
) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let mut sets: Vec<String> = vec![];
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![];

    if let Some(ref v) = fields.name {
        sets.push("name = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.priority {
        sets.push("priority = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.category {
        sets.push("category = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.path {
        sets.push("path = ?".into());
        params.push(Box::new(v.clone()));
    }
    if let Some(ref v) = fields.evaluation {
        sets.push("evaluation = ?".into());
        params.push(Box::new(v.clone()));
    }

    if sets.is_empty() {
        return Err("No fields to update".into());
    }

    sets.push("updated_at = datetime('now')".into());
    params.push(Box::new(id));

    let sql = format!("UPDATE projects SET {} WHERE id = ?", sets.join(", "));
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    db.execute(&sql, param_refs.as_slice()).map_err(|e| e.to_string())?;

    let project = db
        .query_row(
            "SELECT id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at FROM projects WHERE id = ?1",
            [id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    priority: row.get(1)?,
                    number: row.get(2)?,
                    name: row.get(3)?,
                    category: row.get(4)?,
                    path: row.get(5)?,
                    evaluation: row.get(6)?,
                    sort_order: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    name: String,
    priority: String,
    category: Option<String>,
    path: Option<String>,
) -> Result<Project, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;

    let max_order: i64 = db
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) FROM projects WHERE priority = ?1",
            [&priority],
            |row| row.get(0),
        )
        .unwrap_or(0);

    db.execute(
        "INSERT INTO projects (name, priority, category, path, sort_order) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name, priority, category, path, max_order + 1],
    )
    .map_err(|e| e.to_string())?;

    let id = db.last_insert_rowid();
    let project = db
        .query_row(
            "SELECT id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at FROM projects WHERE id = ?1",
            [id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    priority: row.get(1)?,
                    number: row.get(2)?,
                    name: row.get(3)?,
                    category: row.get(4)?,
                    path: row.get(5)?,
                    evaluation: row.get(6)?,
                    sort_order: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.execute("DELETE FROM projects WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_projects(state: State<'_, AppState>, ids: Vec<i64>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let tx = db.unchecked_transaction().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE projects SET sort_order = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![i as i64, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn search_projects(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Project>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let pattern = format!("%{}%", query);
    let mut stmt = db
        .prepare(
            "SELECT id, priority, number, name, category, path, evaluation, sort_order, created_at, updated_at
             FROM projects
             WHERE name LIKE ?1 OR evaluation LIKE ?1 OR category LIKE ?1
             ORDER BY sort_order ASC",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([&pattern], |row| {
            Ok(Project {
                id: row.get(0)?,
                priority: row.get(1)?,
                number: row.get(2)?,
                name: row.get(3)?,
                category: row.get(4)?,
                path: row.get(5)?,
                evaluation: row.get(6)?,
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
