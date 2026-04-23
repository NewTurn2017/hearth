pub mod ai_tools;
mod cmd_actions;
mod cmd_ai;
mod cmd_autostart;
mod cmd_backup;
mod cmd_categories;
mod cmd_clients;
mod cmd_memos;
mod cmd_notify;
mod cmd_projects;
mod cmd_quick_capture;
mod cmd_schedules;
mod cmd_settings;
mod db;
mod excel_import;
mod models;
mod watcher;

use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("data.db");
            // If the DB file is corrupt (`database disk image is malformed`),
            // quarantine it and boot from an empty schema instead of
            // panicking. The user is notified via the `db:recovered` event so
            // they can restore from a backup in Settings → 백업.
            let (conn, recovered_from) = match db::init_db_with_recovery(&db_path) {
                Ok(db::DbInitOutcome::Ok(c)) => (c, None),
                Ok(db::DbInitOutcome::Recovered {
                    conn,
                    quarantined_to,
                }) => (conn, Some(quarantined_to)),
                Err(e) => return Err(Box::new(e).into()),
            };

            app.manage(AppState {
                db: Mutex::new(conn),
            });
            app.manage(crate::cmd_notify::Scheduler::new());

            let launched_hidden = std::env::args().any(|a| a == "--hidden");
            if !launched_hidden {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }

            // If we recovered from corruption, tell the webview after it
            // finishes loading so a toast / modal can be shown. The frontend
            // listens for `db:recovered` with the quarantined path as payload.
            if let Some(path) = recovered_from {
                let app_handle = app.handle().clone();
                let payload = path.to_string_lossy().into_owned();
                tauri::async_runtime::spawn(async move {
                    // Small delay so listener is mounted.
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    let _ = app_handle.emit("db:recovered", payload);
                });
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::cmd_notify::reschedule_all_future(&app_handle) {
                    eprintln!("notification boot reschedule failed: {e}");
                }
            });

            // Pre-build the quick-capture overlay window (hidden).
            let _ = crate::cmd_quick_capture::ensure_window(app.handle());

            // Read saved combo (falls back to DEFAULT_COMBO) and register the
            // global shortcut. Failure must NOT crash the app.
            let combo = {
                let state = app.state::<AppState>();
                let db = state.db.lock().map_err(|e| e.to_string());
                match db {
                    Ok(db) => crate::cmd_quick_capture::read_combo(&db)
                        .unwrap_or_else(|_| crate::cmd_quick_capture::DEFAULT_COMBO.to_string()),
                    Err(_) => crate::cmd_quick_capture::DEFAULT_COMBO.to_string(),
                }
            };

            let shortcut_result = app
                .global_shortcut()
                .on_shortcut(combo.as_str(), |app_handle, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = crate::cmd_quick_capture::toggle_quick_capture_window(
                            app_handle.clone(),
                        );
                    }
                });

            // Write success/failure to KV — non-fatal on error.
            let error_msg = match shortcut_result {
                Ok(()) => String::new(),
                Err(e) => {
                    eprintln!("quick-capture shortcut registration failed: {e}");
                    e.to_string()
                }
            };
            {
                let app_state = app.state::<AppState>();
                let db_guard = app_state.db.lock();
                if let Ok(db) = db_guard {
                    let _ = crate::cmd_settings::write(
                        &db,
                        crate::cmd_quick_capture::K_SHORTCUT_LAST_ERROR,
                        &error_msg,
                    );
                }
            }

            crate::watcher::spawn(app.handle().clone());

            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    #[cfg(target_os = "macos")]
                    {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    cmd_backup::auto_backup_on_close(window.app_handle());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            cmd_projects::get_projects,
            cmd_projects::update_project,
            cmd_projects::create_project,
            cmd_projects::delete_project,
            cmd_projects::reorder_projects,
            cmd_projects::search_projects,
            cmd_schedules::get_schedules,
            cmd_schedules::create_schedule,
            cmd_schedules::update_schedule,
            cmd_schedules::delete_schedule,
            cmd_memos::get_memos,
            cmd_memos::create_memo,
            cmd_memos::update_memo,
            cmd_memos::delete_memo,
            cmd_memos::reorder_memos,
            cmd_memos::update_memo_by_number,
            cmd_memos::delete_memo_by_number,
            cmd_clients::get_clients,
            cmd_actions::open_in_terminal,
            cmd_actions::open_in_finder,
            cmd_actions::import_excel,
            cmd_backup::backup_db,
            cmd_backup::restore_db,
            cmd_backup::list_backups,
            cmd_backup::get_backup_dir,
            cmd_backup::set_backup_dir,
            cmd_backup::reset_data,
            cmd_categories::get_categories,
            cmd_categories::create_category,
            cmd_categories::update_category,
            cmd_categories::delete_category,
            cmd_categories::reorder_categories,
            cmd_ai::ai_chat,
            cmd_ai::ai_confirm,
            cmd_settings::get_ai_settings,
            cmd_settings::save_ai_settings,
            cmd_settings::get_ui_scale,
            cmd_settings::set_ui_scale,
            cmd_settings::get_theme,
            cmd_settings::set_theme,
            cmd_notify::notifications_permission,
            cmd_notify::notifications_request,
            cmd_autostart::get_autostart,
            cmd_autostart::set_autostart,
            cmd_quick_capture::get_quick_capture_shortcut,
            cmd_quick_capture::get_quick_capture_shortcut_error,
            cmd_quick_capture::rebind_quick_capture_shortcut,
            cmd_quick_capture::show_quick_capture_window,
            cmd_quick_capture::hide_quick_capture_window,
            cmd_quick_capture::toggle_quick_capture_window,
            cmd_quick_capture::resize_quick_capture_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Reopen { has_visible_windows, .. } = event {
                if !has_visible_windows {
                    if let Some(win) = app_handle.get_webview_window("main") {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        });
}
