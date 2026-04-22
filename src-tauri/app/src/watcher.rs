//! Poll SQLite `PRAGMA data_version` every 500ms to detect writes made by
//! other connections (e.g. `hearth` CLI) and invalidate UI caches.

use crate::AppState;
use tauri::{AppHandle, Emitter, Manager};

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<i64> = None;
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let state = match app.try_state::<AppState>() {
                Some(s) => s,
                None => continue,
            };
            let v: Option<i64> = {
                let guard = state.db.lock().ok();
                guard.and_then(|db| db.query_row("PRAGMA data_version", [], |r| r.get(0)).ok())
            };
            let Some(v) = v else { continue };
            match last {
                None => {
                    last = Some(v);
                }
                Some(l) if l != v => {
                    let _ = app.emit("projects:changed", ());
                    let _ = app.emit("memos:changed", ());
                    let _ = app.emit("schedules:changed", ());
                    let _ = app.emit("categories:changed", ());
                    last = Some(v);
                }
                _ => {}
            }
        }
    });
}
