//! Export / Import helpers for hearth workspace data.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{Client, Memo, Project, Schedule};

/// Full workspace dump.
#[derive(Debug, Serialize, Deserialize)]
pub struct Dump {
    pub projects: Vec<Project>,
    pub memos: Vec<Memo>,
    pub schedules: Vec<Schedule>,
    /// Stored as raw JSON values to avoid round-tripping the computed `usage_count` field.
    pub categories: Vec<Value>,
    pub clients: Vec<Client>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audit_log: Option<Vec<crate::audit::AuditEntry>>,
}

/// Produce a full JSON dump of all workspace data.
pub fn export_json(conn: &Connection, include_audit: bool) -> rusqlite::Result<Dump> {
    let projects = crate::projects::list(conn)?;
    let memos = crate::memos::list(conn)?;
    let schedules = crate::schedules::list(conn, None)?;

    // Categories — query raw columns, omit the computed usage_count
    let categories: Vec<Value> = {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, sort_order, created_at, updated_at FROM categories ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "name": row.get::<_, String>(1)?,
                "color": row.get::<_, String>(2)?,
                "sort_order": row.get::<_, i64>(3)?,
                "created_at": row.get::<_, String>(4)?,
                "updated_at": row.get::<_, String>(5)?,
            }))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let clients = crate::clients::list(conn)?;

    let audit_log = if include_audit {
        Some(crate::audit::list(conn, i64::MAX, None, None, true)?)
    } else {
        None
    };

    Ok(Dump {
        projects,
        memos,
        schedules,
        categories,
        clients,
        audit_log,
    })
}

/// Report returned by `import_json_merge`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportReport {
    pub inserted_projects: usize,
    pub inserted_memos: usize,
    pub inserted_schedules: usize,
    pub inserted_categories: usize,
    pub skipped_duplicates: usize,
    pub dry_run: bool,
}

/// Merge a `Dump` into the database. Duplicates are matched by semantic fields
/// and skipped (reported in `skipped_duplicates`). Pass `dry_run = true` to
/// preview without writing anything.
pub fn import_json_merge(
    conn: &mut Connection,
    dump: &Dump,
    dry_run: bool,
) -> rusqlite::Result<ImportReport> {
    let tx = conn.transaction()?;
    let mut report = ImportReport {
        inserted_projects: 0,
        inserted_memos: 0,
        inserted_schedules: 0,
        inserted_categories: 0,
        skipped_duplicates: 0,
        dry_run,
    };

    // --- categories (match by name) ---
    for cat in &dump.categories {
        let name = cat.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM categories WHERE name = ?1",
            [name],
            |r| r.get(0),
        )?;
        if exists > 0 {
            report.skipped_duplicates += 1;
        } else {
            if !dry_run {
                let color = cat
                    .get("color")
                    .and_then(|v| v.as_str())
                    .unwrap_or("#6b7280");
                let sort_order = cat
                    .get("sort_order")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                tx.execute(
                    "INSERT INTO categories (name, color, sort_order) VALUES (?1, ?2, ?3)",
                    rusqlite::params![name, color, sort_order],
                )?;
            }
            report.inserted_categories += 1;
        }
    }

    // --- projects (match by name + priority) ---
    for p in &dump.projects {
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM projects WHERE name = ?1 AND priority = ?2",
            rusqlite::params![p.name, p.priority],
            |r| r.get(0),
        )?;
        if exists > 0 {
            report.skipped_duplicates += 1;
        } else {
            if !dry_run {
                tx.execute(
                    "INSERT INTO projects (priority, number, name, category, path, evaluation, sort_order, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        p.priority,
                        p.number,
                        p.name,
                        p.category,
                        p.path,
                        p.evaluation,
                        p.sort_order,
                        p.created_at,
                        p.updated_at,
                    ],
                )?;
            }
            report.inserted_projects += 1;
        }
    }

    // --- memos (match by content + color) ---
    for m in &dump.memos {
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM memos WHERE content = ?1 AND color = ?2",
            rusqlite::params![m.content, m.color],
            |r| r.get(0),
        )?;
        if exists > 0 {
            report.skipped_duplicates += 1;
        } else {
            if !dry_run {
                tx.execute(
                    "INSERT INTO memos (content, color, project_id, sort_order, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        m.content,
                        m.color,
                        m.project_id,
                        m.sort_order,
                        m.created_at,
                        m.updated_at,
                    ],
                )?;
            }
            report.inserted_memos += 1;
        }
    }

    // --- schedules (match by date + time + description) ---
    for s in &dump.schedules {
        let exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM schedules WHERE date = ?1 AND COALESCE(time,'') = COALESCE(?2,'') AND COALESCE(description,'') = COALESCE(?3,'')",
            rusqlite::params![s.date, s.time, s.description],
            |r| r.get(0),
        )?;
        if exists > 0 {
            report.skipped_duplicates += 1;
        } else {
            if !dry_run {
                tx.execute(
                    "INSERT INTO schedules (date, time, location, description, notes, remind_before_5min, remind_at_start, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    rusqlite::params![
                        s.date,
                        s.time,
                        s.location,
                        s.description,
                        s.notes,
                        s.remind_before_5min as i64,
                        s.remind_at_start as i64,
                        s.created_at,
                        s.updated_at,
                    ],
                )?;
            }
            report.inserted_schedules += 1;
        }
    }

    if dry_run {
        tx.rollback()?;
    } else {
        tx.commit()?;
    }

    Ok(report)
}
