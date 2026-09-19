use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use walkdir::WalkDir;

pub fn archive_key(path: &Path) -> String {
    let base = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = base.split('.').next().unwrap_or("");
    stem.to_string()
}

pub fn walk_files(root: &Path) -> impl Iterator<Item = Result<PathBuf>> {
    WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_map(move |entry| match entry {
            Ok(entry) if entry.file_type().is_file() => Some(Ok(entry.into_path())),
            Ok(_) => None,
            Err(err) => Some(Err(err).context(format!("遍历 {root:?} 时出错"))),
        })
}

#[macro_export]
macro_rules! padding_to_16 {
    ($num:expr) => {{
        let n = $num;
        (16 - (n % 16)) % 16
    }};
}
