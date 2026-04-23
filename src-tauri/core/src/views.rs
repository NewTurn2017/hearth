use crate::models::{Memo, Project, Schedule};
use chrono::{Duration, Local};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TodayView {
    pub date: String,
    pub schedules_today: Vec<Schedule>,
    pub p0_projects: Vec<Project>,
    pub recent_memos: Vec<Memo>,
}

pub fn today(conn: &Connection) -> rusqlite::Result<TodayView> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    // schedules today
    let mut stmt = conn.prepare(
        "SELECT id,date,time,location,description,notes,remind_before_5min,remind_at_start,created_at,updated_at
         FROM schedules WHERE date = ?1 ORDER BY COALESCE(time,'') ASC",
    )?;
    let schedules_today: Vec<Schedule> = stmt
        .query_map([&today], |r| {
            Ok(Schedule {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                location: r.get(3)?,
                description: r.get(4)?,
                notes: r.get(5)?,
                remind_before_5min: r.get::<_, i64>(6)? != 0,
                remind_at_start: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // p0 projects
    let mut stmt = conn.prepare(
        "SELECT id,priority,number,name,category,path,evaluation,sort_order,created_at,updated_at
         FROM projects WHERE priority='P0' ORDER BY sort_order ASC",
    )?;
    let p0_projects: Vec<Project> = stmt
        .query_map([], |r| {
            Ok(Project {
                id: r.get(0)?,
                priority: r.get(1)?,
                number: r.get(2)?,
                name: r.get(3)?,
                category: r.get(4)?,
                path: r.get(5)?,
                evaluation: r.get(6)?,
                sort_order: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // recent memos (updated_at within 24h)
    let cutoff = (Local::now() - Duration::hours(24))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let mut stmt = conn.prepare(
        "SELECT id,content,color,project_id,sort_order,created_at,updated_at
         FROM memos WHERE updated_at >= ?1 ORDER BY updated_at DESC LIMIT 10",
    )?;
    let recent_memos: Vec<Memo> = stmt
        .query_map([&cutoff], |r| {
            Ok(Memo {
                id: r.get(0)?,
                content: r.get(1)?,
                color: r.get(2)?,
                project_id: r.get(3)?,
                sort_order: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(TodayView {
        date: today,
        schedules_today,
        p0_projects,
        recent_memos,
    })
}

#[derive(Debug, Serialize)]
pub struct OverdueView {
    pub overdue_schedules: Vec<Schedule>,
    pub stale_projects: Vec<Project>,
}

pub fn overdue(conn: &Connection) -> rusqlite::Result<OverdueView> {
    let today = Local::now().format("%Y-%m-%d").to_string();
    let month_ago = (Local::now() - Duration::days(30))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let mut stmt = conn.prepare(
        "SELECT id,date,time,location,description,notes,remind_before_5min,remind_at_start,created_at,updated_at
         FROM schedules WHERE date < ?1 ORDER BY date DESC LIMIT 50",
    )?;
    let overdue_schedules: Vec<Schedule> = stmt
        .query_map([&today], |r| {
            Ok(Schedule {
                id: r.get(0)?,
                date: r.get(1)?,
                time: r.get(2)?,
                location: r.get(3)?,
                description: r.get(4)?,
                notes: r.get(5)?,
                remind_before_5min: r.get::<_, i64>(6)? != 0,
                remind_at_start: r.get::<_, i64>(7)? != 0,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut stmt = conn.prepare(
        "SELECT id,priority,number,name,category,path,evaluation,sort_order,created_at,updated_at
         FROM projects WHERE updated_at < ?1 ORDER BY updated_at ASC LIMIT 50",
    )?;
    let stale_projects: Vec<Project> = stmt
        .query_map([&month_ago], |r| {
            Ok(Project {
                id: r.get(0)?,
                priority: r.get(1)?,
                number: r.get(2)?,
                name: r.get(3)?,
                category: r.get(4)?,
                path: r.get(5)?,
                evaluation: r.get(6)?,
                sort_order: r.get(7)?,
                created_at: r.get(8)?,
                updated_at: r.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(OverdueView {
        overdue_schedules,
        stale_projects,
    })
}

#[derive(Debug, Serialize)]
pub struct StatsView {
    pub total_projects: i64,
    pub priorities: std::collections::BTreeMap<String, i64>,
    pub categories: std::collections::BTreeMap<String, i64>,
    pub total_memos: i64,
    pub memos_by_color: std::collections::BTreeMap<String, i64>,
    pub total_schedules: i64,
    pub schedules_next_30d: i64,
}

pub fn stats(conn: &Connection) -> rusqlite::Result<StatsView> {
    let total_projects: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
    let mut priorities = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare("SELECT priority, COUNT(*) FROM projects GROUP BY priority")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (p, n) = row?;
        priorities.insert(p, n);
    }
    let mut categories = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare(
        "SELECT COALESCE(category, '(none)'), COUNT(*) FROM projects GROUP BY category",
    )?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (c, n) = row?;
        categories.insert(c, n);
    }
    let total_memos: i64 = conn.query_row("SELECT COUNT(*) FROM memos", [], |r| r.get(0))?;
    let mut memos_by_color = std::collections::BTreeMap::new();
    let mut stmt = conn.prepare("SELECT color, COUNT(*) FROM memos GROUP BY color")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (c, n) = row?;
        memos_by_color.insert(c, n);
    }
    let total_schedules: i64 = conn.query_row("SELECT COUNT(*) FROM schedules", [], |r| r.get(0))?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let plus_30 = (Local::now() + Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();
    let schedules_next_30d: i64 = conn.query_row(
        "SELECT COUNT(*) FROM schedules WHERE date >= ?1 AND date <= ?2",
        [&today, &plus_30],
        |r| r.get(0),
    )?;
    Ok(StatsView {
        total_projects,
        priorities,
        categories,
        total_memos,
        memos_by_color,
        total_schedules,
        schedules_next_30d,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Source;
    use crate::db::init_db;
    use crate::{memos, projects};
    use tempfile::TempDir;

    fn fresh() -> Connection {
        let d = TempDir::new().unwrap();
        let p = d.path().join("t.db");
        std::mem::forget(d);
        init_db(&p).unwrap()
    }

    #[test]
    fn today_returns_p0_projects() {
        let mut c = fresh();
        projects::create(
            &mut c,
            Source::Cli,
            &projects::NewProject {
                name: "urgent",
                priority: "P0",
                category: None,
                path: None,
                evaluation: None,
            },
        )
        .unwrap();
        let v = today(&c).unwrap();
        assert_eq!(v.p0_projects.len(), 1);
    }

    #[test]
    fn stats_counts() {
        let mut c = fresh();
        memos::create(
            &mut c,
            Source::Cli,
            &memos::NewMemo {
                content: "a",
                color: "yellow",
                project_id: None,
            },
        )
        .unwrap();
        let s = stats(&c).unwrap();
        assert_eq!(s.total_memos, 1);
    }
}
