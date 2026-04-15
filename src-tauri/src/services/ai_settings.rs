use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::application::command_usecases::agent::DEFAULT_OPENAI_MODEL;
use crate::models::{AppError, ErrorCode};
use crate::services::ensure_dir;

const MAGEIC_DIR: &str = ".magic";
const SETTINGS_FILE: &str = "setting.json";
const AI_SETTINGS_ROOT_ENV: &str = "MAGIC_NOVEL_AI_SETTINGS_ROOT";
const DEFAULT_PROVIDER_TYPE: &str = "openai-compatible";
const DEFAULT_PLANNING_GENERATION_MODE: &str = "follow_primary";
const PLANNING_PROVIDER_CONFIGURATION_INVALID: &str = "PlanningProviderConfigurationInvalid";
const PLANNING_SOURCE_TAG_PRIMARY: &str = "llm_primary";
const PLANNING_SOURCE_TAG_DEDICATED: &str = "llm_dedicated";
const PLANNING_SOURCE_TAG_DETERMINISTIC_FALLBACK: &str = "deterministic_fallback";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiSearchSettings {
    #[serde(default = "default_provider_type")]
    pub provider_type: String,
    #[serde(default)]
    pub openai_base_url: String,
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    #[serde(default)]
    pub openai_embedding_model: String,
    #[serde(default)]
    pub openai_embedding_base_url: String,
    #[serde(default)]
    pub openai_embedding_api_key: String,
    #[serde(default = "default_local_embedding_base_url")]
    pub openai_local_embedding_base_url: String,
    #[serde(default)]
    pub openai_local_embedding_api_key: String,
    #[serde(default = "default_embedding_source")]
    pub openai_embedding_source: String,
    #[serde(default = "default_embedding_enabled")]
    pub openai_embedding_enabled: bool,
    #[serde(default)]
    pub openai_embedding_detected: bool,
    #[serde(default)]
    pub openai_embedding_detection_reason: String,
    #[serde(default)]
    pub openai_enabled_models: Vec<String>,
    #[serde(default = "default_planning_generation_mode")]
    pub planning_generation_mode: String,
    #[serde(default = "default_provider_type")]
    pub planning_provider_type: String,
    #[serde(default)]
    pub planning_base_url: String,
    #[serde(default)]
    pub planning_api_key: String,
    #[serde(default)]
    pub planning_model: String,
    #[serde(default)]
    pub planning_enabled_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedPlanningGenerationConfig {
    pub mode: String,
    pub provider_type: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
    pub source_tag: String,
    pub can_use_llm: bool,
}

impl Default for OpenAiSearchSettings {
    fn default() -> Self {
        Self {
            provider_type: default_provider_type(),
            openai_base_url: String::new(),
            openai_api_key: String::new(),
            openai_model: DEFAULT_OPENAI_MODEL.to_string(),
            openai_embedding_model: DEFAULT_OPENAI_MODEL.to_string(),
            openai_embedding_base_url: String::new(),
            openai_embedding_api_key: String::new(),
            openai_local_embedding_base_url: default_local_embedding_base_url(),
            openai_local_embedding_api_key: String::new(),
            openai_embedding_source: default_embedding_source(),
            openai_embedding_enabled: default_embedding_enabled(),
            openai_embedding_detected: false,
            openai_embedding_detection_reason: default_embedding_detection_reason(),
            openai_enabled_models: vec![DEFAULT_OPENAI_MODEL.to_string()],
            planning_generation_mode: default_planning_generation_mode(),
            planning_provider_type: default_provider_type(),
            planning_base_url: String::new(),
            planning_api_key: String::new(),
            planning_model: String::new(),
            planning_enabled_models: Vec::new(),
        }
    }
}

pub fn load_openai_search_settings() -> Result<OpenAiSearchSettings, AppError> {
    let file = settings_file_path()?;
    if !file.exists() {
        return Ok(OpenAiSearchSettings::default());
    }

    let content = std::fs::read_to_string(&file).map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("读取 AI 配置失败: {e}"),
        details: Some(json!({ "code": "E_AI_SETTINGS_READ_FAILED" })),
        recoverable: Some(true),
    })?;

    let mut parsed: OpenAiSearchSettings =
        serde_json::from_str(&content).map_err(|e| AppError {
            code: ErrorCode::JsonParseError,
            message: format!("AI 配置格式错误: {e}"),
            details: Some(json!({ "code": "E_AI_SETTINGS_PARSE_FAILED" })),
            recoverable: Some(false),
        })?;

    parsed.provider_type = normalize_provider_type(parsed.provider_type.as_str());
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
            .unwrap_or_else(default_openai_model);
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
        parsed.openai_local_embedding_base_url = default_local_embedding_base_url();
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
            default_embedding_detection_reason()
        };
    }

    parsed.openai_embedding_enabled =
        parsed.openai_embedding_enabled && parsed.openai_embedding_detected;

    parsed.planning_generation_mode =
        normalize_planning_generation_mode(parsed.planning_generation_mode.as_str());
    parsed.planning_provider_type = normalize_provider_type(parsed.planning_provider_type.as_str());
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

