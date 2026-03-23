#[cfg(test)]
pub mod inspiration_env {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    pub struct InspirationTempRootGuard {
        _lock: MutexGuard<'static, ()>,
        _dir: tempfile::TempDir,
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub fn enter_temp_root() -> InspirationTempRootGuard {
        let lock = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var("MAGIC_NOVEL_INSPIRATION_ROOT", dir.path());

        InspirationTempRootGuard {
            _lock: lock,
            _dir: dir,
        }
    }

    pub fn with_temp_root<T>(f: impl FnOnce() -> T) -> T {
        let _guard = enter_temp_root();
        f()
    }

    impl Drop for InspirationTempRootGuard {
        fn drop(&mut self) {
            std::env::remove_var("MAGIC_NOVEL_INSPIRATION_ROOT");
        }
    }
}
