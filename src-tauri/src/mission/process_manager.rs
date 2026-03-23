//! Mission system - Worker process manager
//!
//! Spawns, monitors, and kills worker processes.
//! Manages stdin/stdout NDJSON pipes via tokio mpsc channels.
//!
//! Based on docs/magic_plan/plan_agent/13-mission-worker-protocol.md

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, Mutex as TokioMutex};

use crate::models::{AppError, ErrorCode};

use super::worker_protocol::*;
#[path = "../agent_engine/delegate_runtime/mod.rs"]
pub mod delegate_runtime;
pub use delegate_runtime::{
    AttachedWorkerProcessTransport, DelegateRunContext, DelegateRunner, InProcessDelegateRunner,
    ProcessDelegateRunner, ProcessTransport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerBinaryResolution {
    EnvOverride {
        path: String,
    },
    ExecutableSibling {
        path: String,
    },
    DevTargetFallback {
        path: String,
    },
    Missing {
        attempted_env_override: Option<String>,
        attempted_executable_sibling: Option<String>,
        attempted_dev_target_fallbacks: Vec<String>,
    },
}

impl WorkerBinaryResolution {
    pub fn source_label(&self) -> &'static str {
        match self {
            Self::EnvOverride { .. } => "env_override",
            Self::ExecutableSibling { .. } => "executable_sibling",
            Self::DevTargetFallback { .. } => "dev_target_fallback",
            Self::Missing { .. } => "missing",
        }
    }

    pub fn path(&self) -> Option<&str> {
        match self {
            Self::EnvOverride { path }
            | Self::ExecutableSibling { path }
            | Self::DevTargetFallback { path } => Some(path.as_str()),
            Self::Missing { .. } => None,
        }
    }

    pub(crate) fn into_path_result(self) -> Result<String, AppError> {
        match self {
            Self::EnvOverride { path }
            | Self::ExecutableSibling { path }
            | Self::DevTargetFallback { path } => Ok(path),
            Self::Missing {
                attempted_env_override,
                attempted_executable_sibling,
                attempted_dev_target_fallbacks,
            } => {
                let message = build_missing_worker_binary_message(
                    attempted_env_override.as_deref(),
                    attempted_executable_sibling.as_deref(),
                    &attempted_dev_target_fallbacks,
                );
                Err(AppError {
                    code: ErrorCode::Internal,
                    message,
                    details: Some(serde_json::json!({
                        "worker_binary_resolution": {
                            "source": "missing",
                            "attempted_env_override": attempted_env_override,
                            "attempted_executable_sibling": attempted_executable_sibling,
                            "attempted_dev_target_fallbacks": attempted_dev_target_fallbacks,
                        }
                    })),
                    recoverable: Some(false),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerTransportDiagnosticKind {
    ProtocolIncompatibility,
    ParseError,
    MissingBinary,
    Other,
}

// ── WorkerProcess ───────────────────────────────────────────────

/// Handle to a running worker process.
#[derive(Clone)]
pub struct WorkerProcess {
    pub worker_id: String,
    pub pid: u32,
    child: Arc<Mutex<Child>>,
    stdin_tx: mpsc::Sender<WorkerInstruction>,
    event_rx: Arc<TokioMutex<mpsc::Receiver<Result<WorkerEvent, String>>>>,
    last_pong: Arc<Mutex<Instant>>,
}

impl WorkerProcess {
    /// Send an instruction to the worker via stdin.
    pub async fn send(&self, instruction: WorkerInstruction) -> Result<(), AppError> {
        self.stdin_tx
            .send(instruction)
            .await
            .map_err(|e| AppError::internal(format!("failed to send to worker: {e}")))
    }

    /// Receive the next event from the worker (blocks until available or channel closed).
    pub async fn recv(&self) -> Option<Result<WorkerEvent, String>> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    /// Send initialize instruction and wait for ack.
    pub async fn initialize(&self, project_path: &str, mission_dir: &str) -> Result<(), AppError> {
        let req_id = new_request_id();
        let expected_ack_id = format!("res_{req_id}");
        let instruction = WorkerInstruction::initialize(
            &req_id,
            InitializePayload {
                worker_id: self.worker_id.clone(),
                project_path: project_path.to_string(),
                mission_dir: mission_dir.to_string(),
            },
        );
        self.send(instruction).await?;

        // Wait for matching ack with timeout.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(AppError::internal("worker initialize timeout"));
            }

            let remaining = deadline.saturating_duration_since(now);
            let next = tokio::time::timeout(remaining, self.recv())
                .await
                .map_err(|_| AppError::internal("worker initialize timeout"))?
                .ok_or_else(|| AppError::internal("worker closed before ack"))?;

            let event = match next {
                Ok(event) => event,
                Err(e) => return Err(worker_stream_error("initialize", &self.worker_id, &e)),
            };

            if event.event_type != WorkerEventType::Ack {
                continue;
            }

            if event.id != expected_ack_id {
                tracing::warn!(
                    target: "mission",
                    worker_id = %self.worker_id,
                    expected_ack_id = %expected_ack_id,
                    received_ack_id = %event.id,
                    "ignoring unrelated worker ack during initialize"
                );
                continue;
            }

            let payload: AckPayload = serde_json::from_value(event.payload).map_err(|e| {
                AppError::internal(format!(
                    "worker initialize ack payload parse error (worker_id={}): {e}",
                    self.worker_id
                ))
            })?;

            if !payload.ok {
                let error = payload.error.unwrap_or_default();
                return Err(worker_rejection_error(
                    "initialize",
                    &self.worker_id,
                    &error,
                ));
            }

            return Ok(());
        }
    }

    /// Send start_feature instruction.
    pub async fn start_feature(&self, payload: StartFeaturePayload) -> Result<(), AppError> {
        let req_id = new_request_id();
        let instruction = WorkerInstruction::start_feature(&req_id, payload);
        self.send(instruction).await
    }

    pub async fn start_feature_and_wait_ack(
        &self,
        payload: StartFeaturePayload,
        timeout: Duration,
    ) -> Result<(), AppError> {
        let req_id = new_request_id();
        let expected_ack_id = format!("res_{req_id}");
        let instruction = WorkerInstruction::start_feature(&req_id, payload);
        self.send(instruction).await?;

        let next = tokio::time::timeout(timeout, async {
            loop {
                match self.recv().await {
                    Some(Ok(event)) if event.event_type == WorkerEventType::Ack => {
                        if event.id != expected_ack_id {
                            tracing::warn!(
                                target: "mission",
                                worker_id = %self.worker_id,
                                expected_ack_id = %expected_ack_id,
                                received_ack_id = %event.id,
                                "ignoring unrelated worker ack during start_feature"
                            );
                            continue;
                        }

                        let payload: AckPayload = serde_json::from_value(event.payload)
                            .map_err(|e| {
                                AppError::internal(format!(
                                    "worker start_feature ack payload parse error (worker_id={}): {e}",
                                    self.worker_id
                                ))
                            })?;

                        if !payload.ok {
                            let error = payload.error.unwrap_or_default();
                            return Err(worker_rejection_error(
                                "start_feature",
                                &self.worker_id,
                                &error,
                            ));
                        }

                        return Ok(());
                    }
                    Some(Ok(_)) => continue,
                    Some(Err(error)) => {
                        return Err(worker_stream_error(
                            "start_feature",
                            &self.worker_id,
                            &error,
                        ));
                    }
                    None => {
                        return Err(AppError::internal(
                            "worker closed before start_feature ack",
                        ))
                    }
                }
            }
        })
        .await;

        next.map_err(|_| AppError::internal("worker start_feature timeout"))?
    }

    /// Soft kill: send cancel + shutdown instructions.
    pub async fn soft_kill(&self) -> Result<(), AppError> {
        let cancel = WorkerInstruction::cancel(&new_request_id(), None);
        let _ = self.send(cancel).await;

        let shutdown = WorkerInstruction::shutdown(&new_request_id());
        let _ = self.send(shutdown).await;

        Ok(())
    }

    /// Hard kill: OS-level process termination.
    pub fn hard_kill(&self) -> Result<(), AppError> {
        let mut child = self
            .child
            .lock()
            .map_err(|e| AppError::internal(format!("child mutex poisoned: {e}")))?;
        child
            .kill()
            .map_err(|e| AppError::internal(format!("failed to kill worker: {e}")))
    }

    /// Kill with soft-then-hard strategy.
    pub async fn kill(&self, grace_period: Duration) -> Result<(), AppError> {
        self.soft_kill().await?;
        tokio::time::sleep(grace_period).await;

        // Check if process already exited
        let mut child = self
            .child
            .lock()
            .map_err(|e| AppError::internal(format!("child mutex poisoned: {e}")))?;

        match child.try_wait() {
            Ok(Some(_)) => Ok(()), // already exited
            _ => {
                tracing::warn!(
                    target: "mission",
                    worker_id = %self.worker_id,
                    pid = self.pid,
                    "hard killing worker"
                );
                child
                    .kill()
                    .map_err(|e| AppError::internal(format!("hard kill failed: {e}")))
            }
        }
    }

    /// Send a heartbeat ping.
    pub async fn send_ping(&self) -> Result<(), AppError> {
        let ping = WorkerInstruction::ping(&new_request_id());
        self.send(ping).await
    }

    /// Record that a pong was received.
    pub fn record_pong(&self) {
        if let Ok(mut last) = self.last_pong.lock() {
            *last = Instant::now();
        }
    }

    /// Check if worker has timed out (no pong for the given duration).
    pub fn is_timed_out(&self, timeout: Duration) -> bool {
        self.last_pong
            .lock()
            .map(|last| last.elapsed() > timeout)
            .unwrap_or(true)
    }

    /// Check if the child process is still alive.
    pub fn is_alive(&self) -> bool {
        self.child
            .lock()
            .map(|mut c| c.try_wait().map(|s| s.is_none()).unwrap_or(false))
            .unwrap_or(false)
    }

    /// Get the process ID.
    pub fn pid(&self) -> u32 {
        self.pid
    }
}

// ── ProcessManager ──────────────────────────────────────────────

/// Manages spawning and lifecycle of worker processes.
pub struct ProcessManager {
    worker_binary_path: String,
}

impl ProcessManager {
    pub fn new(worker_binary_path: String) -> Self {
        Self { worker_binary_path }
    }

    /// Resolve the worker binary path for the current platform.
    pub fn resolve_binary_path(base_path: &str) -> String {
        if cfg!(target_os = "windows") {
            if base_path.ends_with(".exe") {
                base_path.to_string()
            } else {
                format!("{base_path}.exe")
            }
        } else {
            base_path.to_string()
        }
    }

    /// Resolve the worker binary path at runtime.
    ///
    /// Strategy (in order):
    /// 1. `MAGIC_WORKER_BINARY` env var (explicit override for testing/CI)
    /// 2. Same directory as the current executable (production / packaged app)
    /// 3. Development fallback under `target/{debug|release}`
    pub fn resolve_worker_binary() -> WorkerBinaryResolution {
        Self::resolve_worker_binary_with(
            std::env::var("MAGIC_WORKER_BINARY").ok(),
            std::env::current_exe().ok(),
            Self::default_dev_candidates(),
        )
    }

    pub fn find_worker_binary() -> Result<String, crate::models::AppError> {
        let resolution = Self::resolve_worker_binary();
        if let Some(path) = resolution.path() {
            tracing::debug!(
                target: "mission",
                source = resolution.source_label(),
                path = %path,
                "resolved worker binary"
            );
        }
        resolution.into_path_result()
    }

    fn default_dev_candidates() -> Vec<String> {
        [
            "target/debug/agent_worker",
            "target/release/agent_worker",
            "../target/debug/agent_worker",
            "../target/release/agent_worker",
            "../../target/debug/agent_worker",
            "../../target/release/agent_worker",
        ]
        .into_iter()
        .map(Self::resolve_binary_path)
        .collect()
    }

    fn worker_binary_name() -> &'static str {
        if cfg!(target_os = "windows") {
            "agent_worker.exe"
        } else {
            "agent_worker"
        }
    }

    fn resolve_worker_binary_with(
        env_override: Option<String>,
        current_exe: Option<PathBuf>,
        dev_candidates: Vec<String>,
    ) -> WorkerBinaryResolution {
        let env_override = env_override
            .map(|raw| Self::resolve_binary_path(raw.trim()))
            .filter(|path| !path.trim().is_empty());

        if let Some(path) = env_override {
            if std::path::Path::new(&path).exists() {
                return WorkerBinaryResolution::EnvOverride { path };
            }

            return WorkerBinaryResolution::Missing {
                attempted_env_override: Some(path),
                attempted_executable_sibling: None,
                attempted_dev_target_fallbacks: Vec::new(),
            };
        }

        let executable_sibling = current_exe
            .as_ref()
            .and_then(|exe_path| exe_path.parent())
            .map(|exe_dir| exe_dir.join(Self::worker_binary_name()))
            .map(|path| path.to_string_lossy().to_string());

        if let Some(path) = executable_sibling.as_ref() {
            if std::path::Path::new(path).exists() {
                return WorkerBinaryResolution::ExecutableSibling { path: path.clone() };
            }
        }

        for candidate in &dev_candidates {
            if std::path::Path::new(candidate).exists() {
                return WorkerBinaryResolution::DevTargetFallback {
                    path: candidate.clone(),
                };
            }
        }

        WorkerBinaryResolution::Missing {
            attempted_env_override: None,
            attempted_executable_sibling: executable_sibling,
            attempted_dev_target_fallbacks: dev_candidates,
        }
    }

    /// Spawn a new worker process.
    ///
    /// Returns a WorkerProcess handle with stdin/stdout channels.
    /// The worker_id should be unique per worker (e.g. "wk_{uuid}").
    pub fn spawn(&self, worker_id: &str) -> Result<WorkerProcess, AppError> {
        let binary = Self::resolve_binary_path(&self.worker_binary_path);

        let mut child = Command::new(&binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                AppError::internal(format!("failed to spawn worker binary '{}': {e}", binary))
            })?;

        let pid = child.id();

        let child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| AppError::internal("failed to capture worker stdin"))?;
        let child_stdout = child
            .stdout
            .take()
            .ok_or_else(|| AppError::internal("failed to capture worker stdout"))?;

        tracing::info!(
            target: "mission",
            worker_id = %worker_id,
            pid = pid,
            binary = %binary,
            "worker process spawned"
        );

        // ── stdin writer thread ─────────────────────────────────
        let (stdin_tx, mut stdin_rx) = mpsc::channel::<WorkerInstruction>(64);
        let writer_worker_id = worker_id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut writer = child_stdin;
            while let Some(instruction) = stdin_rx.blocking_recv() {
                match instruction.to_ndjson_line() {
                    Ok(line) => {
                        if writer.write_all(line.as_bytes()).is_err() {
                            tracing::error!(
                                target: "mission",
                                worker_id = %writer_worker_id,
                                "stdin write failed, stopping writer"
                            );
                            break;
                        }
                        if writer.write_all(b"\n").is_err() {
                            break;
                        }
                        if writer.flush().is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            target: "mission",
                            worker_id = %writer_worker_id,
                            error = %e,
                            "failed to serialize instruction"
                        );
                    }
                }
            }
            tracing::debug!(
                target: "mission",
                worker_id = %writer_worker_id,
                "stdin writer thread exiting"
            );
        });

        // ── stdout reader thread ────────────────────────────────
        let (event_tx, event_rx) = mpsc::channel::<Result<WorkerEvent, String>>(256);
        let reader_worker_id = worker_id.to_string();

        tokio::task::spawn_blocking(move || {
            let reader = BufReader::new(child_stdout);
            for line_result in reader.lines() {
                match line_result {
                    Ok(line) if line.trim().is_empty() => continue,
                    Ok(line) => match WorkerEvent::from_ndjson_line(&line) {
                        Ok(event) => {
                            if event_tx.blocking_send(Ok(event)).is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let err_text = e.to_string();
                            let classified = if is_protocol_incompatibility(&err_text) {
                                format!("protocol incompatibility: {err_text}")
                            } else {
                                format!("parse error: {err_text}")
                            };
                            tracing::warn!(
                                target: "mission",
                                worker_id = %reader_worker_id,
                                line = %line,
                                error = %e,
                                "failed to parse worker event line"
                            );
                            let _ = event_tx.blocking_send(Err(classified));
                        }
                    },
                    Err(e) => {
                        tracing::error!(
                            target: "mission",
                            worker_id = %reader_worker_id,
                            error = %e,
                            "worker stdout read error"
                        );
                        break;
                    }
                }
            }
            tracing::debug!(
                target: "mission",
                worker_id = %reader_worker_id,
                "stdout reader thread exiting"
            );
        });

        let child_arc = Arc::new(Mutex::new(child));

        Ok(WorkerProcess {
            worker_id: worker_id.to_string(),
            pid,
            child: child_arc,
            stdin_tx,
            event_rx: Arc::new(TokioMutex::new(event_rx)),
            last_pong: Arc::new(Mutex::new(Instant::now())),
        })
    }
}

