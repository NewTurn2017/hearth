use crate::cmd_settings::{self, K_BACKUP_DIR};
use crate::AppState;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, State};

/// Resolve the directory that holds rolling backups. Reads `backup.dir` from
/// the settings KV; falls back to `$APP_DATA/backups` when unset so first-run
/// behavior matches the pre-setting world.
fn backup_dir(app: &AppHandle, state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let configured = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::read(&db, K_BACKUP_DIR)?
    };
    let dir = if configured.is_empty() {
        app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("backups")
    } else {
        PathBuf::from(configured)
    };
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// GFS-style retention. From the newest backup down, each file lands in
/// exactly one bucket based on its age in days:
///   •  0..7    → day bucket (one per calendar date — keeps the most recent
///                close on each of the last seven days)
///   •  7..35   → week bucket (one per ISO week — four weeks of coverage
///                beyond the daily window)
///   • 35..125  → month bucket (one per calendar month — three months beyond
///                the weekly window)
///   • older    → deleted
/// The newest file in each bucket wins; everything else is pruned. Only
/// `hearth-backup-*.db` is touched; `pre-reset-*.db` snapshots are preserved
/// so a reset can always be undone via the Restore list.
fn prune_backups_with_today(dir: &Path, today: NaiveDate) {
    let entries: Vec<(PathBuf, NaiveDateTime)> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let stripped =
                    name.strip_prefix("hearth-backup-")?.strip_suffix(".db")?;
                let ts =
                    NaiveDateTime::parse_from_str(stripped, "%Y-%m-%d-%H%M%S").ok()?;
                Some((e.path(), ts))
            })
            .collect(),
        Err(_) => return,
    };

    let mut sorted = entries.clone();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    let mut seen: HashSet<String> = HashSet::new();
    let mut keep: HashSet<PathBuf> = HashSet::new();

    for (path, ts) in &sorted {
        let d = ts.date();
        // (today - d) can be negative from clock skew on a filename from
        // "the future" — treat those as age 0 so they land in the daily
        // bucket and are not dropped.
        let age = (today - d).num_days().max(0);
        let key = if age < 7 {
            format!("d:{d}")
        } else if age < 35 {
            let w = d.iso_week();
            format!("w:{}-{}", w.year(), w.week())
        } else if age < 125 {
            format!("m:{}-{:02}", d.year(), d.month())
        } else {
            continue;
        };
        if seen.insert(key) {
            keep.insert(path.clone());
        }
    }

    for (path, _) in entries {
        if !keep.contains(&path) {
            let _ = fs::remove_file(&path);
        }
    }
}

fn prune_backups(dir: &Path) {
    prune_backups_with_today(dir, Local::now().date_naive());
}

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("data.db"))
}

#[tauri::command]
pub fn get_backup_dir(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    Ok(backup_dir(&app, &state)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn set_backup_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let canonical = PathBuf::from(path.trim());
    if canonical.as_os_str().is_empty() {
        return Err("백업 위치가 비어 있습니다".into());
    }
    fs::create_dir_all(&canonical)
        .map_err(|e| format!("백업 폴더를 만들 수 없습니다: {e}"))?;
    let stored = canonical.to_string_lossy().to_string();
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        cmd_settings::write(&db, K_BACKUP_DIR, &stored)?;
    }
    Ok(stored)
}

#[tauri::command]
pub fn backup_db(
    state: State<'_, AppState>,
    app: AppHandle,
    dest_path: Option<String>,
) -> Result<String, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }

    let source = db_path(&app)?;
    let dir = backup_dir(&app, &state)?;
    let dest = match dest_path {
        Some(p) => PathBuf::from(p),
        None => {
            let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
            dir.join(format!("hearth-backup-{}.db", timestamp))
        }
    };

    fs::copy(&source, &dest).map_err(|e| format!("Backup failed: {}", e))?;

    prune_backups(&dir);

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn restore_db(app: AppHandle, src_path: String) -> Result<(), String> {
    let source = PathBuf::from(&src_path);
    if !source.exists() {
        return Err("Backup file not found".into());
    }
    let dest = db_path(&app)?;
    fs::copy(&source, &dest).map_err(|e| format!("Restore failed: {}", e))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created: String,
}

