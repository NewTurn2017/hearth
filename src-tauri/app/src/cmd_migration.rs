// Security-scoped bookmark plumbing for the canonical data folder
// (~/Library/Application Support/com.codewithgenie.hearth/).
//
// PR-A: exposes three Tauri commands that the migration wizard UI (PR-B)
// will drive. The DB bootstrap path in lib.rs::setup() is intentionally
// unchanged in this PR.
//
// Spec: docs/superpowers/specs/2026-04-26-mas-readiness-design.md §4-3.

use serde::Serialize;
use std::path::PathBuf;
use tauri::AppHandle;

const BOOKMARK_KEY: &str = "hearth.dataDirBookmark";
const DISMISSED_KEY: &str = "hearth.migrationDismissed";

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DataFolderStatus {
    pub has_bookmark: bool,
    pub resolved_path: Option<String>,
    pub stale: bool,
    pub dismissed: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChooseFolderResponse {
    pub resolved_path: String,
}

#[tauri::command]
pub async fn get_data_folder_status() -> Result<DataFolderStatus, String> {
    #[cfg(target_os = "macos")]
    {
        macos::get_status()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(DataFolderStatus {
            has_bookmark: false,
            resolved_path: None,
            stale: false,
            dismissed: false,
        })
    }
}

#[tauri::command]
pub async fn choose_data_folder(app: AppHandle) -> Result<ChooseFolderResponse, String> {
    #[cfg(target_os = "macos")]
    {
        macos::choose_folder(app).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("choose_data_folder is only supported on macOS".to_string())
    }
}

#[tauri::command]
pub async fn dismiss_migration() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::set_dismissed(true);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    app.restart();
}

/// Outcome of the boot-time bookmark decision.
///
/// `Bookmarked` — the persisted bookmark resolved successfully and
/// `startAccessingSecurityScopedResource` succeeded; `access` must live for
/// the duration of the app process so the resource stays accessible.
///
/// `Fallback` — no usable bookmark; the app boots from the sandbox container
/// path. `needs_wizard` is true when the user has neither chosen a folder
/// nor explicitly dismissed the wizard, so the frontend should surface the
/// migration prompt.
pub enum BootDecision {
    Bookmarked {
        db_dir: PathBuf,
        access: BookmarkAccess,
    },
    Fallback {
        db_dir: PathBuf,
        needs_wizard: bool,
    },
}

/// RAII handle that releases the security-scoped resource when dropped.
/// Stored in app state so it survives for the lifetime of the process.
pub struct BookmarkAccess {
    #[cfg(target_os = "macos")]
    url: objc2::rc::Retained<objc2_foundation::NSURL>,
}

#[cfg(target_os = "macos")]
impl Drop for BookmarkAccess {
    fn drop(&mut self) {
        unsafe {
            self.url.stopAccessingSecurityScopedResource();
        }
    }
}

