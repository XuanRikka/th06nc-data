use std::path::Path;

pub fn archive_key(path: &Path) -> String {
    let base = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = base.split('.').next().unwrap_or("");
    stem.to_string()
}