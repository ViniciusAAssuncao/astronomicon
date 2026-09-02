use crate::error::RocketDbResult;
use std::path::{Path, PathBuf};

pub const SAVES_DIRECTORY: &str = "saves";

pub fn save_filename(timestamp: i64) -> String {
    format!("save-{timestamp}.db")
}

pub fn save_database_url(filename: &str) -> String {
    format!("sqlite://{SAVES_DIRECTORY}/{filename}")
}

pub fn save_path(filename: &str) -> PathBuf {
    Path::new(SAVES_DIRECTORY).join(filename)
}

pub async fn list_existing_saves() -> RocketDbResult<Vec<PathBuf>> {
    tokio::fs::create_dir_all(SAVES_DIRECTORY).await?;
    let mut entries = tokio::fs::read_dir(SAVES_DIRECTORY).await?;
    let mut saves = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let is_save_prefix = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("save-"))
            .unwrap_or(false);

        let is_db_ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "db")
            .unwrap_or(false);

        if is_save_prefix && is_db_ext {
            saves.push(path);
        }
    }

    Ok(saves)
}

pub fn most_recent_save(saves: &[PathBuf]) -> Option<&PathBuf> {
    saves.iter().max_by(|a, b| a.file_name().cmp(&b.file_name()))
}