pub fn resolve_planning_generation_config() -> Result<ResolvedPlanningGenerationConfig, AppError> {
    let settings = load_openai_search_settings()?;
    resolve_planning_generation_config_from_settings(&settings)
}

pub fn resolve_planning_generation_config_from_settings(
    settings: &OpenAiSearchSettings,
) -> Result<ResolvedPlanningGenerationConfig, AppError> {
    if settings.planning_generation_mode == "dedicated" {
        let base_url = settings.planning_base_url.trim().to_string();
        if base_url.is_empty() {
            return Err(planning_provider_configuration_error(
                "planning_base_url_missing",
                "创建期规划模型缺少 Base URL",
            ));
        }

        let api_key = settings.planning_api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(planning_provider_configuration_error(
                "planning_api_key_missing",
                "创建期规划模型缺少 API Key",
            ));
        }

        let enabled_models = normalize_optional_models(settings.planning_enabled_models.clone());
        if enabled_models.is_empty() {
            return Err(planning_provider_configuration_error(
                "planning_enabled_models_empty",
                "创建期规划模型缺少可用模型列表",
            ));
        }

        let model = settings.planning_model.trim().to_string();
        if model.is_empty() {
            return Err(planning_provider_configuration_error(
                "planning_model_missing",
                "创建期规划模型缺少默认模型",
            ));
        }

        return Ok(ResolvedPlanningGenerationConfig {
            mode: "dedicated".to_string(),
            provider_type: settings.planning_provider_type.clone(),
            model,
            base_url,
            api_key,
            source_tag: PLANNING_SOURCE_TAG_DEDICATED.to_string(),
            can_use_llm: true,
        });
    }

    let base_url = settings.openai_base_url.trim().to_string();
    let api_key = settings.openai_api_key.trim().to_string();
    let can_use_llm = !base_url.is_empty() && !api_key.is_empty();

    Ok(ResolvedPlanningGenerationConfig {
        mode: DEFAULT_PLANNING_GENERATION_MODE.to_string(),
        provider_type: settings.provider_type.clone(),
        model: settings.openai_model.clone(),
        base_url,
        api_key,
        source_tag: if can_use_llm {
            PLANNING_SOURCE_TAG_PRIMARY.to_string()
        } else {
            PLANNING_SOURCE_TAG_DETERMINISTIC_FALLBACK.to_string()
        },
        can_use_llm,
    })
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
    .ok_or_else(|| AppError {
        code: ErrorCode::Internal,
        message: "无法定位用户目录".to_string(),
        details: Some(json!({ "code": "E_AI_SETTINGS_HOME_NOT_FOUND" })),
        recoverable: Some(false),
    })?;

    let settings_dir = home.join(MAGEIC_DIR);
    ensure_dir(&settings_dir)?;
    Ok(settings_dir.join(SETTINGS_FILE))
}

fn normalize_models(input: Vec<String>) -> Vec<String> {
    let mut out = normalize_optional_models(input);

    if out.is_empty() {
        out.push(DEFAULT_OPENAI_MODEL.to_string());
    }

    out
}

