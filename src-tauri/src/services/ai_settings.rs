use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::application::command_usecases::agent::DEFAULT_OPENAI_MODEL;
use crate::models::{AppError, ErrorCode};
use crate::services::ensure_dir;

const MAGEIC_DIR: &str = ".magic";
const SETTINGS_FILE: &str = "setting.json";
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiSearchSettings {
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
}

impl Default for OpenAiSearchSettings {
    fn default() -> Self {
        Self {
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

    Ok(parsed)
}

fn settings_file_path() -> Result<PathBuf, AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError {
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

    if out.is_empty() {
        out.push(DEFAULT_OPENAI_MODEL.to_string());
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
