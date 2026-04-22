// Settings store for AI configuration.
//
// AI is OpenAI-only as of 0.3.0. The local MLX backend was removed because
// it hard-coded a path that only worked on the original developer's machine.
//
// The OpenAI key is stored plaintext in the same SQLite DB as the rest of the
// user's data. We do not use Keychain: this is a single-user desktop tool and
// the DB already lives in the user's private app data dir. If that threat
// model tightens later, swap to `tauri-plugin-keyring`.
//
// The key is *never* returned to the frontend — `get_ai_settings` returns a
// `has_openai_key` boolean instead. Saving with `openai_api_key: Some("")`
// clears it; `None` leaves it untouched.
//
// Model selection is intentionally NOT exposed here: OpenAI uses the
// `OPENAI_MODEL` constant in `cmd_ai`. Keeping a single source of truth
// avoids the "wrong ID → 404 / HF 401" failure mode a UI picker used to create.

use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Keys used in the `settings` KV table. Centralized so any rename happens in
/// one place and we don't typo the string at a read site.
const K_OPENAI_KEY: &str = "ai.openai_api_key";
const K_UI_SCALE: &str = "ui.scale";
pub(crate) const K_BACKUP_DIR: &str = "backup.dir";
pub(crate) const K_AUTOSTART: &str = "autostart.enabled";

/// Shape safe to expose over IPC — the raw API key never crosses this
/// boundary. The UI only needs to know whether one is on file.
#[derive(Debug, Clone, Serialize)]
pub struct AiSettingsView {
    pub has_openai_key: bool,
}

/// Internal-only view that includes the decrypted key, consumed by `cmd_ai`
/// when it needs to authorize an OpenAI request.
#[derive(Debug, Clone)]
pub struct AiSettingsFull {
    pub openai_api_key: Option<String>,
}

impl AiSettingsFull {
    /// Convert to the IPC-safe view, stripping the secret.
    pub fn redact(&self) -> AiSettingsView {
        AiSettingsView {
            has_openai_key: self
                .openai_api_key
                .as_deref()
                .map(|s| !s.is_empty())
                .unwrap_or(false),
        }
    }
}

/// Read a single KV entry, returning an owned string (possibly empty).
pub(crate) fn read(db: &rusqlite::Connection, key: &str) -> Result<String, String> {
    db.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get::<_, String>(0),
    )
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(String::new()),
        other => Err(other.to_string()),
    })
}

/// Upsert a KV entry.
pub(crate) fn write(db: &rusqlite::Connection, key: &str, value: &str) -> Result<(), String> {
    db.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Read the full settings (including the API key) for backend use. Consumers
/// outside this module should only use this to make privileged API calls —
/// never hand the key back to the frontend.
pub fn load_full(state: &State<'_, AppState>) -> Result<AiSettingsFull, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let key_raw = read(&db, K_OPENAI_KEY)?;
    let openai_api_key = if key_raw.is_empty() {
        None
    } else {
        Some(key_raw)
    };
    Ok(AiSettingsFull { openai_api_key })
}

#[tauri::command]
pub fn get_ai_settings(state: State<'_, AppState>) -> Result<AiSettingsView, String> {
    Ok(load_full(&state)?.redact())
}

#[derive(Debug, Deserialize)]
pub struct SaveAiSettingsInput {
    /// - `Some("")` → clear the stored key
    /// - `Some("sk-...")` → overwrite
    /// - `None` → keep whatever is already stored
    #[serde(default)]
    pub openai_api_key: Option<String>,
}

#[tauri::command]
pub fn save_ai_settings(
    state: State<'_, AppState>,
    input: SaveAiSettingsInput,
) -> Result<AiSettingsView, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(key) = input.openai_api_key {
        // Trim incidental whitespace — copy-paste commonly carries a newline.
        write(&db, K_OPENAI_KEY, key.trim())?;
    }

    // Re-read to build the canonical view.
    drop(db);
    Ok(load_full(&state)?.redact())
}

#[tauri::command]
pub fn get_ui_scale(state: State<'_, AppState>) -> Result<f64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let raw = read(&db, K_UI_SCALE)?;
    if raw.is_empty() {
        return Ok(1.0);
    }
    raw.parse::<f64>()
        .map_err(|e| format!("invalid ui.scale value: {e}"))
}

#[tauri::command]
pub fn set_ui_scale(state: State<'_, AppState>, scale: f64) -> Result<(), String> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!("invalid scale: {scale}"));
    }
    let db = state.db.lock().map_err(|e| e.to_string())?;
    write(&db, K_UI_SCALE, &scale.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_hides_the_key_but_reports_presence() {
        let with_key = AiSettingsFull {
            openai_api_key: Some("sk-abc".into()),
        };
        let v = with_key.redact();
        assert!(v.has_openai_key);
    }

    #[test]
    fn redact_reports_absence_when_key_is_none_or_empty() {
        let none = AiSettingsFull {
            openai_api_key: None,
        };
        assert!(!none.redact().has_openai_key);

        let empty = AiSettingsFull {
            openai_api_key: Some(String::new()),
        };
        assert!(!empty.redact().has_openai_key);
    }
}
