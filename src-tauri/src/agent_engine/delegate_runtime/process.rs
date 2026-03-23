use async_trait::async_trait;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tauri::Emitter;

use crate::mission::delegate_types::{
    DelegateInputRef, DelegateRequest, DelegateResult, ExpectedOutputRef,
};
use crate::mission::process_manager::{
    classify_worker_transport_diagnostic, is_protocol_incompatibility, ProcessManager,
    WorkerBinaryResolution, WorkerProcess, WorkerTransportDiagnosticKind,
};
use crate::mission::result_types::{AgentTaskResult, OpenIssue, TaskResultStatus, TaskStopReason};
use crate::mission::types::{Feature, FeatureStatus};
use crate::mission::worker_protocol::{
    FeatureCompletedPayload, StartFeaturePayload, WorkerEventType,
};
use crate::models::AppError;

use super::runner::{DelegateRunContext, DelegateRunner};

#[async_trait]
pub trait ProcessTransport: Send + Sync {
    async fn run_via_transport(
        &self,
        context: DelegateRunContext,
    ) -> Result<AgentTaskResult, AppError>;
}

const DEFAULT_START_ACK_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_COMPLETION_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const DEFAULT_KILL_GRACE_PERIOD: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DelegateEventLineage {
    delegate_id: Option<String>,
    job_id: Option<String>,
    parent_task_id: Option<String>,
    parent_session_id: Option<String>,
    parent_turn_id: Option<u32>,
    actor_id: Option<String>,
    session_source: Option<String>,
}

impl DelegateEventLineage {
    fn from_context(context: &DelegateRunContext) -> Self {
        Self {
            delegate_id: optional_string(&context.request.delegate_id),
            job_id: optional_string(&context.request.job_id),
            parent_task_id: optional_string(&context.request.parent_task_id),
            parent_session_id: optional_string(&context.request.parent_session_id),
            parent_turn_id: context.request.parent_turn_id,
            actor_id: optional_string(&context.actor_id),
            session_source: Some(context.request.session_source.as_str().to_string()),
        }
    }

