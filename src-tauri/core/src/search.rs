use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SearchHit {
    pub kind: String, // "project" | "memo" | "schedule"
    pub id: i64,
    pub title: String,
    pub snippet: String,
    pub score: f64, // bm25 (lower is better; negate for "higher is better")
}

fn fts_escape(query: &str) -> String {
    // For single-word queries, use prefix matching (word*) so "agent" matches "agents".
    // For multi-word queries, use phrase matching with double quotes.
    let trimmed = query.trim();
    if trimmed.split_whitespace().count() == 1 {
        // Escape special FTS5 chars and use prefix match.
        let safe = trimmed.replace('"', "").replace('*', "").replace('+', "");
        format!("{}*", safe)
    } else {
        let escaped = trimmed.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    }
}

pub fn search_all(conn: &Connection, query: &str, limit: i64) -> rusqlite::Result<Vec<SearchHit>> {
    let q = fts_escape(query);
    let mut hits = Vec::new();

    // Projects
    {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, snippet(projects_fts, 0, '<', '>', '…', 8), bm25(projects_fts)
             FROM projects_fts JOIN projects p ON p.id = projects_fts.rowid
             WHERE projects_fts MATCH ?1 ORDER BY bm25(projects_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "project".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    // Memos
    {
        let mut stmt = conn.prepare(
            "SELECT m.id, substr(m.content,1,40), snippet(memos_fts, 0, '<', '>', '…', 8), bm25(memos_fts)
             FROM memos_fts JOIN memos m ON m.id = memos_fts.rowid
             WHERE memos_fts MATCH ?1 ORDER BY bm25(memos_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "memo".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    // Schedules
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, COALESCE(s.description, s.date), snippet(schedules_fts, 0, '<', '>', '…', 8), bm25(schedules_fts)
             FROM schedules_fts JOIN schedules s ON s.id = schedules_fts.rowid
             WHERE schedules_fts MATCH ?1 ORDER BY bm25(schedules_fts) LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![q, limit], |r| {
            Ok(SearchHit {
                kind: "schedule".into(),
                id: r.get(0)?,
                title: r.get(1)?,
                snippet: r.get(2)?,
                score: r.get(3)?,
            })
        })?;
        for h in rows {
            hits.push(h?);
        }
    }
    hits.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Source;
    use crate::db::init_db;
    use crate::{memos, projects, schedules};
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("t.db");
        std::mem::forget(dir);
        init_db(&path).unwrap()
    }

    #[test]
    fn search_matches_across_scopes() {
        let mut c = fresh();
        projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "Hearth CLI",
                priority: "P1",
                category: Some("Tools"),
                path: None,
                evaluation: Some("agent interface"),
            },
        )
        .unwrap();
        memos::create(
            &mut c,
            Source::Cli,
            &memos::NewMemo {
                content: "note about agents",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        // Search for 'agent'.
        let hits = search_all(&c, "agent", 20).unwrap();
        assert!(hits.iter().any(|h| h.kind == "project"));
        assert!(hits.iter().any(|h| h.kind == "memo"));
    }

    #[test]
    fn search_schedule_by_description() {
        let mut c = fresh();
        schedules::create(
            &mut c,
            Source::Cli,
            &schedules::NewSchedule {
                date: "2026-05-01",
                time: None,
                location: None,
                description: Some("dentist appointment"),
                notes: None,
                remind_before_5min: false,
                remind_at_start: false,
            },
        )
        .unwrap();
        let hits = search_all(&c, "dentist", 20).unwrap();
        assert!(hits.iter().any(|h| h.kind == "schedule"));
    }
}
