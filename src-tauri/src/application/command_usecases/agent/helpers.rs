use crate::models::{AppError, ErrorCode};
use crate::services::ensure_dir;
use crate::utils::atomic_write::atomic_write_json;
use serde_json::json;
use std::path::PathBuf;

use super::{
    OpenAiProviderSettings, SaveOpenAiProviderSettingsInput, DEFAULT_LOCAL_EMBEDDING_BASE_URL,
    DEFAULT_OPENAI_MODEL, DEFAULT_PLANNING_GENERATION_MODE, DEFAULT_PROVIDER_TYPE,
};

const MAGEIC_DIR: &str = ".magic";
const SETTINGS_FILE: &str = "setting.json";
const AI_SETTINGS_ROOT_ENV: &str = "MAGIC_NOVEL_AI_SETTINGS_ROOT";

pub(super) async fn read_response_body(
    response: reqwest::Response,
    code: &str,
    message: &str,
) -> Result<String, AppError> {
    response.text().await.map_err(|e| {
        app_err(
            ErrorCode::IoError,
            &format!("{message}: {e}"),
            Some(json!({ "code": code })),
            true,
        )
    })
}

pub(super) fn parse_model_list_response(raw: String) -> Result<Vec<String>, AppError> {
    let value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        app_err(
            ErrorCode::JsonParseError,
            &format!("OpenAI 模型列表解析失败: {e}"),
            Some(json!({ "code": "E_AI_MODELS_PARSE_FAILED" })),
            false,
        )
    })?;

    let models = value
        .get("data")
        .and_then(|data| data.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(|id| id.as_str()))
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    if models.is_empty() {
        return Err(app_err(
            ErrorCode::Internal,
            "模型列表为空",
            Some(json!({ "code": "E_AI_MODELS_EMPTY" })),
            true,
        ));
    }

    Ok(models)
}

pub(super) fn build_chat_completions_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/chat/completions") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    }
}

pub(super) fn build_models_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');
    if normalized.ends_with("/models") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/models")
    } else {
        format!("{normalized}/v1/models")
    }
}

fn settings_file_path() -> Result<PathBuf, AppError> {
    let home = if let Some(root) = std::env::var_os(AI_SETTINGS_ROOT_ENV) {
        let root = PathBuf::from(root);
        if root.as_os_str().is_empty() {
            dirs::home_dir()
        } else {
            Some(root)
        }
    } else {
        dirs::home_dir()
    }
    .ok_or_else(|| {
        app_err(
            ErrorCode::Internal,
            "无法定位用户目录",
            Some(json!({ "code": "E_AI_SETTINGS_HOME_NOT_FOUND" })),
            false,
        )
    })?;

    let settings_dir = home.join(MAGEIC_DIR);
    ensure_dir(&settings_dir)?;
    Ok(settings_dir.join(SETTINGS_FILE))
}

pub(super) fn load_openai_provider_settings() -> Result<OpenAiProviderSettings, AppError> {
    let file = settings_file_path()?;
    if !file.exists() {
        return Ok(OpenAiProviderSettings::default());
    }

    let content = std::fs::read_to_string(&file).map_err(|e| {
        app_err(
            ErrorCode::IoError,
            &format!("读取 AI 配置失败: {e}"),
            Some(json!({ "code": "E_AI_SETTINGS_READ_FAILED" })),
            true,
        )
    })?;

    let mut parsed: OpenAiProviderSettings = serde_json::from_str(&content).map_err(|e| {
        app_err(
            ErrorCode::JsonParseError,
            &format!("AI 配置格式错误: {e}"),
            Some(json!({ "code": "E_AI_SETTINGS_PARSE_FAILED" })),
            false,
        )
    })?;

    parsed.provider_type = normalize_provider_type(Some(parsed.provider_type.as_str()));
    parsed.openai_enabled_models = normalize_models(parsed.openai_enabled_models);
    if !parsed
        .openai_enabled_models
        .iter()
        .any(|item| item == &parsed.openai_model)
    {
        parsed.openai_model = parsed
            .openai_enabled_models
            .first()
            .cloned()
            .unwrap_or_else(super::default_openai_model);
    }

    if parsed.openai_embedding_model.trim().is_empty() {
        parsed.openai_embedding_model = parsed.openai_model.clone();
    }

    if parsed.openai_embedding_base_url.trim().is_empty() {
        parsed.openai_embedding_base_url = parsed.openai_base_url.clone();
    }

    if parsed.openai_embedding_api_key.trim().is_empty() {
        parsed.openai_embedding_api_key = parsed.openai_api_key.clone();
    }

    if parsed.openai_local_embedding_base_url.trim().is_empty() {
        parsed.openai_local_embedding_base_url =
            super::DEFAULT_LOCAL_EMBEDDING_BASE_URL.to_string();
    }

    parsed.openai_embedding_source = match parsed.openai_embedding_source.trim() {
        "local" => "local".to_string(),
        _ => "provider".to_string(),
    };

    parsed.openai_embedding_detected = parsed
        .openai_enabled_models
        .iter()
        .any(|item| item == &parsed.openai_embedding_model);

    if parsed.openai_embedding_detection_reason.trim().is_empty() {
        parsed.openai_embedding_detection_reason = if parsed.openai_embedding_detected {
            String::new()
        } else {
            "embedding_model_unavailable".to_string()
        };
    }

    parsed.openai_embedding_enabled =
        parsed.openai_embedding_enabled && parsed.openai_embedding_detected;

    parsed.planning_generation_mode =
        normalize_planning_generation_mode(Some(parsed.planning_generation_mode.as_str()));
    parsed.planning_provider_type =
        normalize_provider_type(Some(parsed.planning_provider_type.as_str()));
    parsed.planning_enabled_models = normalize_optional_models(parsed.planning_enabled_models);

    let planning_model = parsed.planning_model.trim().to_string();
    if !planning_model.is_empty()
        && !parsed
            .planning_enabled_models
            .iter()
            .any(|item| item == &planning_model)
    {
        parsed
            .planning_enabled_models
            .insert(0, planning_model.clone());
    }

    parsed.planning_model = if planning_model.is_empty() {
        parsed
            .planning_enabled_models
            .first()
            .cloned()
            .unwrap_or_default()
    } else {
        planning_model
    };

    Ok(parsed)
}

