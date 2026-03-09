use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::models::{AppError, ErrorCode};

pub(crate) fn new_tx_id() -> String {
    format!("tx_{}", uuid::Uuid::new_v4())
}

pub(crate) fn compute_json_hash(json: &serde_json::Value) -> String {
    // NOTE: Keep this dependency-free for now. This is NOT cryptographically strong.
    // We use stable JSON serialization + std hash as a lightweight content signature.
    let stable = serde_json::to_string(json).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    use std::hash::Hasher;
    stable.hash(&mut hasher);
    format!("h:{}", hasher.finish())
}

pub(crate) fn compute_patch_hash(patch_ops: &[serde_json::Value]) -> String {
    let stable = serde_json::to_string(patch_ops).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    use std::hash::Hash;
    use std::hash::Hasher;
    stable.hash(&mut hasher);
    format!("p:{}", hasher.finish())
}

pub(crate) fn app_err_vc(code: &str, message: String, recoverable: bool) -> AppError {
    let mapped = match code {
        "E_VC_CONFLICT_REVISION" | "E_VC_DUP_CALL_ID" => ErrorCode::Conflict,
        "E_VC_LOCK_TIMEOUT" => ErrorCode::Conflict,
        "E_VC_IO_WRITE_FAIL" => ErrorCode::IoError,
        "E_VC_RECOVERY_REQUIRED" => ErrorCode::MigrationRequired,
        _ => ErrorCode::Internal,
    };

    AppError {
        code: mapped,
        message: format!("{code}: {message}"),
        details: Some(serde_json::json!({ "code": code })),
        recoverable: Some(recoverable),
    }
}

pub(crate) struct FileLockGuard {
    path: PathBuf,
    _file: File,
}

impl FileLockGuard {
    pub(crate) fn acquire(lock_path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = lock_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(lock_path)
        {
            Ok(mut f) => {
                let _ = f.write_all(format!("pid={}\n", std::process::id()).as_bytes());
                Ok(Self {
                    path: lock_path.to_path_buf(),
                    _file: f,
                })
            }
            Err(_) => Err(app_err_vc(
                "E_VC_LOCK_TIMEOUT",
                "lock already held".to_string(),
                true,
            )),
        }
    }
}

impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
