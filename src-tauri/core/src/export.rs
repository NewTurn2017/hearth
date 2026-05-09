//! Export / Import helpers for hearth workspace data.

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::models::{Client, Memo, Project, Schedule};

fn memo_tag_names(memo: &Memo) -> Vec<String> {
    memo.tags.iter().map(|tag| tag.name.clone()).collect()
}

fn export_memo_tags(conn: &Connection) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, sort_order, created_at, updated_at
         FROM memo_tags ORDER BY sort_order ASC, name ASC",
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
    rows.collect()
}

fn export_memo_tag_links(conn: &Connection) -> rusqlite::Result<Vec<Value>> {
    let mut stmt = conn
        .prepare("SELECT memo_id, tag_id FROM memo_tag_links ORDER BY memo_id ASC, tag_id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "memo_id": row.get::<_, i64>(0)?,
            "tag_id": row.get::<_, i64>(1)?,
        }))
    })?;
    rows.collect()
}

fn normalized_memo_tag_names(memo: &Memo) -> Vec<String> {
    let mut out = Vec::new();
    for raw in memo_tag_names(memo) {
        let name = raw.trim();
        if name.is_empty() || out.iter().any(|existing| existing == name) {
            continue;
        }
        out.push(name.to_string());
    }
    out
}

fn next_memo_tag_sort_order(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM memo_tags",
        [],
        |r| r.get(0),
    )
}

enum ResolvedMemoTag {
    Existing(i64),
    Inserted(Option<i64>),
}

impl ResolvedMemoTag {
    fn id(&self) -> Option<i64> {
        match self {
            ResolvedMemoTag::Existing(id) => Some(*id),
            ResolvedMemoTag::Inserted(id) => *id,
        }
    }
}

fn resolve_memo_tag_for_import(
    conn: &Connection,
    name: &str,
    color: Option<&str>,
    sort_order: Option<i64>,
    dry_run: bool,
    report: &mut ImportReport,
) -> rusqlite::Result<ResolvedMemoTag> {
    match conn.query_row("SELECT id FROM memo_tags WHERE name = ?1", [name], |r| {
        r.get::<_, i64>(0)
    }) {
        Ok(id) => {
            report.skipped_duplicates += 1;
            Ok(ResolvedMemoTag::Existing(id))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            report.inserted_memo_tags += 1;
            if dry_run {
                Ok(ResolvedMemoTag::Inserted(None))
            } else {
                conn.execute(
                    "INSERT INTO memo_tags (name, color, sort_order) VALUES (?1, ?2, ?3)",
                    rusqlite::params![
                        name,
                        color.unwrap_or("#6b7280"),
                        sort_order.unwrap_or(next_memo_tag_sort_order(conn)?)
                    ],
                )?;
                Ok(ResolvedMemoTag::Inserted(Some(conn.last_insert_rowid())))
            }
        }
        Err(e) => Err(e),
    }
}

fn insert_memo_tag_link_for_import(
    conn: &Connection,
    memo_id: i64,
    tag_id: i64,
    dry_run: bool,
    report: &mut ImportReport,
) -> rusqlite::Result<()> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM memo_tag_links WHERE memo_id = ?1 AND tag_id = ?2",
        rusqlite::params![memo_id, tag_id],
        |r| r.get(0),
    )?;
    if exists > 0 {
        report.skipped_duplicates += 1;
    } else {
        if !dry_run {
            conn.execute(
                "INSERT INTO memo_tag_links (memo_id, tag_id) VALUES (?1, ?2)",
                rusqlite::params![memo_id, tag_id],
            )?;
        }
        report.inserted_memo_tag_links += 1;
    }
    Ok(())
}

fn import_nested_memo_tags(
    conn: &Connection,
    memo_id: Option<i64>,
    memo: &Memo,
    dry_run: bool,
    report: &mut ImportReport,
) -> rusqlite::Result<()> {
    for name in normalized_memo_tag_names(memo) {
        let tag = resolve_memo_tag_for_import(conn, &name, None, None, dry_run, report)?;
        if let Some(memo_id) = memo_id {
            if let Some(tag_id) = tag.id() {
                insert_memo_tag_link_for_import(conn, memo_id, tag_id, dry_run, report)?;
            } else {
                // Dry-run path for a would-be new tag: no stable local id exists yet, but
                // importing this nested tag would create exactly one memo_tag_link.
                report.inserted_memo_tag_links += 1;
            }
        }
    }
    Ok(())
}