    fn is_empty(&self) -> bool {
        self.delegate_id.is_none()
            && self.job_id.is_none()
            && self.parent_task_id.is_none()
            && self.parent_session_id.is_none()
            && self.parent_turn_id.is_none()
            && self.actor_id.is_none()
            && self.session_source.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedTransportWorkerBinary {
    path: String,
    source_label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProcessTransportStage {
    ResolveBinary,
    SpawnWorker,
    InitializeWorker,
    StartFeature,
    AwaitCompletion,
}

impl ProcessTransportStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResolveBinary => "resolve_worker_binary",
            Self::SpawnWorker => "spawn_worker",
            Self::InitializeWorker => "initialize_worker",
            Self::StartFeature => "start_feature",
            Self::AwaitCompletion => "await_feature_completion",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerProcessTransport {
    worker_binary_path: Option<String>,
    start_ack_timeout: Duration,
    completion_timeout: Duration,
    kill_grace_period: Duration,
}

impl Default for WorkerProcessTransport {
    fn default() -> Self {
        Self {
            worker_binary_path: None,
            start_ack_timeout: DEFAULT_START_ACK_TIMEOUT,
            completion_timeout: DEFAULT_COMPLETION_TIMEOUT,
            kill_grace_period: DEFAULT_KILL_GRACE_PERIOD,
        }
    }
}

impl WorkerProcessTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_worker_binary_path(mut self, path: impl Into<String>) -> Self {
        let value = path.into().trim().to_string();
        self.worker_binary_path = if value.is_empty() { None } else { Some(value) };
        self
    }

    pub fn with_start_ack_timeout(mut self, timeout: Duration) -> Self {
        self.start_ack_timeout = normalize_timeout(timeout, DEFAULT_START_ACK_TIMEOUT);
        self
    }

    pub fn with_completion_timeout(mut self, timeout: Duration) -> Self {
        self.completion_timeout = normalize_timeout(timeout, DEFAULT_COMPLETION_TIMEOUT);
        self
    }

    pub fn with_kill_grace_period(mut self, grace_period: Duration) -> Self {
        self.kill_grace_period = normalize_timeout(grace_period, DEFAULT_KILL_GRACE_PERIOD);
        self
    }

    fn resolve_worker_binary_path(&self) -> Result<ResolvedTransportWorkerBinary, AppError> {
        match self.worker_binary_path.as_deref() {
            Some(path) if !path.trim().is_empty() => {
                let resolved = ProcessManager::resolve_binary_path(path.trim());
                if std::path::Path::new(&resolved).exists() {
                    Ok(ResolvedTransportWorkerBinary {
                        path: resolved,
                        source_label: "configured_path",
                    })
                } else {
                    Err(AppError {
                        code: crate::models::ErrorCode::Internal,
                        message: format!(
                            "configured worker binary path does not exist: '{}'",
                            resolved
                        ),
                        details: Some(serde_json::json!({
                            "worker_binary_resolution": {
                                "source": "configured_path",
                                "configured_path": resolved,
                            }
                        })),
                        recoverable: Some(false),
                    })
                }
            }
            _ => resolved_transport_worker_binary(ProcessManager::resolve_worker_binary()),
        }
    }
}

#[derive(Clone)]
pub struct AttachedWorkerProcessTransport {
    worker: WorkerProcess,
    app_handle: tauri::AppHandle,
    mission_id: String,
    worker_id: String,
    completion_timeout: Duration,
}

impl AttachedWorkerProcessTransport {
    pub fn new(
        worker: WorkerProcess,
        app_handle: tauri::AppHandle,
        mission_id: impl Into<String>,
        worker_id: impl Into<String>,
    ) -> Self {
        Self {
            worker,
            app_handle,
            mission_id: mission_id.into().trim().to_string(),
            worker_id: worker_id.into().trim().to_string(),
            completion_timeout: DEFAULT_COMPLETION_TIMEOUT,
        }
    }

    pub fn with_completion_timeout(mut self, timeout: Duration) -> Self {
        self.completion_timeout = normalize_timeout(timeout, DEFAULT_COMPLETION_TIMEOUT);
        self
    }
}

#[async_trait]
impl ProcessTransport for WorkerProcessTransport {
    async fn run_via_transport(
        &self,
        context: DelegateRunContext,
    ) -> Result<AgentTaskResult, AppError> {
        let context = context.normalized();
        validate_context(&context)?;
        let lineage = DelegateEventLineage::from_context(&context);

        let feature = build_delegate_feature(&context.request);
        let feature_id = feature.id.clone();
        let worker_id = format!("wk_delegate_{}", uuid::Uuid::new_v4());
        let session_id = build_session_id(&context.request, &feature_id);
        let payload = build_start_feature_payload(&context, &worker_id, session_id, feature);

        let worker_binary = self.resolve_worker_binary_path().map_err(|err| {
            attach_process_transport_context(
                err,
                ProcessTransportStage::ResolveBinary,
                &worker_id,
                None,
            )
        })?;
        tracing::info!(
            target: "mission",
            worker_id = %worker_id,
            binary_source = worker_binary.source_label,
            binary_path = %worker_binary.path,
            "starting delegate process transport worker"
        );
        let process_manager = ProcessManager::new(worker_binary.path.clone());
        let worker = process_manager.spawn(&worker_id).map_err(|err| {
            attach_process_transport_context(
                err,
                ProcessTransportStage::SpawnWorker,
                &worker_id,
                Some(&worker_binary),
            )
        })?;

        if let Err(err) = worker
            .initialize(&context.project_path, &context.mission_dir)
            .await
        {
            let _ = worker.hard_kill();
            return Err(attach_process_transport_context(
                err,
                ProcessTransportStage::InitializeWorker,
                &worker_id,
                Some(&worker_binary),
            ));
        }

        let run_result = async {
            worker
                .start_feature_and_wait_ack(payload, self.start_ack_timeout)
                .await
                .map_err(|err| {
                    attach_process_transport_context(
                        err,
                        ProcessTransportStage::StartFeature,
                        &worker_id,
                        Some(&worker_binary),
                    )
                })?;
            let raw_result = await_feature_completion(
                &worker,
                &feature_id,
                self.completion_timeout,
                None,
                None,
                None,
                Some(&lineage),
            )
            .await
            .map_err(|err| {
                attach_process_transport_context(
                    err,
                    ProcessTransportStage::AwaitCompletion,
                    &worker_id,
                    Some(&worker_binary),
                )
            })?;
            Ok(normalize_delegate_task_result(&context, raw_result))
        }
        .await;

        let kill_result = worker.kill(self.kill_grace_period).await;
        if let Err(kill_err) = kill_result {
            tracing::warn!(
                target: "mission",
                worker_id = %worker_id,
                error = %kill_err,
                "delegate worker cleanup failed after process transport run"
            );
        }

        run_result
    }
}

#[async_trait]
impl ProcessTransport for AttachedWorkerProcessTransport {
    async fn run_via_transport(
        &self,
        context: DelegateRunContext,
    ) -> Result<AgentTaskResult, AppError> {
        let context = context.normalized();
        validate_context(&context)?;
        let feature_id = delegate_parent_task_id(&context.request)?;
        let lineage = DelegateEventLineage::from_context(&context);

        let raw_result = await_feature_completion(
            &self.worker,
            &feature_id,
            self.completion_timeout,
            Some(&self.app_handle),
            Some(self.mission_id.as_str()),
            Some(self.worker_id.as_str()),
            Some(&lineage),
        )
        .await
        .map_err(|err| {
            attach_process_transport_context(
                err,
                ProcessTransportStage::AwaitCompletion,
                self.worker_id.as_str(),
                None,
            )
        })?;

        Ok(normalize_delegate_task_result(&context, raw_result))
    }
}

#[derive(Debug, Clone)]
pub struct ProcessDelegateRunner<T: ProcessTransport> {
    transport: T,
}

impl<T: ProcessTransport> ProcessDelegateRunner<T> {
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

fn resolved_transport_worker_binary(
    resolution: WorkerBinaryResolution,
) -> Result<ResolvedTransportWorkerBinary, AppError> {
    match resolution {
        WorkerBinaryResolution::EnvOverride { path } => Ok(ResolvedTransportWorkerBinary {
            path,
            source_label: "env_override",
        }),
        WorkerBinaryResolution::ExecutableSibling { path } => Ok(ResolvedTransportWorkerBinary {
            path,
            source_label: "executable_sibling",
        }),
        WorkerBinaryResolution::DevTargetFallback { path } => Ok(ResolvedTransportWorkerBinary {
            path,
            source_label: "dev_target_fallback",
        }),
        missing @ WorkerBinaryResolution::Missing { .. } => {
            missing
                .into_path_result()
                .map(|path| ResolvedTransportWorkerBinary {
                    path,
                    source_label: "missing",
                })
        }
    }
}

fn normalize_timeout(timeout: Duration, fallback: Duration) -> Duration {
    if timeout.is_zero() {
        fallback
    } else {
        timeout
    }
}

fn transport_failure_kind_label(err: &AppError) -> &'static str {
    match classify_worker_transport_diagnostic(&err.message) {
        WorkerTransportDiagnosticKind::ProtocolIncompatibility => "protocol incompatibility",
        WorkerTransportDiagnosticKind::ParseError => "parse error",
        WorkerTransportDiagnosticKind::MissingBinary => "missing binary",
        WorkerTransportDiagnosticKind::Other => "failure",
    }
}

fn attach_process_transport_context(
    mut err: AppError,
    stage: ProcessTransportStage,
    worker_id: &str,
    worker_binary: Option<&ResolvedTransportWorkerBinary>,
) -> AppError {
    let kind = transport_failure_kind_label(&err);
    let binary_source = worker_binary
        .map(|binary| binary.source_label.to_string())
        .or_else(|| {
            err.details
                .as_ref()
                .and_then(|details| details.get("worker_binary_resolution"))
                .and_then(|resolution| resolution.get("source"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unresolved".to_string());
    let binary_path = worker_binary.map(|binary| binary.path.clone()).or_else(|| {
        err.details
            .as_ref()
            .and_then(|details| details.get("worker_binary_resolution"))
            .and_then(|resolution| {
                [
                    "configured_path",
                    "attempted_env_override",
                    "attempted_executable_sibling",
                ]
                .into_iter()
                .find_map(|field| {
                    resolution
                        .get(field)
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
            })
    });

    let mut details = match err.details.take() {
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("cause_details".to_string(), other);
            map
        }
        None => serde_json::Map::new(),
    };
    details.insert(
        "process_transport".to_string(),
        serde_json::json!({
            "stage": stage.as_str(),
            "worker_id": worker_id,
            "binary_source": binary_source.clone(),
            "binary_path": binary_path.clone(),
            "failure_kind": kind,
        }),
    );

    let binary_context = if let Some(binary_path) = binary_path.as_deref() {
        format!("binary_source={binary_source}, binary_path={binary_path}")
    } else {
        format!("binary_source={binary_source}")
    };

    err.message = format!(
        "process delegate transport {kind} during {} for worker {} ({binary_context}): {}",
        stage.as_str(),
        worker_id,
        err.message
    );
    err.details = Some(serde_json::Value::Object(details));
    err
}

fn validate_context(context: &DelegateRunContext) -> Result<(), AppError> {
    if context.base_url.trim().is_empty() || context.api_key.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "process delegate run requires base_url and api_key",
        ));
    }
    if context.project_path.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "process delegate run requires project_path",
        ));
    }
    if context.mission_dir.trim().is_empty() {
        return Err(AppError::invalid_argument(
            "process delegate run requires mission_dir",
        ));
    }
    Ok(())
}

fn build_start_feature_payload(
    context: &DelegateRunContext,
    worker_id: &str,
    session_id: String,
    feature: Feature,
) -> StartFeaturePayload {
    let agent_profile = context.role_profile.to_agent_profile();
    let model = agent_profile
        .model
        .as_deref()
        .unwrap_or(context.model.as_str())
        .trim()
        .to_string();

    StartFeaturePayload {
        feature,
        session_id,
        model,
        provider: context.provider.clone(),
        base_url: context.base_url.clone(),
        api_key: context.api_key.clone(),
        mission_id: context.mission_id.clone(),
        worker_id: worker_id.to_string(),
        agent_profile: Some(agent_profile),
        session_source: context.request.session_source,
        parent_session_id: optional_string(&context.request.parent_session_id),
        parent_turn_id: context.request.parent_turn_id,
    }
}

fn build_session_id(request: &DelegateRequest, feature_id: &str) -> String {
    if let Some(parent_session_id) = optional_string(&request.parent_session_id) {
        format!(
            "{}_delegate_{}",
            sanitize_token(&parent_session_id),
            sanitize_token(feature_id)
        )
    } else {
        format!(
            "delegate_{}_{}",
            sanitize_token(feature_id),
            chrono::Utc::now().timestamp_millis()
        )
    }
}

fn build_delegate_feature(request: &DelegateRequest) -> Feature {
    let feature_id = delegate_feature_id(request);
    let expected_behavior = request
        .expected_outputs
        .iter()
        .map(|output| format_delegate_ref(output.kind.as_str(), output.value.as_str()))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let verification_steps = request
        .input_refs
        .iter()
        .map(|input| format_delegate_ref(input.kind.as_str(), input.value.as_str()))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let preconditions = build_lock_preconditions(request);

    Feature {
        id: feature_id,
        status: FeatureStatus::Pending,
        description: request.goal.trim().to_string(),
        skill: request.selected_profile_id.trim().to_string(),
        preconditions,
        depends_on: Vec::new(),
        expected_behavior,
        verification_steps,
        write_paths: candidate_write_paths(request),
    }
}

fn delegate_feature_id(request: &DelegateRequest) -> String {
    let requested = sanitize_token(&request.delegate_id);
    if requested.is_empty() {
        format!("delegate_{}", uuid::Uuid::new_v4())
    } else {
        requested
    }
}

fn candidate_write_paths(request: &DelegateRequest) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for candidate in request
        .input_refs
        .iter()
        .filter_map(candidate_path_from_input_ref)
    {
        if seen.insert(candidate.clone()) {
            paths.push(candidate);
        }
    }
    for candidate in request
        .expected_outputs
        .iter()
        .filter_map(candidate_path_from_expected_output)
    {
        if seen.insert(candidate.clone()) {
            paths.push(candidate);
        }
    }
    paths
}

