use crate::AppState;
use hearth_core::audit::Source;
use hearth_core::models::Schedule;
use hearth_core::schedules::{self, NewSchedule, UpdateSchedule};
use serde::Deserialize;
use tauri::{AppHandle, State};

#[derive(Debug, Deserialize)]
pub struct ScheduleInput {
    pub date: String,
    pub time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub remind_before_5min: bool,
    #[serde(default)]
    pub remind_at_start: bool,
}

#[tauri::command]
pub fn get_schedules(
    state: State<'_, AppState>,
    month: Option<String>,
) -> Result<Vec<Schedule>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    schedules::list(&db, month.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    let sched = {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        schedules::create(
            &mut db,
            Source::App,
            &NewSchedule {
                date: &data.date,
                time: data.time.as_deref(),
                location: data.location.as_deref(),
                description: data.description.as_deref(),
                notes: data.notes.as_deref(),
                remind_before_5min: data.remind_before_5min,
                remind_at_start: data.remind_at_start,
            },
        )
        .map_err(|e| e.to_string())?
    };
    crate::cmd_notify::apply_for(&app, &sched).ok();
    Ok(sched)
}

#[tauri::command]
pub fn update_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    data: ScheduleInput,
) -> Result<Schedule, String> {
    let sched = {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        schedules::update(
            &mut db,
            Source::App,
            id,
            &UpdateSchedule {
                date: Some(&data.date),
                time: data.time.as_deref(),
                location: data.location.as_deref(),
                description: data.description.as_deref(),
                notes: data.notes.as_deref(),
                remind_before_5min: Some(data.remind_before_5min),
                remind_at_start: Some(data.remind_at_start),
            },
        )
        .map_err(|e| e.to_string())?
    };
    crate::cmd_notify::apply_for(&app, &sched).ok();
    Ok(sched)
}

#[tauri::command]
pub fn delete_schedule(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    {
        let mut db = state.db.lock().map_err(|e| e.to_string())?;
        schedules::delete(&mut db, Source::App, id).map_err(|e| e.to_string())?;
    }
    crate::cmd_notify::cancel_for_id(&app, id);
    Ok(())
}
