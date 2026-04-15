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

#[cfg(test)]
pub mod ai_settings_env {
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    const AI_SETTINGS_ROOT_ENV: &str = "MAGIC_NOVEL_AI_SETTINGS_ROOT";

    pub struct AiSettingsTempRootGuard {
        lock: MutexGuard<'static, ()>,
        dir: tempfile::TempDir,
    }

    impl AiSettingsTempRootGuard {
        pub fn root(&self) -> &Path {
            self.dir.path()
        }
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    pub fn enter_temp_root() -> AiSettingsTempRootGuard {
        let lock = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = tempfile::tempdir().expect("tempdir");
        std::env::set_var(AI_SETTINGS_ROOT_ENV, dir.path());

        AiSettingsTempRootGuard { lock, dir }
    }

    pub fn with_temp_root<T>(f: impl FnOnce(&Path) -> T) -> T {
        let guard = enter_temp_root();
        f(guard.root())
    }

    impl Drop for AiSettingsTempRootGuard {
        fn drop(&mut self) {
            let _ = &self.lock;
            std::env::remove_var(AI_SETTINGS_ROOT_ENV);
        }
    }
}