fn candidate_path_from_input_ref(entry: &DelegateInputRef) -> Option<String> {
    candidate_path_from_parts(entry.kind.as_str(), entry.value.as_str())
}

fn candidate_path_from_expected_output(entry: &ExpectedOutputRef) -> Option<String> {
    candidate_path_from_parts(entry.kind.as_str(), entry.value.as_str())
}

fn candidate_path_from_parts(kind: &str, value: &str) -> Option<String> {
    let kind = kind.trim().to_ascii_lowercase();
    if matches!(
        kind.as_str(),
        "path" | "file" | "write_path" | "write-path" | "chapter_path" | "chapter"
    ) {
        normalize_candidate_path(value)
    } else {
        None
    }
}

fn build_lock_preconditions(request: &DelegateRequest) -> Vec<String> {
    if request.resource_locks.is_empty() {
        return Vec::new();
    }

    let specs = request
        .resource_locks
        .iter()
        .map(|lock| {
            let kind = format!("{:?}", lock.lock_kind).to_ascii_lowercase();
            let mode = match lock.mode {
                crate::mission::job_types::ResourceLockMode::Shared => "shared:",
                crate::mission::job_types::ResourceLockMode::Exclusive => "",
            };
            format!("{mode}{kind}:{}", lock.scope.trim())
        })
        .collect::<Vec<_>>();

    vec![format!("resource_locks:{}", specs.join(","))]
}