fn build_missing_worker_binary_message(
    attempted_env_override: Option<&str>,
    attempted_executable_sibling: Option<&str>,
    attempted_dev_target_fallbacks: &[String],
) -> String {
    if let Some(path) = attempted_env_override {
        return format!(
            "agent_worker binary not found: MAGIC_WORKER_BINARY points to '{}' but the file does not exist",
            path
        );
    }

    let mut segments = Vec::new();
    if let Some(path) = attempted_executable_sibling {
        segments.push(format!("sibling candidate '{}'", path));
    }
    if !attempted_dev_target_fallbacks.is_empty() {
        segments.push(format!(
            "dev candidates [{}]",
            attempted_dev_target_fallbacks.join(", ")
        ));
    }

    if segments.is_empty() {
        "agent_worker binary not found".to_string()
    } else {
        format!(
            "agent_worker binary not found after checking {}; set MAGIC_WORKER_BINARY or build with `cargo build`",
            segments.join(" and ")
        )
    }
}

pub(crate) fn classify_worker_transport_diagnostic(message: &str) -> WorkerTransportDiagnosticKind {
    let lower = message.to_ascii_lowercase();
    if is_protocol_incompatibility(message) || lower.contains("protocol incompatibility") {
        WorkerTransportDiagnosticKind::ProtocolIncompatibility
    } else if lower.contains("parse error") {
        WorkerTransportDiagnosticKind::ParseError
    } else if lower.contains("binary not found") {
        WorkerTransportDiagnosticKind::MissingBinary
    } else {
        WorkerTransportDiagnosticKind::Other
    }
}

