use magic_novel_lib::agent_engine::exposure_policy::{
    CapabilityPolicy, CapabilityPreset, ExposureContext, SessionSource,
};
use magic_novel_lib::agent_engine::types::{AgentMode, ApprovalMode, ClarificationMode};
use magic_novel_lib::agent_tools::definition::ToolSchemaContext;
use magic_novel_lib::agent_tools::registry::build_openai_tool_schema_report_for_exposure;
use magic_novel_lib::load_openai_search_settings;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, Serialize)]
struct SmokeScenarioResult {
    name: String,
    tool_count: usize,
    ok: bool,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SmokeSummary {
    provider: String,
    model: String,
    base_url: String,
    skipped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    scenarios: Vec<SmokeScenarioResult>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let require_provider = std::env::args()
        .skip(1)
        .any(|arg| arg == "--require-provider");
    let settings = load_openai_search_settings()?;
    let base_url = settings.openai_base_url.trim().to_string();
    let model = settings.openai_model.trim().to_string();

    if base_url.is_empty() || model.is_empty() {
        if require_provider {
            return Err(
                "tool provider smoke requires configured openai_base_url and openai_model".into(),
            );
        }

        let summary = SmokeSummary {
            provider: "openai-compatible".to_string(),
            model,
            base_url,
            skipped: true,
            reason: Some("missing_provider_configuration".to_string()),
            scenarios: Vec::new(),
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let context = ToolSchemaContext {
        clarification_mode: ClarificationMode::Interactive,
        ..ToolSchemaContext::default()
    };

    let scenarios = [
        (
            "main_interactive",
            ExposureContext::new(
                AgentMode::Writing,
                ApprovalMode::ConfirmWrites,
                ClarificationMode::Interactive,
                SessionSource::UserInteractive,
                0,
                context.semantic_retrieval_enabled,
                None,
                None,
                CapabilityPolicy::new(CapabilityPreset::MainInteractive),
            ),
        ),
        (
            "main_planning",
            ExposureContext::new(
                AgentMode::Planning,
                ApprovalMode::ConfirmWrites,
                ClarificationMode::Interactive,
                SessionSource::UserInteractive,
                0,
                context.semantic_retrieval_enabled,
                None,
                None,
                CapabilityPolicy::new(CapabilityPreset::MainPlanning),
            ),
        ),
        (
            "headless_writer",
            ExposureContext::new(
                AgentMode::Writing,
                ApprovalMode::Auto,
                ClarificationMode::HeadlessDefer,
                SessionSource::WorkflowJob,
                0,
                context.semantic_retrieval_enabled,
                None,
                None,
                CapabilityPolicy::new(CapabilityPreset::HeadlessWriter),
            ),
        ),
        (
            "read_only_reviewer",
            ExposureContext::new(
                AgentMode::Planning,
                ApprovalMode::Auto,
                ClarificationMode::HeadlessDefer,
                SessionSource::ReviewGate,
                0,
                context.semantic_retrieval_enabled,
                None,
                None,
                CapabilityPolicy::new(CapabilityPreset::ReadOnlyReviewer),
            ),
        ),
    ];

    let mut results = Vec::new();
    for (name, exposure) in scenarios {
        let tool_report = build_openai_tool_schema_report_for_exposure(&exposure, &context);

        let request_body = build_request_body(&model, &tool_report.tools);
        let url = completions_url(&base_url);
        let mut request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body);
        if !settings.openai_api_key.trim().is_empty() {
            request = request.bearer_auth(settings.openai_api_key.trim());
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();

        if status == 200 {
            let finish_reason = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|json| json.get("choices")?.as_array()?.first().cloned())
                .and_then(|choice| {
                    choice
                        .get("finish_reason")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                });

            results.push(SmokeScenarioResult {
                name: name.to_string(),
                tool_count: tool_report.tools.len(),
                ok: true,
                status,
                finish_reason,
                error: None,
            });
            continue;
        }

        results.push(SmokeScenarioResult {
            name: name.to_string(),
            tool_count: tool_report.tools.len(),
            ok: false,
            status,
            finish_reason: None,
            error: Some(truncate(&body, 240)),
        });
    }

    let summary = SmokeSummary {
        provider: "openai-compatible".to_string(),
        model,
        base_url,
        skipped: false,
        reason: None,
        scenarios: results.clone(),
    };
    println!("{}", serde_json::to_string_pretty(&summary)?);

    if results.iter().any(|result| !result.ok) {
        return Err("one or more tool provider smoke scenarios failed".into());
    }

    Ok(())
}

fn completions_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    }
}

fn build_request_body(model: &str, tools: &[serde_json::Value]) -> serde_json::Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": "Reply with the single word ok. Do not call any tools."
            }
        ],
        "temperature": 0,
        "max_tokens": 8,
        "stream": false,
    });

    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("none");
    }

    body
}

fn truncate(input: &str, max_len: usize) -> String {
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.len() <= max_len {
        normalized
    } else {
        format!("{}...", &normalized[..max_len])
    }
}