#[tauri::command]
pub fn list_backups(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir(&app, &state)?;
    let mut backups: Vec<BackupInfo> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            // `hearth-backup-*.db` → regular rolling backups
            // `pre-reset-*.db`     → snapshots captured right before a reset
            n.starts_with("hearth-backup-") || n.starts_with("pre-reset-")
        })
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let modified = meta
                .modified()
                .ok()
                .map(|t| {
                    let dt: chrono::DateTime<Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            Some(BackupInfo {
                path: e.path().to_string_lossy().to_string(),
                filename: e.file_name().to_string_lossy().to_string(),
                size_bytes: meta.len(),
                created: modified,
            })
        })
        .collect();
    backups.sort_by(|a, b| b.filename.cmp(&a.filename));
    Ok(backups)
}

/// Wipe user content (projects, memos, schedules, clients) in a single
/// transaction, after taking a `pre-reset-<ts>.db` snapshot in the backup
/// directory. The snapshot uses its own filename prefix so the normal
/// rolling-retention logic (which only touches `hearth-backup-*.db`) leaves
/// it alone — the user should always be able to get back to the pre-reset
/// state via the Restore list in the 백업 tab.
///
/// Categories, settings (AI creds, backup dir, UI scale), and the
/// `sqlite_sequence` table are **preserved** — the intent is "empty my
/// workspace" not "factory reset."
#[tauri::command]
pub fn reset_data(
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<String, String> {
    // 1) Flush WAL so the snapshot carries the latest committed state.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(|e| e.to_string())?;
    }

    // 2) Copy the current DB to <backup_dir>/pre-reset-<ts>.db.
    let source = db_path(&app)?;
    let dir = backup_dir(&app, &state)?;
    let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
    let snapshot = dir.join(format!("pre-reset-{}.db", timestamp));
    fs::copy(&source, &snapshot)
        .map_err(|e| format!("pre-reset snapshot failed: {}", e))?;

    // 3) Wipe user content in a single transaction. `sqlite_sequence` is
    // reset too so the next created row starts at id=1 again — otherwise
    // AUTOINCREMENT would keep climbing past the deleted max.
    {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        let tx = db.transaction().map_err(|e| e.to_string())?;
        for table in ["memos", "schedules", "projects", "clients"] {
            tx.execute(&format!("DELETE FROM {}", table), [])
                .map_err(|e| e.to_string())?;
        }
        tx.execute(
            "DELETE FROM sqlite_sequence WHERE name IN (?, ?, ?, ?)",
            ["memos", "schedules", "projects", "clients"],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }

    Ok(snapshot.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn touch(dir: &Path, ts: NaiveDateTime) -> PathBuf {
        let name = format!("hearth-backup-{}.db", ts.format("%Y-%m-%d-%H%M%S"));
        let p = dir.join(name);
        std::fs::File::create(&p).unwrap();
        p
    }

    fn listing(dir: &Path) -> BTreeSet<String> {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect()
    }

    fn ymdhms(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap()
    }

    #[test]
    fn prune_keeps_one_per_day_in_daily_window() {
        let tmp = tempfile::TempDir::new().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();

        // Three closes on 2026-04-19 — only the latest should survive.
        let a = touch(tmp.path(), ymdhms(2026, 4, 19, 8, 0, 0));
        let b = touch(tmp.path(), ymdhms(2026, 4, 19, 14, 30, 0));
        let c = touch(tmp.path(), ymdhms(2026, 4, 19, 23, 59, 59));
        // And one close on 2026-04-18 that should also survive (different day).
        let y = touch(tmp.path(), ymdhms(2026, 4, 18, 22, 0, 0));

        prune_backups_with_today(tmp.path(), today);

        let files = listing(tmp.path());
        assert!(!files.contains(a.file_name().unwrap().to_str().unwrap()));
        assert!(!files.contains(b.file_name().unwrap().to_str().unwrap()));
        assert!(files.contains(c.file_name().unwrap().to_str().unwrap()));
        assert!(files.contains(y.file_name().unwrap().to_str().unwrap()));
    }

    #[test]
    fn prune_bucket_transitions_daily_weekly_monthly() {
        let tmp = tempfile::TempDir::new().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();

        // Daily window (age < 7): keep one per day → 7 candidates.
        let daily: Vec<PathBuf> = (0..7)
            .map(|offset| {
                let d = today - chrono::Duration::days(offset);
                touch(
                    tmp.path(),
                    d.and_hms_opt(12, 0, 0).unwrap(),
                )
            })
            .collect();

        // Weekly window (age 7..35): drop a couple extra in the same iso-week
        // to verify the "one per week" coalesce.
        let wk_same_a = touch(tmp.path(), ymdhms(2026, 4, 11, 12, 0, 0)); // age 8
        let wk_same_b = touch(tmp.path(), ymdhms(2026, 4, 10, 12, 0, 0)); // age 9, same iso week
        let wk_other = touch(tmp.path(), ymdhms(2026, 3, 25, 12, 0, 0));  // age 25, different week

        // Monthly window (age 35..125): two in March 2026 — only newest stays.
        let mo_same_a = touch(tmp.path(), ymdhms(2026, 3, 10, 12, 0, 0));
        let mo_same_b = touch(tmp.path(), ymdhms(2026, 3, 5, 12, 0, 0));

        // Out of range (age > 125): should be deleted outright.
        let ancient = touch(tmp.path(), ymdhms(2025, 11, 1, 12, 0, 0));

        // Non-backup files must be left alone.
        let reset = tmp.path().join("pre-reset-2026-01-01-000000.db");
        std::fs::File::create(&reset).unwrap();
        let unrelated = tmp.path().join("readme.txt");
        std::fs::File::create(&unrelated).unwrap();

        prune_backups_with_today(tmp.path(), today);
        let files = listing(tmp.path());

        // Daily: all 7 kept.
        for p in &daily {
            assert!(
                files.contains(p.file_name().unwrap().to_str().unwrap()),
                "daily {:?} should survive",
                p
            );
        }
        // Weekly coalesce: the newer of the two same-week files survives,
        // the older is pruned, and the different-week one is kept.
        assert!(files.contains(wk_same_a.file_name().unwrap().to_str().unwrap()));
        assert!(!files.contains(wk_same_b.file_name().unwrap().to_str().unwrap()));
        assert!(files.contains(wk_other.file_name().unwrap().to_str().unwrap()));
        // Monthly coalesce: newer March file wins.
        assert!(files.contains(mo_same_a.file_name().unwrap().to_str().unwrap()));
        assert!(!files.contains(mo_same_b.file_name().unwrap().to_str().unwrap()));
        // Out of range dropped.
        assert!(!files.contains(ancient.file_name().unwrap().to_str().unwrap()));
        // Unrelated files untouched.
        assert!(files.contains("pre-reset-2026-01-01-000000.db"));
        assert!(files.contains("readme.txt"));
    }

    #[test]
    fn prune_tolerates_future_timestamps() {
        let tmp = tempfile::TempDir::new().unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 19).unwrap();

        // A backup dated tomorrow (e.g. user traveled or NTP blipped). Should
        // land in the daily bucket (age clamps to 0) and not be pruned.
        let future = touch(tmp.path(), ymdhms(2026, 4, 20, 10, 0, 0));

        prune_backups_with_today(tmp.path(), today);

        let files = listing(tmp.path());
        assert!(files.contains(future.file_name().unwrap().to_str().unwrap()));
    }
}

pub fn auto_backup_on_close(app: &AppHandle) {
    let state: State<'_, AppState> = app.state();
    if let Ok(db) = state.db.lock() {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").ok();
    }
    let source = app
        .path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("data.db"));
    let dest_dir = backup_dir(app, &state).ok();

    if let (Some(src), Some(dir)) = (source, dest_dir) {
        let timestamp = Local::now().format("%Y-%m-%d-%H%M%S");
        let dest = dir.join(format!("hearth-backup-{}.db", timestamp));
        fs::copy(&src, &dest).ok();
        prune_backups(&dir);
    }
}
