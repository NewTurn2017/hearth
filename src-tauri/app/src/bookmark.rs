// Per-project security-scoped bookmark plumbing.
//
// Project folders (`projects.path`) live outside the MAS sandbox container,
// so opening them in Finder / Terminal under sandbox requires a
// security-scoped bookmark. We store one bookmark blob per project keyed by
// project id in NSUserDefaults — same mechanism as the data-folder bookmark
// in cmd_migration, but separately keyed.
//
// FB-001 (1.0.0 hotfix): adds the storage + a pick command + an
// access-scoped open path. Frontend captures the bookmark at folder-pick
// time and ships it through create/update; the legacy "Finder에서 열기"
// path falls back to a one-time JIT pick prompt for projects that
// pre-date the bookmark column.

use serde::Serialize;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PickFolderResponse {
    pub path: String,
    /// Security-scoped bookmark blob, JSON-serialized as a number array.
    /// Frontend treats it as opaque bytes — only ships it back to backend.
    pub bookmark: Vec<u8>,
}

/// RAII handle that releases the security-scoped resource on drop.
/// Held only for the duration of a single open/openURL call.
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

#[cfg(target_os = "macos")]
impl BookmarkAccess {
    pub fn url(&self) -> &objc2_foundation::NSURL {
        &self.url
    }
}

#[cfg(target_os = "macos")]
pub fn project_bookmark_key(id: i64) -> String {
    format!("hearth.projectBookmark.{id}")
}

#[cfg(target_os = "macos")]
pub fn read_project_bookmark(id: i64) -> Option<Vec<u8>> {
    macos::read_blob(&project_bookmark_key(id))
}

#[cfg(target_os = "macos")]
pub fn write_project_bookmark(id: i64, blob: &[u8]) {
    macos::write_blob(&project_bookmark_key(id), blob);
}

#[cfg(target_os = "macos")]
pub fn clear_project_bookmark(id: i64) {
    macos::clear_blob(&project_bookmark_key(id));
}

#[cfg(not(target_os = "macos"))]
pub fn read_project_bookmark(_id: i64) -> Option<Vec<u8>> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn write_project_bookmark(_id: i64, _blob: &[u8]) {}

#[cfg(not(target_os = "macos"))]
pub fn clear_project_bookmark(_id: i64) {}

/// Open NSOpenPanel and create a security-scoped bookmark for the chosen
/// directory. Must be invoked from the AppKit main thread.
pub async fn pick_directory(
    app: tauri::AppHandle,
    suggested: Option<String>,
) -> Result<PickFolderResponse, String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<PickFolderResponse, String>>();
        app.run_on_main_thread(move || {
            let res = macos::pick_directory_with_bookmark(suggested.as_deref());
            let _ = tx.send(res);
        })
        .map_err(|e| format!("run_on_main_thread failed: {e}"))?;

        rx.await.map_err(|e| format!("oneshot recv failed: {e}"))?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, suggested);
        Err("pick_directory is only supported on macOS".to_string())
    }
}

/// Resolve a stored bookmark blob and start a security-scoped access. The
/// returned `BookmarkAccess` must outlive any FS / NSWorkspace call against
/// the resolved URL — drop it as soon as the call completes.
#[cfg(target_os = "macos")]
pub fn start_access(blob: &[u8]) -> Result<BookmarkAccess, String> {
    let resolved = macos::resolve_bookmark(blob)?;
    let started = unsafe { resolved.url.startAccessingSecurityScopedResource() };
    if !started {
        return Err("startAccessingSecurityScopedResource returned false".to_string());
    }
    Ok(BookmarkAccess { url: resolved.url })
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2_app_kit::{NSModalResponseOK, NSOpenPanel};
    use objc2_foundation::{
        MainThreadMarker, NSData, NSString, NSURL, NSURLBookmarkCreationOptions,
        NSURLBookmarkResolutionOptions, NSUserDefaults,
    };
    use std::path::PathBuf;

    pub struct Resolved {
        #[allow(dead_code)]
        pub path: String,
        #[allow(dead_code)]
        pub stale: bool,
        pub url: objc2::rc::Retained<NSURL>,
    }

    pub fn pick_directory_with_bookmark(
        suggested: Option<&str>,
    ) -> Result<PickFolderResponse, String> {
        // SAFETY: tauri::AppHandle::run_on_main_thread guarantees this
        // closure executes on the AppKit main thread.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };

        let panel = NSOpenPanel::openPanel(mtm);
        panel.setCanChooseDirectories(true);
        panel.setCanChooseFiles(false);
        panel.setAllowsMultipleSelection(false);
        let prompt = NSString::from_str("폴더 선택");
        panel.setPrompt(Some(&prompt));
        let title = NSString::from_str("프로젝트 폴더 연결");
        panel.setTitle(Some(&title));
        let message = NSString::from_str(
            "Hearth가 이 폴더를 Finder/터미널에서 열 수 있도록 폴더를 한 번 선택해 주세요.",
        );
        panel.setMessage(Some(&message));

        if let Some(dir) = suggested {
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
        let path = url_to_path_string(&url)?;
        Ok(PickFolderResponse {
            path,
            bookmark: blob,
        })
    }

    pub fn resolve_bookmark(blob: &[u8]) -> Result<Resolved, String> {
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

    fn defaults() -> objc2::rc::Retained<NSUserDefaults> {
        NSUserDefaults::standardUserDefaults()
    }

    pub fn read_blob(key: &str) -> Option<Vec<u8>> {
        let key = NSString::from_str(key);
        defaults().dataForKey(&key).map(|d| d.to_vec())
    }

    pub fn write_blob(key: &str, blob: &[u8]) {
        let key_ns = NSString::from_str(key);
        let data = NSData::with_bytes(blob);
        let any: &objc2::runtime::AnyObject = (*data).as_ref();
        unsafe {
            defaults().setObject_forKey(Some(any), &key_ns);
        }
        defaults().synchronize();
    }

    pub fn clear_blob(key: &str) {
        let key_ns = NSString::from_str(key);
        defaults().removeObjectForKey(&key_ns);
        defaults().synchronize();
    }
}