pub(super) fn write_openai_provider_settings(
    settings: &OpenAiProviderSettings,
) -> Result<(), AppError> {
    let file = settings_file_path()?;
    atomic_write_json(&file, settings)
}

pub(super) fn normalize_models(input: Vec<String>) -> Vec<String> {
    let mut out = normalize_optional_models(input);

    if out.is_empty() {
        out.push(super::DEFAULT_OPENAI_MODEL.to_string());
    }

    out
}

pub(super) fn normalize_optional_models(input: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for item in input {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if out.iter().any(|value| value == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }

    out
}

pub(super) fn default_embedding_detection_reason() -> String {
    "embedding_model_unavailable".to_string()
}

pub(super) fn normalize_save_input(
    input: SaveOpenAiProviderSettingsInput,
    existing: &OpenAiProviderSettings,
) -> OpenAiProviderSettings {
    let base = input.openai_base_url.trim().to_string();
    let key = input.openai_api_key.trim().to_string();

    let mut enabled_models = normalize_models(input.openai_enabled_models);

    let model = input
        .openai_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            enabled_models
                .first()
                .map(String::as_str)
                .unwrap_or(DEFAULT_OPENAI_MODEL)
        })
        .to_string();

    if !enabled_models.iter().any(|value| value == &model) {
        enabled_models.insert(0, model.clone());
    }

    let embedding_model = input
        .openai_embedding_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&model)
        .to_string();

    let embedding_base = input
        .openai_embedding_base_url
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let embedding_key = input
        .openai_embedding_api_key
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let local_embedding_base = input
        .openai_local_embedding_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_LOCAL_EMBEDDING_BASE_URL)
        .to_string();

    let local_embedding_key = input
        .openai_local_embedding_api_key
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_string();

    let embedding_source = normalize_embedding_source(input.openai_embedding_source.as_deref());

    let embedding_detected = input
        .openai_embedding_detected
        .unwrap_or_else(|| enabled_models.iter().any(|value| value == &embedding_model));

    let embedding_detection_reason = input
        .openai_embedding_detection_reason
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            if embedding_detected {
                String::new()
            } else {
                default_embedding_detection_reason()
            }
        });

    let embedding_enabled = input.openai_embedding_enabled.unwrap_or(false) && embedding_detected;
    let provider_type = normalize_provider_type(
        input
            .provider_type
            .as_deref()
            .or(Some(existing.provider_type.as_str())),
    );
    let planning_generation_mode = normalize_planning_generation_mode(
        input
            .planning_generation_mode
            .as_deref()
            .or(Some(existing.planning_generation_mode.as_str())),
    );
    let planning_provider_type = normalize_provider_type(
        input
            .planning_provider_type
            .as_deref()
            .or(Some(existing.planning_provider_type.as_str())),
    );
    let planning_base_url = input
        .planning_base_url
        .as_deref()
        .unwrap_or(existing.planning_base_url.as_str())
        .trim()
        .to_string();
    let planning_api_key = input
        .planning_api_key
        .as_deref()
        .unwrap_or(existing.planning_api_key.as_str())
        .trim()
        .to_string();
    let mut planning_enabled_models = normalize_optional_models(
        input
            .planning_enabled_models
            .unwrap_or_else(|| existing.planning_enabled_models.clone()),
    );
    let requested_planning_model = input
        .planning_model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            let existing_model = existing.planning_model.trim();
            if existing_model.is_empty() {
                None
            } else {
                Some(existing_model.to_string())
            }
        });

    if let Some(model) = requested_planning_model.as_ref() {
        if !planning_enabled_models.iter().any(|value| value == model) {
            planning_enabled_models.insert(0, model.clone());
        }
    }

    let planning_model = requested_planning_model
        .or_else(|| planning_enabled_models.first().cloned())
        .unwrap_or_default();

    OpenAiProviderSettings {
        provider_type,
        openai_base_url: base,
        openai_api_key: key,
        openai_model: model,
        openai_embedding_model: embedding_model,
        openai_embedding_base_url: embedding_base,
        openai_embedding_api_key: embedding_key,
        openai_local_embedding_base_url: local_embedding_base,
        openai_local_embedding_api_key: local_embedding_key,
        openai_embedding_source: embedding_source,
        openai_embedding_enabled: embedding_enabled,
        openai_embedding_detected: embedding_detected,
        openai_embedding_detection_reason: embedding_detection_reason,
        openai_enabled_models: enabled_models,
        planning_generation_mode,
        planning_provider_type,
        planning_base_url,
        planning_api_key,
        planning_model,
        planning_enabled_models,
    }
}