fn normalize_candidate_path(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') {
        return None;
    }
    if normalized.split('/').any(|part| part == "..") {
        return None;
    }

    Some(normalized)
}

fn format_delegate_ref(kind: &str, value: &str) -> String {
    let kind = kind.trim();
    let value = value.trim();
    if kind.is_empty() || value.is_empty() {
        String::new()
    } else {
        format!("{kind}: {value}")
    }
}

fn optional_string(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn sanitize_token(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn delegate_parent_task_id(request: &DelegateRequest) -> Result<String, AppError> {
    let feature_id = request.parent_task_id.trim();
    if feature_id.is_empty() {
        return Err(AppError::invalid_argument(
            "process delegate run requires parent_task_id",
        ));
    }
    Ok(feature_id.to_string())
}

async fn await_feature_completion(
    worker: &WorkerProcess,
    expected_feature_id: &str,
    completion_timeout: Duration,
    app_handle: Option<&tauri::AppHandle>,
    mission_id: Option<&str>,
    worker_id: Option<&str>,
    lineage: Option<&DelegateEventLineage>,
) -> Result<AgentTaskResult, AppError> {
    let deadline = Instant::now() + completion_timeout;
    let feature_label = if expected_feature_id.trim().is_empty() {
        "unknown".to_string()
    } else {
        expected_feature_id.trim().to_string()
    };
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(AppError::internal(format!(
                "worker feature completion timeout for feature '{}'",
                feature_label
            )));
        }

        let remaining = deadline.saturating_duration_since(now);
        let next = tokio::time::timeout(remaining, worker.recv())
            .await
            .map_err(|_| {
                AppError::internal(format!(
                    "worker feature completion timeout for feature '{}'",
                    feature_label
                ))
            })?;

        match next {
            Some(Ok(event)) => match event.event_type {
                WorkerEventType::AgentEvent => {
                    if let (Some(app_handle), Some(mission_id), Some(worker_id)) =
                        (app_handle, mission_id, worker_id)
                    {
                        let payload = enrich_worker_agent_event_payload(
                            event.payload,
                            mission_id,
                            worker_id,
                            lineage,
                        );
                        let _ = app_handle
                            .emit(crate::agent_engine::events::AGENT_EVENT_CHANNEL, &payload);
                    }
                }
                WorkerEventType::FeatureCompleted => {
                    let completed: FeatureCompletedPayload = serde_json::from_value(event.payload)
                        .map_err(|error| {
                            AppError::internal(format!(
                                "worker completion payload parse error for feature '{}': {error}",
                                feature_label
                            ))
                        })?;

                    if !expected_feature_id.trim().is_empty()
                        && completed.feature_id.trim() != expected_feature_id.trim()
                    {
                        tracing::warn!(
                            target: "mission",
                            expected_feature_id = %expected_feature_id,
                            received_feature_id = %completed.feature_id,
                            "ignoring mismatched worker completion payload"
                        );
                        continue;
                    }

                    return Ok(completed.result);
                }
                WorkerEventType::Pong => worker.record_pong(),
                WorkerEventType::Ack => {}
            },
            Some(Err(error)) => {
                if is_protocol_incompatibility(&error) {
                    return Err(AppError::invalid_argument(format!(
                        "worker protocol incompatibility during delegate run for feature '{}': {error}",
                        feature_label
                    )));
                }
                return Err(AppError::internal(format!(
                    "worker delegate transport parse error for feature '{}': {error}",
                    feature_label
                )));
            }
            None => {
                return Err(AppError::internal(format!(
                    "worker closed before feature completion for feature '{}'",
                    feature_label
                )))
            }
        }
    }
}