/// Full workspace dump.
#[derive(Debug, Serialize, Deserialize)]
pub struct Dump {
    pub projects: Vec<Project>,
    pub memos: Vec<Memo>,
    pub schedules: Vec<Schedule>,
    /// Stored as raw JSON values to avoid round-tripping the computed `usage_count` field.
    pub categories: Vec<Value>,
    #[serde(default)]
    pub memo_tags: Vec<Value>,
    #[serde(default)]
    pub memo_tag_links: Vec<Value>,
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

    let memo_tags = export_memo_tags(conn)?;
    let memo_tag_links = export_memo_tag_links(conn)?;

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
        memo_tags,
        memo_tag_links,
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
    pub inserted_memo_tags: usize,
    pub inserted_memo_tag_links: usize,
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
        inserted_memo_tags: 0,
        inserted_memo_tag_links: 0,
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

    // --- memo tags (match by semantic name, preserve metadata) ---
    let mut memo_tag_id_map: HashMap<i64, i64> = HashMap::new();
    let mut inserted_memo_tag_source_ids: HashSet<i64> = HashSet::new();
    for tag in &dump.memo_tags {
        let old_id = tag.get("id").and_then(|v| v.as_i64());
        let name = tag
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if name.is_empty() {
            continue;
        }
        let color = tag
            .get("color")
            .and_then(|v| v.as_str())
            .unwrap_or("#6b7280");
        let sort_order = tag.get("sort_order").and_then(|v| v.as_i64()).unwrap_or(0);
        let tag = resolve_memo_tag_for_import(
            &tx,
            name,
            Some(color),
            Some(sort_order),
            dry_run,
            &mut report,
        )?;
        match (old_id, tag) {
            (Some(old_id), ResolvedMemoTag::Existing(tag_id)) => {
                memo_tag_id_map.insert(old_id, tag_id);
            }
            (Some(old_id), ResolvedMemoTag::Inserted(Some(tag_id))) => {
                memo_tag_id_map.insert(old_id, tag_id);
                inserted_memo_tag_source_ids.insert(old_id);
            }
            (Some(old_id), ResolvedMemoTag::Inserted(None)) if dry_run => {
                memo_tag_id_map.insert(old_id, old_id);
                inserted_memo_tag_source_ids.insert(old_id);
            }
            _ => {}
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
    let mut memo_id_map: HashMap<i64, i64> = HashMap::new();
    let mut inserted_memo_source_ids: HashSet<i64> = HashSet::new();
    for m in &dump.memos {
        let existing_id = tx.query_row(
            "SELECT id FROM memos WHERE content = ?1 AND color = ?2",
            rusqlite::params![m.content, m.color],
            |r| r.get::<_, i64>(0),
        );
        if let Ok(existing_id) = existing_id {
            memo_id_map.insert(m.id, existing_id);
            report.skipped_duplicates += 1;
        } else if matches!(existing_id, Err(rusqlite::Error::QueryReturnedNoRows)) {
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
                memo_id_map.insert(m.id, memo_id);
            } else {
                memo_id_map.insert(m.id, m.id);
            }
            inserted_memo_source_ids.insert(m.id);
            report.inserted_memos += 1;
        } else if let Err(e) = existing_id {
            return Err(e);
        }
    }

    if dump.memo_tag_links.is_empty() {
        // Legacy/nested-only dumps have tags on each memo but no raw link section.
        for memo in &dump.memos {
            import_nested_memo_tags(
                &tx,
                memo_id_map.get(&memo.id).copied(),
                memo,
                dry_run,
                &mut report,
            )?;
        }
    } else {
        // Raw link sections are the authoritative link-import/counting path for new dumps.
        let mut planned_links = HashSet::new();
        for link in &dump.memo_tag_links {
            let old_memo_id = link.get("memo_id").and_then(|v| v.as_i64());
            let old_tag_id = link.get("tag_id").and_then(|v| v.as_i64());
            let (Some(old_memo_id), Some(old_tag_id)) = (old_memo_id, old_tag_id) else {
                continue;
            };
            let Some(&memo_id) = memo_id_map.get(&old_memo_id) else {
                continue;
            };
            let Some(&tag_id) = memo_tag_id_map.get(&old_tag_id) else {
                continue;
            };
            if dry_run
                && (inserted_memo_source_ids.contains(&old_memo_id)
                    || inserted_memo_tag_source_ids.contains(&old_tag_id))
            {
                if planned_links.insert((old_memo_id, old_tag_id)) {
                    report.inserted_memo_tag_links += 1;
                } else {
                    report.skipped_duplicates += 1;
                }
            } else {
                insert_memo_tag_link_for_import(&tx, memo_id, tag_id, dry_run, &mut report)?;
            }
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
    fn export_import_roundtrips_raw_memo_tag_sections_and_metadata() {
        let mut source = fresh();
        let tag =
            memos::create_memo_tag(&mut source, Source::Cli, "휴대", Some("#123456")).unwrap();
        memos::update_memo_tag(
            &mut source,
            Source::Cli,
            tag.id,
            &memos::UpdateMemoTag {
                name: None,
                color: Some("#abcdef"),
                sort_order: Some(42),
            },
        )
        .unwrap();
        memos::create(
            &mut source,
            Source::Cli,
            &NewMemo {
                content: "raw portable",
                color: "green",
                project_id: None,
                font_size: Some("small"),
                is_bold: Some(true),
                focus_x: Some(0.31),
                focus_y: Some(0.62),
                tag_names: vec!["휴대".to_string()],
            },
        )
        .unwrap();

        let dump = export_json(&source, false).unwrap();
        assert!(dump.memo_tags.iter().any(|tag| tag["name"] == "휴대"));
        assert_eq!(dump.memo_tag_links.len(), 1);

        let mut dry_run_target = fresh();
        let dry_run = import_json_merge(&mut dry_run_target, &dump, true).unwrap();
        assert_eq!(dry_run.inserted_memos, 1);
        assert_eq!(dry_run.inserted_memo_tags, 1);
        assert_eq!(dry_run.inserted_memo_tag_links, 1);
        assert!(memos::list(&dry_run_target).unwrap().is_empty());

        let mut target = fresh();
        let report = import_json_merge(&mut target, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        assert_eq!(report.inserted_memo_tags, 1);
        assert_eq!(report.inserted_memo_tag_links, 1);
        let imported = memos::list(&target).unwrap().remove(0);
        assert_eq!(imported.font_size, MemoFontSize::Small);
        assert!(imported.is_bold);
        assert_eq!(imported.focus_x, Some(0.31));
        assert_eq!(imported.focus_y, Some(0.62));
        assert_eq!(imported.tags[0].name, "휴대");
        assert_eq!(imported.tags[0].color, "#abcdef");
        assert_eq!(imported.tags[0].sort_order, 42);
    }

    #[test]
    fn import_uses_raw_memo_tag_links_when_nested_tags_are_absent() {
        let raw = r##"{
            "projects": [],
            "memos": [{
                "id": 7,
                "content": "raw link memo",
                "color": "yellow",
                "project_id": null,
                "sort_order": 1,
                "font_size": "large",
                "is_bold": true,
                "focus_x": 0.4,
                "focus_y": 0.8,
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "schedules": [],
            "categories": [],
            "memo_tags": [{
                "id": 11,
                "name": "원시",
                "color": "#654321",
                "sort_order": 3,
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "memo_tag_links": [{"memo_id": 7, "tag_id": 11}],
            "clients": []
        }"##;
        let dump: Dump = serde_json::from_str(raw).unwrap();
        let mut c = fresh();
        let report = import_json_merge(&mut c, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        assert_eq!(report.inserted_memo_tags, 1);
        assert_eq!(report.inserted_memo_tag_links, 1);
        let imported = memos::list(&c).unwrap().remove(0);
        assert_eq!(imported.tags[0].name, "원시");
        assert_eq!(imported.tags[0].color, "#654321");
    }

    #[test]
    fn import_preserves_existing_same_name_memo_tag_metadata() {
        let raw = r##"{
            "projects": [],
            "memos": [{
                "id": 31,
                "content": "existing tag memo",
                "color": "yellow",
                "project_id": null,
                "sort_order": 1,
                "font_size": "normal",
                "is_bold": false,
                "focus_x": null,
                "focus_y": null,
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "schedules": [],
            "categories": [],
            "memo_tags": [{
                "id": 41,
                "name": "로컬태그",
                "color": "#imported",
                "sort_order": 99,
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "memo_tag_links": [{"memo_id": 31, "tag_id": 41}],
            "clients": []
        }"##;
        let dump: Dump = serde_json::from_str(raw).unwrap();
        let mut c = fresh();
        memos::create_memo_tag(&mut c, Source::Cli, "로컬태그", Some("#local")).unwrap();
        let local_id = memos::list_memo_tags(&c)
            .unwrap()
            .into_iter()
            .find(|tag| tag.name == "로컬태그")
            .unwrap()
            .id;
        let local = memos::update_memo_tag(
            &mut c,
            Source::Cli,
            local_id,
            &memos::UpdateMemoTag {
                name: None,
                color: None,
                sort_order: Some(7),
            },
        )
        .unwrap();

        let report = import_json_merge(&mut c, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        assert_eq!(report.inserted_memo_tags, 0);
        assert_eq!(report.inserted_memo_tag_links, 1);
        assert!(report.skipped_duplicates >= 1);

        let tag = memos::list_memo_tags(&c)
            .unwrap()
            .into_iter()
            .find(|tag| tag.id == local.id)
            .unwrap();
        assert_eq!(tag.color, "#local");
        assert_eq!(tag.sort_order, 7);
        let imported = memos::list(&c).unwrap().remove(0);
        assert_eq!(imported.tags[0].name, "로컬태그");
        assert_eq!(imported.tags[0].color, "#local");
    }

    #[test]
    fn import_counts_nested_only_memo_tags_and_links() {
        let nested = r##"{
            "projects": [],
            "memos": [{
                "id": 21,
                "content": "nested only memo",
                "color": "pink",
                "project_id": null,
                "sort_order": 1,
                "font_size": "normal",
                "is_bold": false,
                "focus_x": null,
                "focus_y": null,
                "tags": [{
                    "id": 99,
                    "name": "새태그",
                    "color": "#999999",
                    "sort_order": 0,
                    "usage_count": 1,
                    "created_at": "2026-05-06 00:00:00",
                    "updated_at": "2026-05-06 00:00:00"
                }],
                "created_at": "2026-05-06 00:00:00",
                "updated_at": "2026-05-06 00:00:00"
            }],
            "schedules": [],
            "categories": [],
            "clients": []
        }"##;
        let dump: Dump = serde_json::from_str(nested).unwrap();
        assert!(dump.memo_tags.is_empty());
        assert!(dump.memo_tag_links.is_empty());

        let mut dry_run_target = fresh();
        let dry_run = import_json_merge(&mut dry_run_target, &dump, true).unwrap();
        assert_eq!(dry_run.inserted_memos, 1);
        assert_eq!(dry_run.inserted_memo_tags, 1);
        assert_eq!(dry_run.inserted_memo_tag_links, 1);
        assert!(memos::list(&dry_run_target).unwrap().is_empty());

        let mut c = fresh();
        let report = import_json_merge(&mut c, &dump, false).unwrap();
        assert_eq!(report.inserted_memos, 1);
        assert_eq!(report.inserted_memo_tags, 1);
        assert_eq!(report.inserted_memo_tag_links, 1);
        let imported = memos::list(&c).unwrap().remove(0);
        assert_eq!(imported.tags[0].name, "새태그");
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
