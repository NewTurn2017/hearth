use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct ScanHit {
    pub path: String,
    pub name: String,
    pub already_registered: bool,
}

pub fn scan_dir(
    dir: &Path,
    depth: u32,
    existing_paths: &[String],
) -> std::io::Result<Vec<ScanHit>> {
    let mut hits = Vec::new();
    walk(dir, depth, existing_paths, &mut hits)?;
    Ok(hits)
}

fn walk(
    dir: &Path,
    depth: u32,
    existing: &[String],
    out: &mut Vec<ScanHit>,
) -> std::io::Result<()> {
    if depth == 0 {
        return Ok(());
    }
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path: PathBuf = entry.path();
        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if file_name.starts_with('.') {
            continue;
        }
        if !path.is_dir() {
            continue;
        }
        let path_str = path.to_string_lossy().to_string();
        let hit = ScanHit {
            path: path_str.clone(),
            name: file_name,
            already_registered: existing.contains(&path_str),
        };
        out.push(hit);
        walk(&path, depth - 1, existing, out)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_reports_subdirs_and_flags_existing() {
        let d = TempDir::new().unwrap();
        std::fs::create_dir_all(d.path().join("proj_a")).unwrap();
        std::fs::create_dir_all(d.path().join("proj_b")).unwrap();
        let existing = vec![d.path().join("proj_a").to_string_lossy().to_string()];
        let hits = scan_dir(d.path(), 1, &existing).unwrap();
        assert_eq!(hits.len(), 2);
        let a = hits.iter().find(|h| h.name == "proj_a").unwrap();
        assert!(a.already_registered);
        let b = hits.iter().find(|h| h.name == "proj_b").unwrap();
        assert!(!b.already_registered);
    }
}
