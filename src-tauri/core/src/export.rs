//! Export / Import helpers for hearth workspace data.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{Client, Memo, Project, Schedule};

fn memo_tag_names(memo: &Memo) -> Vec<String> {
    memo.tags.iter().map(|tag| tag.name.clone()).collect()
}

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
                let sort_order = cat.get("sort_order").and_then(|v| v.as_i64()).unwrap_or(0);
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
                    "INSERT INTO memos (content, color, project_id, sort_order, font_size, is_bold, focus_x, focus_y, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    rusqlite::params![
                        m.content,
                        m.color,
                        m.project_id,
                        m.sort_order,
                        m.font_size.as_str(),
                        m.is_bold as i64,
                        m.focus_x,
                        m.focus_y,
                        m.created_at,
                        m.updated_at,
                    ],
                )?;
                let memo_id = tx.last_insert_rowid();
                crate::memos::replace_tags(&tx, memo_id, &memo_tag_names(m))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Source;
    use crate::db::init_db;
    use crate::memos::{self, NewMemo};
    use crate::models::MemoFontSize;
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn import_accepts_legacy_memos_without_focus_fields() {
        let legacy = r##"{
            "projects": [],
            "memos": [{
                "id": 1,
                "content": "legacy memo",
                "color": "yellow",
                "project_id": null,
                "sort_order": 1,
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "schedules": [],
            "categories": [],
            "clients": []
        }"##;
        let dump: Dump = serde_json::from_str(legacy).unwrap();
        assert_eq!(dump.memos[0].font_size, MemoFontSize::Normal);
        assert!(!dump.memos[0].is_bold);
        assert!(dump.memos[0].tags.is_empty());

        let mut c = fresh();
        let report = import_json_merge(&mut c, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        let imported = memos::list(&c).unwrap().remove(0);
        assert_eq!(imported.content, "legacy memo");
        assert_eq!(imported.font_size, MemoFontSize::Normal);
        assert!(imported.tags.is_empty());
    }

    #[test]
    fn import_persists_memo_focus_fields_and_tags() {
        let mut source = fresh();
        memos::create(
            &mut source,
            Source::Cli,
            &NewMemo {
                content: "portable",
                color: "blue",
                project_id: None,
                font_size: Some("large"),
                is_bold: Some(true),
                focus_x: Some(0.7),
                focus_y: Some(0.2),
                tag_names: vec!["중요".to_string(), "검토".to_string()],
            },
        )
        .unwrap();
        let dump = export_json(&source, false).unwrap();

        let mut target = fresh();
        let report = import_json_merge(&mut target, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        let imported = memos::list(&target).unwrap().remove(0);
        assert_eq!(imported.font_size, MemoFontSize::Large);
        assert!(imported.is_bold);
        assert_eq!(imported.focus_x, Some(0.7));
        assert_eq!(imported.focus_y, Some(0.2));
        assert_eq!(
            imported
                .tags
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>(),
            vec!["중요", "검토"]
        );
    }
}
