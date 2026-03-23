use std::sync::Arc;

use serde_json::json;

use crate::models::{AppError, ErrorCode};

#[derive(Clone)]
pub(super) struct ResourceLockManager {
    locks: Arc<dashmap::DashMap<String, Arc<tokio::sync::Semaphore>>>,
}

impl ResourceLockManager {
    pub(super) fn new() -> Self {
        Self {
            locks: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub(super) async fn with_write_lock<F, Fut, T>(
        &self,
        resource_key: Option<String>,
        call_id: Option<String>,
        run: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        let Some(key) = resource_key else {
            return run().await;
        };

        let semaphore = self
            .locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1)))
            .clone();

        let _permit = semaphore.acquire_owned().await.map_err(|_| AppError {
            code: ErrorCode::Internal,
            message: "resource lock closed".to_string(),
            details: Some(json!({
                "code": "E_TOOL_RESOURCE_LOCK_CLOSED",
                "resource_key": key,
                "call_id": call_id,
            })),
            recoverable: Some(true),
        })?;

        run().await
    }
}