fn worker_stream_error(stage: &str, worker_id: &str, error: &str) -> AppError {
    match classify_worker_transport_diagnostic(error) {
        WorkerTransportDiagnosticKind::ProtocolIncompatibility => {
            AppError::invalid_argument(format!(
                "worker protocol incompatibility during {stage} (worker_id={worker_id}): {error}"
            ))
        }
        WorkerTransportDiagnosticKind::ParseError => AppError::internal(format!(
            "worker {stage} parse error (worker_id={worker_id}): {error}"
        )),
        WorkerTransportDiagnosticKind::MissingBinary => AppError::internal(format!(
            "worker {stage} failed due to missing binary (worker_id={worker_id}): {error}"
        )),
        WorkerTransportDiagnosticKind::Other => AppError::internal(format!(
            "worker {stage} transport error (worker_id={worker_id}): {error}"
        )),
    }
}

fn worker_rejection_error(stage: &str, worker_id: &str, error: &str) -> AppError {
    match classify_worker_transport_diagnostic(error) {
        WorkerTransportDiagnosticKind::ProtocolIncompatibility => {
            AppError::invalid_argument(format!(
                "worker protocol incompatibility during {stage} (worker_id={worker_id}): {error}"
            ))
        }
        WorkerTransportDiagnosticKind::ParseError => AppError::internal(format!(
            "worker {stage} rejected request with parse error (worker_id={worker_id}): {error}"
        )),
        WorkerTransportDiagnosticKind::MissingBinary => AppError::internal(format!(
            "worker {stage} rejected request due to missing binary (worker_id={worker_id}): {error}"
        )),
        WorkerTransportDiagnosticKind::Other => AppError::internal(format!(
            "worker {stage} rejected request (worker_id={worker_id}): {error}"
        )),
    }
}