/// Decide where the DB lives at boot. `fallback_dir` is the sandbox container
/// path — used when no bookmark is set. On non-macOS this always returns
/// `Fallback` with `needs_wizard: false` since the wizard is mac-only.
pub fn decide_boot(fallback_dir: PathBuf) -> BootDecision {
    #[cfg(target_os = "macos")]
    {
        macos::decide_boot(fallback_dir)
    }
    #[cfg(not(target_os = "macos"))]
    {
        BootDecision::Fallback {
            db_dir: fallback_dir,
            needs_wizard: false,
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{
        MainThreadMarker, NSData, NSString, NSURL, NSURLBookmarkCreationOptions,
        NSURLBookmarkResolutionOptions, NSUserDefaults,
    };

    pub fn decide_boot(fallback_dir: PathBuf) -> BootDecision {
        let Some(blob) = read_bookmark_blob() else {
            return BootDecision::Fallback {
                db_dir: fallback_dir,
                needs_wizard: !read_dismissed(),
            };
        };

        let resolved = match resolve_bookmark(&blob) {
            Ok(r) => r,
            Err(e) => {
                // Folder gone / blob corrupt — clear and re-prompt next launch.
                eprintln!("bookmark resolve failed: {e}");
                clear_bookmark_blob();
                return BootDecision::Fallback {
                    db_dir: fallback_dir,
                    needs_wizard: !read_dismissed(),
                };
            }
        };

        if resolved.stale {
            // Folder moved/renamed. Best-effort refresh — pass the resolved
            // URL itself, since a freshly-constructed NSURL has no
            // security-scope grant and bookmarkDataWithOptions(.WithSecurityScope)
            // would fail. Spec §4-3: "bookmark 재생성 (resolved url 그대로)".
            if let Err(e) = refresh_stale_bookmark(&resolved.url) {
                eprintln!("stale bookmark refresh failed: {e}");
            }
        }

        let started = unsafe { resolved.url.startAccessingSecurityScopedResource() };
        if !started {
            eprintln!(
                "startAccessingSecurityScopedResource returned false for {}",
                resolved.path
            );
            // Sandbox refused — clear the poison-pill blob so we re-prompt
            // on next launch instead of looping in degraded mode forever.
            clear_bookmark_blob();
            return BootDecision::Fallback {
                db_dir: fallback_dir,
                needs_wizard: !read_dismissed(),
            };
        }

        BootDecision::Bookmarked {
            db_dir: PathBuf::from(&resolved.path),
            access: BookmarkAccess { url: resolved.url },
        }
    }

    pub fn get_status() -> Result<DataFolderStatus, String> {
        let dismissed = read_dismissed();
        let Some(blob) = read_bookmark_blob() else {
            return Ok(DataFolderStatus {
                has_bookmark: false,
                resolved_path: None,
                stale: false,
                dismissed,
            });
        };

        match resolve_bookmark(&blob) {
            Ok(Resolved { path, stale, url }) => {
                if stale {
                    // Best-effort refresh; failure is non-fatal — UI still
                    // gets the resolved path so the user can keep working.
                    let _ = refresh_stale_bookmark(&url);
                }
                Ok(DataFolderStatus {
                    has_bookmark: true,
                    resolved_path: Some(path),
                    stale,
                    dismissed,
                })
            }
            Err(_) => {
                // Bookmark resolution itself failed — treat as no bookmark
                // so the wizard re-prompts. Spec §4-3 "Bookmark resolution
                // 실패" path.
                clear_bookmark_blob();
                Ok(DataFolderStatus {
                    has_bookmark: false,
                    resolved_path: None,
                    stale: false,
                    dismissed,
                })
            }
        }
    }

    pub async fn choose_folder(app: AppHandle) -> Result<ChooseFolderResponse, String> {
        // Open NSOpenPanel pre-pointed at the canonical (non-container)
        // location used by 0.x — that's where existing users' data lives,
        // and it's outside the sandbox container. `app_data_dir()` under
        // MAS would return the container path, which is the wrong place
        // for the primary migration audience.
        let initial_dir = std::env::var("HOME")
            .ok()
            .map(|h| format!("{h}/Library/Application Support/com.codewithgenie.hearth"));

        let (tx, rx) = tokio::sync::oneshot::channel::<Result<ChooseFolderResponse, String>>();
        app.run_on_main_thread(move || {
            let res = run_panel_and_persist(initial_dir.as_deref());
            let _ = tx.send(res);
        })
        .map_err(|e| format!("run_on_main_thread failed: {e}"))?;

        rx.await.map_err(|e| format!("oneshot recv failed: {e}"))?
    }

    fn run_panel_and_persist(initial_dir: Option<&str>) -> Result<ChooseFolderResponse, String> {
        // SAFETY: tauri::AppHandle::run_on_main_thread guarantees this
        // closure executes on the AppKit main thread.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseDirectories(true);
        // Folder-only: lib.rs::setup() unconditionally appends `data.db` to
        // the chosen path, so a file selection would produce
        // `<.../data.db>/data.db` and fail SQLite open. The wizard copy also
        // says "데이터 폴더 선택", so this matches user expectations.
        panel.setCanChooseFiles(false);
        panel.setAllowsMultipleSelection(false);
        let prompt = NSString::from_str("Hearth 데이터 폴더 선택");
        panel.setPrompt(Some(&prompt));
        let title = NSString::from_str("Hearth 데이터 폴더 연결");
        panel.setTitle(Some(&title));
        let message = NSString::from_str(
            "Hearth가 데이터를 보관할 폴더를 선택해 주세요. CLI 및 AI agent와 같은 데이터를 공유합니다.",
        );
        panel.setMessage(Some(&message));

        if let Some(dir) = initial_dir {
            // Best-effort: pre-create the folder so NSOpenPanel can navigate
            // to it on first launch even before the DB has been written.
            let _ = std::fs::create_dir_all(dir);
            let dir_ns = NSString::from_str(dir);
            let dir_url = NSURL::fileURLWithPath(&dir_ns);
            panel.setDirectoryURL(Some(&dir_url));
        }

        let response = panel.runModal();
        if response != NSModalResponseOK {
            return Err("user_cancelled".to_string());
        }

        let urls = panel.URLs();
        let url = urls
            .firstObject()
            .ok_or_else(|| "NSOpenPanel returned no URL".to_string())?;

        let blob = create_bookmark(&url)?;
        write_bookmark_blob(&blob);
        // Selecting a folder counts as engaging with the wizard — clear the
        // "later" marker so the UI stops showing the dismissable banner.
        set_dismissed(false);

        let path = url_to_path_string(&url)?;
        Ok(ChooseFolderResponse {
            resolved_path: path,
        })
    }

    struct Resolved {
        path: String,
        stale: bool,
        url: objc2::rc::Retained<NSURL>,
    }

    fn resolve_bookmark(blob: &[u8]) -> Result<Resolved, String> {
        let data = NSData::with_bytes(blob);
        let mut is_stale = objc2::runtime::Bool::NO;
        let url = unsafe {
            NSURL::URLByResolvingBookmarkData_options_relativeToURL_bookmarkDataIsStale_error(
                &data,
                NSURLBookmarkResolutionOptions::WithSecurityScope,
                None,
                &mut is_stale,
            )
        }
        .map_err(|e| format!("URLByResolvingBookmarkData failed: {}", ns_error_msg(&e)))?;

        let path = url_to_path_string(&url)?;
        Ok(Resolved {
            path,
            stale: is_stale.as_bool(),
            url,
        })
    }

    fn refresh_stale_bookmark(url: &NSURL) -> Result<(), String> {
        let blob = create_bookmark(url)?;
        write_bookmark_blob(&blob);
        Ok(())
    }

    fn create_bookmark(url: &NSURL) -> Result<Vec<u8>, String> {
        let data = url
            .bookmarkDataWithOptions_includingResourceValuesForKeys_relativeToURL_error(
                NSURLBookmarkCreationOptions::WithSecurityScope,
                None,
                None,
            )
            .map_err(|e| format!("bookmarkDataWithOptions failed: {}", ns_error_msg(&e)))?;
        Ok(data.to_vec())
    }

    fn url_to_path_string(url: &NSURL) -> Result<String, String> {
        let ns_path = url
            .path()
            .ok_or_else(|| "NSURL has no filesystem path".to_string())?;
        let path: PathBuf = ns_path.to_string().into();
        Ok(path.to_string_lossy().into_owned())
    }

    fn ns_error_msg(err: &objc2_foundation::NSError) -> String {
        err.localizedDescription().to_string()
    }

    // ---- UserDefaults helpers ------------------------------------------------

    fn defaults() -> objc2::rc::Retained<NSUserDefaults> {
        NSUserDefaults::standardUserDefaults()
    }

    fn read_bookmark_blob() -> Option<Vec<u8>> {
        let key = NSString::from_str(BOOKMARK_KEY);
        defaults().dataForKey(&key).map(|d| d.to_vec())
    }

    fn write_bookmark_blob(blob: &[u8]) {
        let key = NSString::from_str(BOOKMARK_KEY);
        let data = NSData::with_bytes(blob);
        let any: &objc2::runtime::AnyObject = (*data).as_ref();
        unsafe {
            defaults().setObject_forKey(Some(any), &key);
        }
    }

    fn clear_bookmark_blob() {
        let key = NSString::from_str(BOOKMARK_KEY);
        defaults().removeObjectForKey(&key);
    }

    fn read_dismissed() -> bool {
        let key = NSString::from_str(DISMISSED_KEY);
        defaults().boolForKey(&key)
    }

    pub fn set_dismissed(value: bool) {
        let key = NSString::from_str(DISMISSED_KEY);
        defaults().setBool_forKey(value, &key);
    }
}
