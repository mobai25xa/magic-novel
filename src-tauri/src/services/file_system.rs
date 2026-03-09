use crate::models::AppError;
use std::fs;
use std::path::Path;

pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

pub fn read_file(path: &Path) -> Result<String, AppError> {
    if !path.exists() {
        return Err(AppError::not_found(format!("File not found: {:?}", path)));
    }
    fs::read_to_string(path).map_err(AppError::from)
}

pub fn write_file(path: &Path, content: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, content).map_err(AppError::from)
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, AppError> {
    let content = read_file(path)?;
    serde_json::from_str(&content).map_err(AppError::from)
}

pub fn write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(data)?;
    write_file(path, &content)
}

pub fn list_dirs(path: &Path) -> Result<Vec<String>, AppError> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut dirs = vec![];
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                dirs.push(name.to_string());
            }
        }
    }
    dirs.sort();
    Ok(dirs)
}

pub fn list_files(path: &Path, extension: &str) -> Result<Vec<String>, AppError> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut files = vec![];
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(extension) {
                    files.push(name.to_string());
                }
            }
        }
    }
    files.sort();
    Ok(files)
}