pub(super) fn normalize_embedding_source(input: Option<&str>) -> String {
    match input.map(str::trim) {
        Some("local") => "local".to_string(),
        _ => "provider".to_string(),
    }
}

pub(super) fn normalize_provider_type(input: Option<&str>) -> String {
    match input.map(str::trim) {
        Some("openai") => "openai".to_string(),
        Some("anthropic") => "anthropic".to_string(),
        Some("gemini") => "gemini".to_string(),
        _ => DEFAULT_PROVIDER_TYPE.to_string(),
    }
}

pub(super) fn normalize_planning_generation_mode(input: Option<&str>) -> String {
    match input.map(str::trim) {
        Some("dedicated") => "dedicated".to_string(),
        _ => DEFAULT_PLANNING_GENERATION_MODE.to_string(),
    }
}

pub(super) fn app_err(
    code: ErrorCode,
    message: &str,
    details: Option<serde_json::Value>,
    recoverable: bool,
) -> AppError {
    AppError {
        code,
        message: message.to_string(),
        details,
        recoverable: Some(recoverable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_save_input_preserves_planning_settings_when_omitted() {
        let existing = OpenAiProviderSettings {
            provider_type: "gemini".to_string(),
            planning_generation_mode: "dedicated".to_string(),
            planning_provider_type: "anthropic".to_string(),
            planning_base_url: "https://planning.example/v1".to_string(),
            planning_api_key: "planning-secret".to_string(),
            planning_model: "claude-sonnet".to_string(),
            planning_enabled_models: vec!["claude-sonnet".to_string()],
            ..OpenAiProviderSettings::default()
        };

        let normalized = normalize_save_input(
            SaveOpenAiProviderSettingsInput {
                provider_type: None,
                openai_base_url: "https://primary.example/v1".to_string(),
                openai_api_key: "primary-secret".to_string(),
                openai_model: Some("gpt-4o-mini".to_string()),
                openai_embedding_model: None,
                openai_embedding_base_url: None,
                openai_embedding_api_key: None,
                openai_local_embedding_base_url: None,
                openai_local_embedding_api_key: None,
                openai_embedding_source: None,
                openai_embedding_enabled: Some(false),
                openai_embedding_detected: Some(false),
                openai_embedding_detection_reason: None,
                openai_enabled_models: vec!["gpt-4o-mini".to_string()],
                planning_generation_mode: None,
                planning_provider_type: None,
                planning_base_url: None,
                planning_api_key: None,
                planning_model: None,
                planning_enabled_models: None,
            },
            &existing,
        );

        assert_eq!(normalized.provider_type, "gemini");
        assert_eq!(normalized.planning_generation_mode, "dedicated");
        assert_eq!(normalized.planning_provider_type, "anthropic");
        assert_eq!(normalized.planning_base_url, "https://planning.example/v1");
        assert_eq!(normalized.planning_api_key, "planning-secret");
        assert_eq!(normalized.planning_model, "claude-sonnet");
        assert_eq!(
            normalized.planning_enabled_models,
            vec!["claude-sonnet".to_string()]
        );
    }
}
