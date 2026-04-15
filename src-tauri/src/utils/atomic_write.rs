use crate::models::AppError;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn temp_path_for(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut file_name = path
        .file_name()
        .map(|value| value.to_os_string())
        .unwrap_or_else(|| "atomic_write".into());
    file_name.push(format!(".{}.tmp", uuid::Uuid::new_v4().simple()));
    parent.join(file_name)
}

fn atomic_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_atomic_write() -> MutexGuard<'static, ()> {
    atomic_write_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_temp_file(path: &Path, content: &str) -> Result<(), AppError> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

fn backup_path_for(path: &Path) -> PathBuf {
    path.with_extension("bak")
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), AppError> {
    let _guard = lock_atomic_write();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = temp_path_for(path);
    write_temp_file(&temp_path, content)?;
    let backup_path = backup_path_for(path);

    if path.exists() {
        if backup_path.exists() {
            fs::remove_file(&backup_path)?;
        }

        if let Err(err) = fs::rename(path, &backup_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(err.into());
        }
    }

    if let Err(err) = fs::rename(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        if backup_path.exists() && !path.exists() {
            let _ = fs::rename(&backup_path, path);
        }
        return Err(err.into());
    }

    if backup_path.exists() {
        let _ = fs::remove_file(&backup_path);
    }

    Ok(())
}

pub fn atomic_write_json<T: serde::Serialize>(path: &Path, data: &T) -> Result<(), AppError> {
    let content = serde_json::to_string_pretty(data)?;
    atomic_write(path, &content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_existing_file_without_sidecar_files() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let path = temp_dir.path().join("state.json");

        atomic_write(&path, "first").expect("write initial content");
        atomic_write(&path, "second").expect("replace existing content");

        let content = fs::read_to_string(&path).expect("read replaced content");
        assert_eq!(content, "second");

        let entries = fs::read_dir(temp_dir.path())
            .expect("list temp dir")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();

        assert_eq!(entries, vec!["state.json".to_string()]);
    }
}
