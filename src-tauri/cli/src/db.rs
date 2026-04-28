use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

pub fn default_db_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

#[cfg(target_os = "macos")]
fn dirs_macos() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let app_support = home.join("Library").join("Application Support");
    let new_dir = app_support.join("com.codewithgenie.hearth");
    let legacy_dir = app_support.join("com.newturn2017.hearth");
    // One-shot migration from the pre-rebrand bundle id. Only fires when the
    // legacy directory exists and the new one does not — never overwrites.
    if !new_dir.exists() && legacy_dir.exists() {
        let _ = std::fs::rename(&legacy_dir, &new_dir);
    }
    Some(new_dir.join("data.db"))
}

pub fn resolve_db_path(flag: Option<&str>) -> Result<PathBuf> {
    if let Some(p) = flag {
        return Ok(PathBuf::from(p));
    }
    if let Ok(e) = std::env::var("HEARTH_DB") {
        return Ok(PathBuf::from(e));
    }
    default_db_path().context(
        "could not resolve default DB path. Pass --db <PATH> or set HEARTH_DB=<PATH>.",
    )
}

pub fn open(path: &std::path::Path) -> Result<Connection> {
    let conn = hearth_core::db::init_db(path)
        .with_context(|| format!("failed to open hearth DB at {}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_millis(3000))
        .ok();
    Ok(conn)
}