fn enrich_worker_agent_event_payload(
    mut payload: serde_json::Value,
    mission_id: &str,
    worker_id: &str,
    lineage: Option<&DelegateEventLineage>,
) -> serde_json::Value {
    let Some(obj) = payload.as_object_mut() else {
        return payload;
    };

    let source = obj
        .entry("source".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !source.is_object() {
        *source = serde_json::json!({});
    }

    if let Some(src) = source.as_object_mut() {
        src.insert("kind".to_string(), serde_json::json!("worker"));
        src.insert("worker_id".to_string(), serde_json::json!(worker_id));
        src.insert("mission_id".to_string(), serde_json::json!(mission_id));
    }

    if let Some(lineage) = lineage.filter(|value| !value.is_empty()) {
        let worker_session_id = obj.get("session_id").cloned();
        let worker_turn_id = obj.get("turn_id").cloned();
        let lineage_value = obj
            .entry("lineage".to_string())
            .or_insert_with(|| serde_json::json!({}));
        if !lineage_value.is_object() {
            *lineage_value = serde_json::json!({});
        }

        if let Some(lineage_obj) = lineage_value.as_object_mut() {
            if let Some(value) = worker_session_id {
                lineage_obj.insert("worker_session_id".to_string(), value);
            }
            if let Some(value) = worker_turn_id {
                lineage_obj.insert("worker_turn_id".to_string(), value);
            }
            if let Some(value) = lineage.delegate_id.as_ref() {
                lineage_obj.insert("delegate_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.job_id.as_ref() {
                lineage_obj.insert("job_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.parent_task_id.as_ref() {
                lineage_obj.insert("parent_task_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.parent_session_id.as_ref() {
                lineage_obj.insert("parent_session_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.parent_turn_id {
                lineage_obj.insert("parent_turn_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.actor_id.as_ref() {
                lineage_obj.insert("actor_id".to_string(), serde_json::json!(value));
            }
            if let Some(value) = lineage.session_source.as_ref() {
                lineage_obj.insert("session_source".to_string(), serde_json::json!(value));
            }
        }
    }

    payload
}

fn normalize_delegate_task_result(
    context: &DelegateRunContext,
    raw_result: AgentTaskResult,
) -> AgentTaskResult {
    let request = &context.request;
    let (status, stop_reason) =
        normalize_delegate_status_and_stop_reason(raw_result.status, raw_result.stop_reason);
    let summary = normalize_delegate_summary(request, &raw_result, status);
    let task_id = delegate_task_id(request, &raw_result);
    let actor_id = delegate_actor_id(context, &raw_result);
    let goal = delegate_goal(request, &raw_result);
    let next_actions =
        normalize_delegate_next_actions(&raw_result.next_actions, status, stop_reason);
    let AgentTaskResult {
        changed_paths,
        artifacts,
        evidence,
        open_issues: raw_open_issues,
        usage,
        ..
    } = raw_result;
    let open_issues =
        normalize_delegate_open_issues(raw_open_issues, &summary, status, stop_reason);

    AgentTaskResult {
        task_id,
        actor_id,
        goal,
        status,
        stop_reason,
        result_summary: summary,
        changed_paths,
        artifacts,
        evidence,
        open_issues,
        next_actions,
        usage,
    }
}

fn normalize_delegate_status_and_stop_reason(
    status: TaskResultStatus,
    stop_reason: TaskStopReason,
) -> (TaskResultStatus, TaskStopReason) {
    match stop_reason {
        TaskStopReason::Success => (TaskResultStatus::Completed, TaskStopReason::Success),
        TaskStopReason::Cancelled => (TaskResultStatus::Cancelled, TaskStopReason::Cancelled),
        TaskStopReason::Error => (TaskResultStatus::Failed, TaskStopReason::Error),
        TaskStopReason::Limit => (TaskResultStatus::Blocked, TaskStopReason::Limit),
        TaskStopReason::WaitingConfirmation => (
            TaskResultStatus::Blocked,
            TaskStopReason::WaitingConfirmation,
        ),
        TaskStopReason::WaitingAskuser => {
            (TaskResultStatus::Blocked, TaskStopReason::WaitingAskuser)
        }
        TaskStopReason::Blocked => (TaskResultStatus::Blocked, TaskStopReason::Blocked),
        TaskStopReason::Unknown => (status, TaskStopReason::Unknown),
    }
}

fn normalize_delegate_summary(
    request: &DelegateRequest,
    raw_result: &AgentTaskResult,
    status: TaskResultStatus,
) -> String {
    let summary = raw_result.result_summary.trim();
    if !summary.is_empty() {
        return summary.to_string();
    }

    if let Some(usage) = raw_result.usage.as_ref() {
        return format!(
            "Delegate '{}' stopped after {} rounds ({} tool calls)",
            delegate_summary_id(request, raw_result),
            usage.rounds_executed,
            usage.total_tool_calls
        );
    }

    match status {
        TaskResultStatus::Completed => "task completed".to_string(),
        TaskResultStatus::Failed => "task failed".to_string(),
        TaskResultStatus::Cancelled => "task cancelled".to_string(),
        TaskResultStatus::Blocked => "task blocked".to_string(),
    }
}

fn delegate_summary_id(request: &DelegateRequest, raw_result: &AgentTaskResult) -> String {
    optional_string(&request.delegate_id)
        .or_else(|| optional_string(&request.parent_task_id))
        .or_else(|| optional_string(&raw_result.task_id))
        .unwrap_or_else(|| "delegate".to_string())
}

fn normalize_delegate_next_actions(
    raw_actions: &[String],
    status: TaskResultStatus,
    stop_reason: TaskStopReason,
) -> Vec<String> {
    let normalized = raw_actions
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if !normalized.is_empty() && !all_worker_default_next_actions(&normalized) {
        return normalized;
    }

    delegate_next_actions(status, stop_reason)
}

fn all_worker_default_next_actions(actions: &[String]) -> bool {
    actions.iter().all(|value| {
        matches!(
            value.as_str(),
            "resume or rerun the delegate"
                | "inspect worker summary and retry"
                | "adjust limits or split the feature"
                | "provide approval from parent runtime"
                | "convert the task to interactive handling"
                | "inspect open issues before resuming"
        )
    })
}

fn delegate_next_actions(status: TaskResultStatus, stop_reason: TaskStopReason) -> Vec<String> {
    match status {
        TaskResultStatus::Completed => Vec::new(),
        TaskResultStatus::Cancelled => vec!["resume or rerun the delegate".to_string()],
        TaskResultStatus::Failed => vec!["inspect delegate summary and retry".to_string()],
        TaskResultStatus::Blocked => match stop_reason {
            TaskStopReason::Limit => vec!["adjust limits or split the delegate goal".to_string()],
            TaskStopReason::WaitingConfirmation => {
                vec!["provide approval from parent runtime".to_string()]
            }
            TaskStopReason::WaitingAskuser => {
                vec!["convert the task to interactive handling".to_string()]
            }
            _ => vec!["inspect open issues before resuming".to_string()],
        },
    }
}

fn normalize_delegate_open_issues(
    raw_issues: Vec<OpenIssue>,
    summary: &str,
    status: TaskResultStatus,
    stop_reason: TaskStopReason,
) -> Vec<OpenIssue> {
    if matches!(status, TaskResultStatus::Completed) {
        return raw_issues;
    }

    let mut normalized = raw_issues
        .into_iter()
        .map(|issue| OpenIssue {
            code: normalize_delegate_issue_code(issue.code, stop_reason),
            summary: issue.summary.trim().to_string(),
            blocking: issue.blocking,
        })
        .collect::<Vec<_>>();

    normalized.retain(|issue| !issue.summary.is_empty());

    if normalized.is_empty() {
        normalized.push(OpenIssue {
            code: Some(delegate_issue_code(stop_reason)),
            summary: summary.to_string(),
            blocking: true,
        });
    }

    normalized
}

fn normalize_delegate_issue_code(
    code: Option<String>,
    stop_reason: TaskStopReason,
) -> Option<String> {
    match code {
        Some(code) => {
            let trimmed = code.trim();
            if trimmed.is_empty() {
                Some(delegate_issue_code(stop_reason))
            } else if let Some(suffix) = trimmed.strip_prefix("worker::") {
                Some(format!("delegate::{suffix}"))
            } else {
                Some(trimmed.to_string())
            }
        }
        None => Some(delegate_issue_code(stop_reason)),
    }
}

fn delegate_issue_code(stop_reason: TaskStopReason) -> String {
    format!("delegate::{stop_reason:?}").to_ascii_lowercase()
}

fn delegate_task_id(request: &DelegateRequest, raw_result: &AgentTaskResult) -> String {
    optional_string(&request.parent_task_id)
        .or_else(|| optional_string(&request.delegate_id))
        .or_else(|| optional_string(&raw_result.task_id))
        .unwrap_or_default()
}

fn delegate_actor_id(context: &DelegateRunContext, raw_result: &AgentTaskResult) -> String {
    optional_string(&context.actor_id)
        .or_else(|| optional_string(&raw_result.actor_id))
        .or_else(|| optional_string(&context.request.delegate_id))
        .unwrap_or_default()
}

fn delegate_goal(request: &DelegateRequest, raw_result: &AgentTaskResult) -> String {
    optional_string(&request.goal)
        .or_else(|| optional_string(&raw_result.goal))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mission::delegate_types::TaskStopReason as DelegateTaskStopReason;
    use crate::mission::role_profile::RoleProfile;

    #[test]
    fn delegate_feature_id_uses_fallback_when_missing() {
        let request = DelegateRequest::default();
        let feature_id = delegate_feature_id(&request);
        assert!(feature_id.starts_with("delegate_"));
        assert!(!feature_id.trim().is_empty());
    }

    #[test]
    fn candidate_write_paths_filters_and_deduplicates_paths() {
        let request = DelegateRequest {
            input_refs: vec![
                DelegateInputRef {
                    kind: "file".to_string(),
                    value: "chapters/ch1.md".to_string(),
                    description: None,
                },
                DelegateInputRef {
                    kind: "path".to_string(),
                    value: "/abs/path.md".to_string(),
                    description: None,
                },
            ],
            expected_outputs: vec![
                ExpectedOutputRef {
                    kind: "write_path".to_string(),
                    value: "chapters/ch1.md".to_string(),
                    description: None,
                },
                ExpectedOutputRef {
                    kind: "chapter".to_string(),
                    value: "chapters/ch2.md".to_string(),
                    description: None,
                },
            ],
            ..DelegateRequest::default()
        };

        let paths = candidate_write_paths(&request);
        assert_eq!(
            paths,
            vec!["chapters/ch1.md".to_string(), "chapters/ch2.md".to_string()]
        );
    }

    #[test]
    fn build_session_id_uses_parent_session_when_present() {
        let request = DelegateRequest {
            parent_session_id: " parent/sess ".to_string(),
            delegate_id: "delegate-1".to_string(),
            ..DelegateRequest::default()
        };

        let session_id = build_session_id(&request, "delegate-1");
        assert!(session_id.starts_with("parent_sess_delegate_delegate-1"));
    }

    #[test]
    fn normalize_delegate_task_result_rewrites_worker_defaults() {
        let context = DelegateRunContext {
            request: DelegateRequest {
                delegate_id: "delegate-1".to_string(),
                parent_task_id: "feature-1".to_string(),
                job_id: "job-1".to_string(),
                goal: "Write the delegate result".to_string(),
                ..DelegateRequest::default()
            },
            role_profile: RoleProfile::default(),
            project_path: "project".to_string(),
            mission_dir: "mission".to_string(),
            mission_id: "mis_1".to_string(),
            actor_id: "wk_delegate".to_string(),
            provider: "openai".to_string(),
            model: "gpt-test".to_string(),
            base_url: "https://example.invalid".to_string(),
            api_key: "secret".to_string(),
        };
        let raw_result = AgentTaskResult {
            task_id: "delegate-1".to_string(),
            actor_id: "wk_worker".to_string(),
            goal: "raw worker goal".to_string(),
            status: TaskResultStatus::Completed,
            stop_reason: DelegateTaskStopReason::Error,
            result_summary: "worker failed".to_string(),
            open_issues: vec![OpenIssue {
                code: Some("worker::error".to_string()),
                summary: "worker failed".to_string(),
                blocking: true,
            }],
            next_actions: vec!["inspect worker summary and retry".to_string()],
            ..AgentTaskResult::default()
        };

        let normalized = normalize_delegate_task_result(&context, raw_result);

        assert_eq!(normalized.task_id, "feature-1");
        assert_eq!(normalized.actor_id, "wk_delegate");
        assert_eq!(normalized.goal, "Write the delegate result");
        assert_eq!(normalized.status, TaskResultStatus::Failed);
        assert_eq!(normalized.stop_reason, DelegateTaskStopReason::Error);
        assert_eq!(
            normalized.next_actions,
            vec!["inspect delegate summary and retry".to_string()]
        );
        assert_eq!(
            normalized.open_issues[0].code.as_deref(),
            Some("delegate::error")
        );
    }

    #[test]
    fn enrich_worker_agent_event_payload_attaches_delegate_lineage() {
        let payload = serde_json::json!({
            "session_id": "worker_session",
            "turn_id": 3,
            "source": {
                "kind": "worker"
            },
            "type": "TURN_STARTED",
            "payload": {}
        });
        let lineage = DelegateEventLineage {
            delegate_id: Some("delegate-1".to_string()),
            job_id: Some("job-1".to_string()),
            parent_task_id: Some("feature-1".to_string()),
            parent_session_id: Some("parent-session".to_string()),
            parent_turn_id: Some(9),
            actor_id: Some("wk_delegate".to_string()),
            session_source: Some("workflow_job".to_string()),
        };

        let enriched = enrich_worker_agent_event_payload(payload, "mis_1", "wk_1", Some(&lineage));

        let source = enriched
            .get("source")
            .and_then(|value| value.as_object())
            .expect("source should be present");
        assert_eq!(
            source.get("worker_id").and_then(|value| value.as_str()),
            Some("wk_1")
        );
        assert_eq!(
            source.get("mission_id").and_then(|value| value.as_str()),
            Some("mis_1")
        );

        let lineage = enriched
            .get("lineage")
            .and_then(|value| value.as_object())
            .expect("lineage should be present");
        assert_eq!(
            lineage
                .get("worker_session_id")
                .and_then(|value| value.as_str()),
            Some("worker_session")
        );
        assert_eq!(
            lineage
                .get("parent_session_id")
                .and_then(|value| value.as_str()),
            Some("parent-session")
        );
        assert_eq!(
            lineage
                .get("parent_turn_id")
                .and_then(|value| value.as_u64()),
            Some(9)
        );
        assert_eq!(
            lineage.get("delegate_id").and_then(|value| value.as_str()),
            Some("delegate-1")
        );
    }

    #[test]
    fn resolved_transport_worker_binary_preserves_resolution_source() {
        let env = resolved_transport_worker_binary(WorkerBinaryResolution::EnvOverride {
            path: "C:/workers/env_worker.exe".to_string(),
        })
        .expect("env override should resolve");
        assert_eq!(env.source_label, "env_override");
        assert_eq!(env.path, "C:/workers/env_worker.exe");

        let sibling = resolved_transport_worker_binary(WorkerBinaryResolution::ExecutableSibling {
            path: "C:/app/agent_worker.exe".to_string(),
        })
        .expect("sibling path should resolve");
        assert_eq!(sibling.source_label, "executable_sibling");

        let fallback =
            resolved_transport_worker_binary(WorkerBinaryResolution::DevTargetFallback {
                path: "target/debug/agent_worker.exe".to_string(),
            })
            .expect("dev fallback should resolve");
        assert_eq!(fallback.source_label, "dev_target_fallback");
    }

    #[test]
    fn resolved_transport_worker_binary_missing_resolution_returns_diagnostic_error() {
        let err = resolved_transport_worker_binary(WorkerBinaryResolution::Missing {
            attempted_env_override: Some("C:/missing/agent_worker.exe".to_string()),
            attempted_executable_sibling: None,
            attempted_dev_target_fallbacks: Vec::new(),
        })
        .expect_err("missing resolution should fail");

        assert!(err.message.contains("MAGIC_WORKER_BINARY"));
        let details = err
            .details
            .expect("missing worker resolution should include details");
        assert_eq!(
            details
                .get("worker_binary_resolution")
                .and_then(|value| value.get("source"))
                .and_then(|value| value.as_str()),
            Some("missing")
        );
    }

    #[test]
    fn attach_process_transport_context_adds_process_diagnostics() {
        let wrapped = attach_process_transport_context(
            AppError::invalid_argument(
                "worker protocol incompatibility during initialize (worker_id=wk_1): protocol incompatibility: protocol schema mismatch: expected worker_event, got worker_instruction",
            ),
            ProcessTransportStage::InitializeWorker,
            "wk_1",
            Some(&ResolvedTransportWorkerBinary {
                path: "C:/workers/agent_worker.exe".to_string(),
                source_label: "env_override",
            }),
        );

        assert!(wrapped
            .message
            .contains("process delegate transport protocol incompatibility"));
        assert!(wrapped.message.contains("initialize_worker"));
        assert!(wrapped.message.contains("binary_source=env_override"));
        assert!(wrapped
            .message
            .contains("binary_path=C:/workers/agent_worker.exe"));

        let details = wrapped
            .details
            .expect("wrapped error should include process transport details");
        assert_eq!(
            details
                .get("process_transport")
                .and_then(|value| value.get("stage"))
                .and_then(|value| value.as_str()),
            Some("initialize_worker")
        );
        assert_eq!(
            details
                .get("process_transport")
                .and_then(|value| value.get("failure_kind"))
                .and_then(|value| value.as_str()),
            Some("protocol incompatibility")
        );
    }

    #[test]
    fn attach_process_transport_context_uses_missing_resolution_details_without_binary_handle() {
        let wrapped = attach_process_transport_context(
            AppError {
                code: crate::models::ErrorCode::Internal,
                message:
                    "agent_worker binary not found: MAGIC_WORKER_BINARY points to 'C:/missing/agent_worker.exe' but the file does not exist"
                        .to_string(),
                details: Some(serde_json::json!({
                    "worker_binary_resolution": {
                        "source": "missing",
                        "attempted_env_override": "C:/missing/agent_worker.exe",
                    }
                })),
                recoverable: Some(false),
            },
            ProcessTransportStage::ResolveBinary,
            "wk_missing",
            None,
        );

        assert!(wrapped.message.contains("missing binary"));
        assert!(wrapped.message.contains("binary_source=missing"));
        assert!(wrapped
            .message
            .contains("binary_path=C:/missing/agent_worker.exe"));

        let details = wrapped
            .details
            .expect("missing binary wrapper should keep details");
        assert_eq!(
            details
                .get("process_transport")
                .and_then(|value| value.get("binary_source"))
                .and_then(|value| value.as_str()),
            Some("missing")
        );
    }
}

#[async_trait]
impl<T: ProcessTransport> DelegateRunner for ProcessDelegateRunner<T> {
    fn runner_kind(&self) -> &'static str {
        "process_transport"
    }

    async fn run_delegate(&self, context: DelegateRunContext) -> Result<DelegateResult, AppError> {
        let context = context.normalized();
        let request = context.request.clone();
        let result = self.transport.run_via_transport(context).await?;

        Ok(DelegateResult::from_agent_task_result(
            request.delegate_id,
            request.job_id,
            request.parent_task_id,
            result,
        ))
    }
}
