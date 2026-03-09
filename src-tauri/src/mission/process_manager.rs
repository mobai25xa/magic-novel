//! Mission system - Worker process manager
//!
//! Spawns, monitors, and kills worker processes.
//! Manages stdin/stdout NDJSON pipes via tokio mpsc channels.
//!
//! Based on docs/magic_plan/plan_agent/13-mission-worker-protocol.md

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::models::AppError;

use super::worker_protocol::*;

// ── WorkerProcess ───────────────────────────────────────────────

/// Handle to a running worker process.
pub struct WorkerProcess {
    pub worker_id: String,
    pub pid: u32,
    child: Arc<Mutex<Child>>,
    stdin_tx: mpsc::Sender<WorkerInstruction>,
    event_rx: mpsc::Receiver<Result<WorkerEvent, String>>,
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
    pub async fn recv(&mut self) -> Option<Result<WorkerEvent, String>> {
        self.event_rx.recv().await
    }

    /// Send initialize instruction and wait for ack.
    pub async fn initialize(
        &mut self,
        project_path: &str,
        mission_dir: &str,
    ) -> Result<(), AppError> {
        let req_id = new_request_id();
        let instruction = WorkerInstruction::initialize(
            &req_id,
            InitializePayload {
                worker_id: self.worker_id.clone(),
                project_path: project_path.to_string(),
                mission_dir: mission_dir.to_string(),
            },
        );
        self.send(instruction).await?;

        // Wait for ack with timeout
        let ack = tokio::time::timeout(Duration::from_secs(10), self.recv())
            .await
            .map_err(|_| AppError::internal("worker initialize timeout"))?
            .ok_or_else(|| AppError::internal("worker closed before ack"))?
            .map_err(|e| AppError::internal(format!("worker init parse error: {e}")))?;

        if ack.event_type != WorkerEventType::Ack {
            return Err(AppError::internal(format!(
                "expected ack, got: {:?}",
                ack.event_type
            )));
        }

        let payload: AckPayload = serde_json::from_value(ack.payload)
            .map_err(|e| AppError::internal(format!("ack payload parse error: {e}")))?;

        if !payload.ok {
            return Err(AppError::internal(format!(
                "worker init rejected: {}",
                payload.error.unwrap_or_default()
            )));
        }

        Ok(())
    }

    /// Send start_feature instruction.
    pub async fn start_feature(&self, payload: StartFeaturePayload) -> Result<(), AppError> {
        let req_id = new_request_id();
        let instruction = WorkerInstruction::start_feature(&req_id, payload);
        self.send(instruction).await
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
    pub fn find_worker_binary() -> Result<String, crate::models::AppError> {
        // 1. Explicit env override
        if let Ok(path) = std::env::var("MAGIC_WORKER_BINARY") {
            let resolved = Self::resolve_binary_path(&path);
            tracing::debug!(target: "mission", path = %resolved, "using MAGIC_WORKER_BINARY env override");
            return Ok(resolved);
        }

        // 2. Same directory as current executable (production / packaged app)
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let worker_name = if cfg!(target_os = "windows") {
                    "agent_worker.exe"
                } else {
                    "agent_worker"
                };
                let candidate = exe_dir.join(worker_name);
                if candidate.exists() {
                    let path = candidate.to_string_lossy().to_string();
                    tracing::debug!(target: "mission", path = %path, "found worker binary beside executable");
                    return Ok(path);
                }
            }
        }

        // 3. Development fallback under target dir
        let dev_candidates = [
            "target/debug/agent_worker",
            "target/release/agent_worker",
            "../target/debug/agent_worker",
            "../target/release/agent_worker",
            "../../target/debug/agent_worker",
            "../../target/release/agent_worker",
        ];

        for candidate in dev_candidates {
            let resolved = Self::resolve_binary_path(candidate);
            if std::path::Path::new(&resolved).exists() {
                tracing::debug!(target: "mission", path = %resolved, "found worker binary in target dir (dev)");
                return Ok(resolved);
            }
        }

        Err(crate::models::AppError::internal(
            "agent_worker binary not found. Set MAGIC_WORKER_BINARY env var or build with `cargo build`."
        ))
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
                            tracing::warn!(
                                target: "mission",
                                worker_id = %reader_worker_id,
                                line = %line,
                                error = %e,
                                "failed to parse worker event line"
                            );
                            let _ = event_tx.blocking_send(Err(format!("parse error: {e}")));
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
            event_rx,
            last_pong: Arc::new(Mutex::new(Instant::now())),
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
}