pub(crate) fn is_protocol_incompatibility(message: &str) -> bool {
    message
        .to_ascii_lowercase()
        .contains("protocol schema mismatch")
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_binary_path_windows() {
        if cfg!(target_os = "windows") {
            assert_eq!(
                ProcessManager::resolve_binary_path("agent_worker"),
                "agent_worker.exe"
            );
            assert_eq!(
                ProcessManager::resolve_binary_path("agent_worker.exe"),
                "agent_worker.exe"
            );
        }
    }

    #[test]
    fn test_resolve_binary_path_unix() {
        if !cfg!(target_os = "windows") {
            assert_eq!(
                ProcessManager::resolve_binary_path("agent_worker"),
                "agent_worker"
            );
        }
    }

    #[test]
    fn resolve_worker_binary_with_prefers_existing_env_override() {
        let temp = tempdir().unwrap();
        let worker_path = temp.path().join(ProcessManager::worker_binary_name());
        std::fs::write(&worker_path, b"worker").unwrap();

        let resolution = ProcessManager::resolve_worker_binary_with(
            Some(worker_path.to_string_lossy().to_string()),
            None,
            Vec::new(),
        );

        assert_eq!(
            resolution,
            WorkerBinaryResolution::EnvOverride {
                path: worker_path.to_string_lossy().to_string()
            }
        );
        assert_eq!(resolution.source_label(), "env_override");
    }

    #[test]
    fn resolve_worker_binary_with_uses_executable_sibling_before_dev_fallback() {
        let temp = tempdir().unwrap();
        let exe_dir = temp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let current_exe = exe_dir.join("host_app.exe");
        std::fs::write(&current_exe, b"host").unwrap();

        let sibling = exe_dir.join(ProcessManager::worker_binary_name());
        std::fs::write(&sibling, b"worker").unwrap();

        let dev_candidate = temp
            .path()
            .join("target")
            .join(ProcessManager::worker_binary_name());
        std::fs::create_dir_all(dev_candidate.parent().unwrap()).unwrap();
        std::fs::write(&dev_candidate, b"dev-worker").unwrap();

        let resolution = ProcessManager::resolve_worker_binary_with(
            None,
            Some(current_exe),
            vec![dev_candidate.to_string_lossy().to_string()],
        );

        assert_eq!(
            resolution,
            WorkerBinaryResolution::ExecutableSibling {
                path: sibling.to_string_lossy().to_string()
            }
        );
    }

    #[test]
    fn resolve_worker_binary_with_uses_dev_fallback_when_sibling_missing() {
        let temp = tempdir().unwrap();
        let exe_dir = temp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let current_exe = exe_dir.join("host_app.exe");
        std::fs::write(&current_exe, b"host").unwrap();

        let dev_candidate = temp
            .path()
            .join("target")
            .join(ProcessManager::worker_binary_name());
        std::fs::create_dir_all(dev_candidate.parent().unwrap()).unwrap();
        std::fs::write(&dev_candidate, b"dev-worker").unwrap();

        let resolution = ProcessManager::resolve_worker_binary_with(
            None,
            Some(current_exe),
            vec![dev_candidate.to_string_lossy().to_string()],
        );

        assert_eq!(
            resolution,
            WorkerBinaryResolution::DevTargetFallback {
                path: dev_candidate.to_string_lossy().to_string()
            }
        );
    }

    #[test]
    fn resolve_worker_binary_with_reports_missing_attempts() {
        let temp = tempdir().unwrap();
        let exe_dir = temp.path().join("bin");
        std::fs::create_dir_all(&exe_dir).unwrap();
        let current_exe = exe_dir.join("host_app.exe");
        std::fs::write(&current_exe, b"host").unwrap();
        let dev_candidate = temp
            .path()
            .join("target")
            .join(ProcessManager::worker_binary_name());

        let resolution = ProcessManager::resolve_worker_binary_with(
            None,
            Some(current_exe),
            vec![dev_candidate.to_string_lossy().to_string()],
        );

        match resolution {
            WorkerBinaryResolution::Missing {
                attempted_env_override,
                attempted_executable_sibling,
                attempted_dev_target_fallbacks,
            } => {
                assert!(attempted_env_override.is_none());
                assert!(attempted_executable_sibling.is_some());
                assert_eq!(
                    attempted_dev_target_fallbacks,
                    vec![dev_candidate.to_string_lossy().to_string()]
                );
            }
            other => panic!("expected missing resolution, got {other:?}"),
        }
    }

    #[test]
    fn classify_worker_transport_diagnostic_distinguishes_protocol_parse_and_missing() {
        assert_eq!(
            classify_worker_transport_diagnostic(
                "protocol incompatibility: protocol schema mismatch"
            ),
            WorkerTransportDiagnosticKind::ProtocolIncompatibility
        );
        assert_eq!(
            classify_worker_transport_diagnostic("parse error: expected value"),
            WorkerTransportDiagnosticKind::ParseError
        );
        assert_eq!(
            classify_worker_transport_diagnostic("agent_worker binary not found"),
            WorkerTransportDiagnosticKind::MissingBinary
        );
        assert_eq!(
            classify_worker_transport_diagnostic("worker closed unexpectedly"),
            WorkerTransportDiagnosticKind::Other
        );
    }
}