fn normalize_optional_models(input: Vec<String>) -> Vec<String> {
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

fn default_openai_model() -> String {
    DEFAULT_OPENAI_MODEL.to_string()
}

fn default_local_embedding_base_url() -> String {
    "http://127.0.0.1:11434/v1".to_string()
}

fn default_embedding_source() -> String {
    "provider".to_string()
}

fn default_embedding_enabled() -> bool {
    false
}

fn default_embedding_detection_reason() -> String {
    "embedding_model_unavailable".to_string()
}

fn default_provider_type() -> String {
    DEFAULT_PROVIDER_TYPE.to_string()
}

fn default_planning_generation_mode() -> String {
    DEFAULT_PLANNING_GENERATION_MODE.to_string()
}

fn normalize_provider_type(input: &str) -> String {
    match input.trim() {
        "openai" => "openai".to_string(),
        "anthropic" => "anthropic".to_string(),
        "gemini" => "gemini".to_string(),
        _ => DEFAULT_PROVIDER_TYPE.to_string(),
    }
}

fn normalize_planning_generation_mode(input: &str) -> String {
    match input.trim() {
        "dedicated" => "dedicated".to_string(),
        _ => DEFAULT_PLANNING_GENERATION_MODE.to_string(),
    }
}

fn planning_provider_configuration_error(issue_code: &str, message: &str) -> AppError {
    AppError {
        code: ErrorCode::InvalidArgument,
        message: message.to_string(),
        details: Some(json!({
            "code": PLANNING_PROVIDER_CONFIGURATION_INVALID,
            "issue_code": issue_code,
        })),
        recoverable: Some(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_settings() -> OpenAiSearchSettings {
        OpenAiSearchSettings {
            provider_type: "openai".to_string(),
            openai_base_url: "https://primary.example/v1".to_string(),
            openai_api_key: "primary-key".to_string(),
            openai_model: "gpt-4o-mini".to_string(),
            openai_enabled_models: vec!["gpt-4o-mini".to_string()],
            ..OpenAiSearchSettings::default()
        }
    }

    #[test]
    fn follow_primary_incomplete_uses_deterministic_fallback() {
        let mut settings = base_settings();
        settings.openai_base_url.clear();
        settings.openai_api_key.clear();

        let resolved = resolve_planning_generation_config_from_settings(&settings)
            .expect("follow_primary should allow deterministic fallback");

        assert_eq!(resolved.mode, DEFAULT_PLANNING_GENERATION_MODE);
        assert_eq!(
            resolved.source_tag,
            PLANNING_SOURCE_TAG_DETERMINISTIC_FALLBACK
        );
        assert!(!resolved.can_use_llm);
    }

    #[test]
    fn dedicated_incomplete_returns_configuration_error() {
        let mut settings = base_settings();
        settings.planning_generation_mode = "dedicated".to_string();
        settings.planning_provider_type = "anthropic".to_string();
        settings.planning_api_key = "planning-key".to_string();
        settings.planning_model = "claude-sonnet".to_string();
        settings.planning_enabled_models = vec!["claude-sonnet".to_string()];

        let error = resolve_planning_generation_config_from_settings(&settings)
            .expect_err("dedicated mode should reject missing base url");

        assert!(matches!(error.code, ErrorCode::InvalidArgument));
        assert_eq!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("code"))
                .and_then(|value| value.as_str()),
            Some(PLANNING_PROVIDER_CONFIGURATION_INVALID)
        );
        assert_eq!(
            error
                .details
                .as_ref()
                .and_then(|details| details.get("issue_code"))
                .and_then(|value| value.as_str()),
            Some("planning_base_url_missing")
        );
    }

    #[test]
    fn dedicated_complete_returns_llm_dedicated() {
        let mut settings = base_settings();
        settings.planning_generation_mode = "dedicated".to_string();
        settings.planning_provider_type = "anthropic".to_string();
        settings.planning_base_url = "https://planning.example/v1".to_string();
        settings.planning_api_key = "planning-key".to_string();
        settings.planning_model = "claude-sonnet".to_string();
        settings.planning_enabled_models = vec!["claude-sonnet".to_string()];

        let resolved = resolve_planning_generation_config_from_settings(&settings)
            .expect("dedicated mode should resolve");

        assert_eq!(resolved.mode, "dedicated");
        assert_eq!(resolved.provider_type, "anthropic");
        assert_eq!(resolved.model, "claude-sonnet");
        assert_eq!(resolved.source_tag, PLANNING_SOURCE_TAG_DEDICATED);
        assert!(resolved.can_use_llm);
    }
}
