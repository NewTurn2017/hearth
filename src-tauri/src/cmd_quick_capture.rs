//! Quick Capture — global shortcut and hidden overlay window.
//!
//! The shortcut combo is persisted in the `settings` KV under
//! `shortcut.quick_capture`. The UI sends combos in display form (e.g.
//! "Cmd+Shift+H"); we normalize to Tauri's accelerator format
//! ("CommandOrControl+Shift+H") before register.

pub(crate) const K_SHORTCUT: &str = "shortcut.quick_capture";
pub(crate) const K_SHORTCUT_LAST_ERROR: &str = "shortcut.quick_capture.last_error";
pub(crate) const DEFAULT_COMBO: &str = "CommandOrControl+Shift+H";
pub(crate) const WINDOW_LABEL: &str = "quick-capture";

/// Normalize a user-facing combo string to Tauri's accelerator format.
/// Accepts "Cmd+Shift+H", "Ctrl+Shift+H", "CommandOrControl+Shift+H".
/// Returns Err if no base (non-modifier) key is present or if an unknown
/// token appears.
pub fn normalize_accelerator(input: &str) -> Result<String, String> {
    let mut mods: Vec<String> = Vec::new();
    let mut base: Option<String> = None;
    for raw in input.split('+') {
        let tok = raw.trim();
        if tok.is_empty() {
            return Err(format!("Invalid accelerator: empty token in {input:?}"));
        }
        match tok.to_ascii_lowercase().as_str() {
            "cmd" | "command" | "meta" | "super" | "commandorcontrol"
            | "ctrl" | "control" => push_mod(&mut mods, "CommandOrControl"),
            "alt" | "option" | "opt" => push_mod(&mut mods, "Alt"),
            "shift" => push_mod(&mut mods, "Shift"),
            _ => {
                if base.is_some() {
                    return Err(format!("Invalid accelerator: multiple base keys in {input:?}"));
                }
                base = Some(canonical_base(tok)?);
            }
        }
    }
    let base = base.ok_or_else(|| format!("Invalid accelerator: no base key in {input:?}"))?;
    let mut parts = mods;
    parts.push(base);
    Ok(parts.join("+"))
}

fn push_mod(mods: &mut Vec<String>, name: &str) {
    let owned = name.to_string();
    if !mods.iter().any(|m| m == &owned) {
        mods.push(owned);
    }
}

fn canonical_base(tok: &str) -> Result<String, String> {
    let t = tok.trim();
    if t.len() == 1 {
        let c = t.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Ok(c.to_ascii_uppercase().to_string());
        }
        if c.is_ascii_digit() {
            return Ok(t.to_string());
        }
    }
    let upper = t.to_ascii_uppercase();
    if upper.starts_with('F') && upper[1..].chars().all(|c| c.is_ascii_digit()) {
        return Ok(upper);
    }
    match upper.as_str() {
        "SPACE" | "ENTER" | "ESCAPE" | "TAB" | "BACKSPACE" | "DELETE"
        | "ARROWUP" | "ARROWDOWN" | "ARROWLEFT" | "ARROWRIGHT" => Ok(upper),
        _ => Err(format!("Invalid accelerator: unsupported key {t:?}")),
    }
}

use crate::cmd_settings;
use crate::AppState;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

/// Read the saved combo, falling back to the default. Returns a value
/// already normalized to Tauri accelerator form.
pub(crate) fn read_combo(db: &rusqlite::Connection) -> Result<String, String> {
    let raw = cmd_settings::read(db, K_SHORTCUT)?;
    let combo = if raw.trim().is_empty() {
        DEFAULT_COMBO.to_string()
    } else {
        raw
    };
    normalize_accelerator(&combo)
}

#[tauri::command]
pub fn get_quick_capture_shortcut(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    read_combo(&db)
}

#[tauri::command]
pub fn get_quick_capture_shortcut_error(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::read(&db, K_SHORTCUT_LAST_ERROR)
}

#[tauri::command]
pub fn rebind_quick_capture_shortcut(
    app: AppHandle,
    state: State<'_, AppState>,
    combo: String,
) -> Result<String, String> {
    let normalized = normalize_accelerator(&combo)?;
    let old = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        read_combo(&db)?
    };

    let gs = app.global_shortcut();
    // unregister accepts &str via TryInto<ShortcutWrapper>
    let _ = gs.unregister(old.as_str());
    if let Err(e) = gs.register(normalized.as_str()) {
        // Roll back: re-register the old shortcut (best-effort)
        let _ = gs.register(old.as_str());
        return Err(format!("register shortcut failed: {e}"));
    }

    let db = state.db.lock().map_err(|e| e.to_string())?;
    cmd_settings::write(&db, K_SHORTCUT, &normalized)?;
    cmd_settings::write(&db, K_SHORTCUT_LAST_ERROR, "")?;
    drop(db);

    let _ = app.emit("quick-capture-shortcut:changed", &normalized);
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_cmd_shift_letter() {
        assert_eq!(
            normalize_accelerator("Cmd+Shift+H").unwrap(),
            "CommandOrControl+Shift+H"
        );
        assert_eq!(
            normalize_accelerator("ctrl+shift+h").unwrap(),
            "CommandOrControl+Shift+H"
        );
        assert_eq!(
            normalize_accelerator("CommandOrControl+Shift+H").unwrap(),
            "CommandOrControl+Shift+H"
        );
    }

    #[test]
    fn normalize_duplicates_collapse() {
        assert_eq!(
            normalize_accelerator("Cmd+Ctrl+H").unwrap(),
            "CommandOrControl+H"
        );
    }

    #[test]
    fn normalize_function_key() {
        assert_eq!(normalize_accelerator("Alt+F5").unwrap(), "Alt+F5");
    }

    #[test]
    fn normalize_rejects_no_base() {
        assert!(normalize_accelerator("Cmd+Shift").is_err());
    }

    #[test]
    fn normalize_rejects_unknown_key() {
        assert!(normalize_accelerator("Cmd+Weird").is_err());
    }

    #[test]
    fn normalize_rejects_empty_token() {
        assert!(normalize_accelerator("Cmd++H").is_err());
    }
}
