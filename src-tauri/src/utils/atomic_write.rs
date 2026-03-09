use crate::models::AppError;
use std::fs;
use std::path::Path;

pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    let temp_path = path.with_extension("tmp");

    fs::write(&temp_path, content)?;

    if path.exists() {
        let backup_path = path.with_extension("bak");
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }
        fs::rename(path, &backup_path)?;
    }

    fs::rename(&temp_path, path)?;

    let backup_path = path.with_extension("bak");
    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    Ok(())
}

pub fn atomic_write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(data)?;
    atomic_write(path, &content)
}
