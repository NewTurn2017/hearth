pub mod ai_tools;
mod cmd_actions;
mod cmd_ai;
mod cmd_backup;
mod cmd_clients;
mod cmd_memos;
mod cmd_projects;
mod cmd_schedules;
mod cmd_settings;
mod db;
mod excel_import;
mod models;

use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            std::fs::create_dir_all(&app_data_dir)?;

            let db_path = app_data_dir.join("data.db");
            let conn = db::init_db(&db_path).expect("failed to init database");

            app.manage(AppState {
                db: Mutex::new(conn),
            });

            app.manage(cmd_ai::AiManager::new(
                "/Users/genie/dev/side/supergemma-bench/start-mlx.sh".to_string(),
            ));

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                cmd_backup::auto_backup_on_close(window.app_handle());
                if let Some(mgr) = window.app_handle().try_state::<cmd_ai::AiManager>() {
                    cmd_ai::kill_child(&mgr);
                    cmd_ai::kill_mlx_if_ours(&mgr);
                }
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
            cmd_clients::get_clients,
            cmd_actions::open_in_ghostty,
            cmd_actions::open_in_finder,
            cmd_actions::import_excel,
            cmd_backup::backup_db,
            cmd_backup::restore_db,
            cmd_backup::list_backups,
            cmd_ai::start_ai_server,
            cmd_ai::stop_ai_server,
            cmd_ai::ai_server_status,
            cmd_ai::ai_chat,
            cmd_ai::ai_confirm,
            cmd_settings::get_ai_settings,
            cmd_settings::save_ai_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
