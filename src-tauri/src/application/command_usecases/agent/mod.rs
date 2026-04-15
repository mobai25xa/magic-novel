mod helpers;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::command;

use self::helpers::{
    app_err, build_chat_completions_url, build_models_url, default_embedding_detection_reason,
    load_openai_provider_settings, normalize_models, normalize_save_input,
    parse_model_list_response, read_response_body, write_openai_provider_settings,
};
use crate::models::{AppError, ErrorCode};

pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
pub const DEFAULT_LOCAL_EMBEDDING_BASE_URL: &str = "http://127.0.0.1:11434/v1";
pub const DEFAULT_PROVIDER_TYPE: &str = "openai-compatible";
pub const DEFAULT_PLANNING_GENERATION_MODE: &str = "follow_primary";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderSettings {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOpenAiProviderSettingsInput {
    #[serde(default)]
    pub provider_type: Option<String>,
    pub openai_base_url: String,
    pub openai_api_key: String,
    #[serde(default)]
    pub openai_model: Option<String>,
    #[serde(default)]
    pub openai_embedding_model: Option<String>,
    #[serde(default)]
    pub openai_embedding_base_url: Option<String>,
    #[serde(default)]
    pub openai_embedding_api_key: Option<String>,
    #[serde(default)]
    pub openai_local_embedding_base_url: Option<String>,
    #[serde(default)]
    pub openai_local_embedding_api_key: Option<String>,
    #[serde(default)]
    pub openai_embedding_source: Option<String>,
    #[serde(default)]
    pub openai_embedding_enabled: Option<bool>,
    #[serde(default)]
    pub openai_embedding_detected: Option<bool>,
    #[serde(default)]
    pub openai_embedding_detection_reason: Option<String>,
    #[serde(default)]
    pub openai_enabled_models: Vec<String>,
    #[serde(default)]
    pub planning_generation_mode: Option<String>,
    #[serde(default)]
    pub planning_provider_type: Option<String>,
    #[serde(default)]
    pub planning_base_url: Option<String>,
    #[serde(default)]
    pub planning_api_key: Option<String>,
    #[serde(default)]
    pub planning_model: Option<String>,
    #[serde(default)]
    pub planning_enabled_models: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchOpenAiModelsInput {
    pub openai_base_url: String,
    pub openai_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiModelListResult {
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiChatCompletionInput {
    pub messages: serde_json::Value,
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    #[serde(default)]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

impl Default for OpenAiProviderSettings {
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

#[command]
pub async fn get_openai_provider_settings() -> Result<OpenAiProviderSettings, AppError> {
    load_openai_provider_settings()
}

#[command]
pub async fn save_openai_provider_settings(
    input: SaveOpenAiProviderSettingsInput,
) -> Result<OpenAiProviderSettings, AppError> {
    let existing = load_openai_provider_settings().unwrap_or_default();
    let settings = normalize_save_input(input, &existing);

    write_openai_provider_settings(&settings)?;
    Ok(settings)
}

#[command]
pub async fn fetch_openai_models(
    input: FetchOpenAiModelsInput,
) -> Result<OpenAiModelListResult, AppError> {
    let base_url = input.openai_base_url.trim().to_string();
    let api_key = input.openai_api_key.trim().to_string();

    validate_provider_inputs(&base_url, &api_key)?;

    let response = request_model_list(build_models_url(&base_url), api_key).await?;
    let raw = read_response_body(
        response,
        "E_AI_MODELS_RESPONSE_READ_FAILED",
        "读取 OpenAI 模型列表响应失败",
    )
    .await?;
    let models = parse_model_list_response(raw)?;

    Ok(OpenAiModelListResult {
        models: normalize_models(models),
    })
}

#[command]
pub async fn ai_openai_chat_completion(
    input: OpenAiChatCompletionInput,
) -> Result<serde_json::Value, AppError> {
    let settings = load_openai_provider_settings()?;
    validate_provider_settings(&settings)?;

    let url = build_chat_completions_url(&settings.openai_base_url);
    let model = input
        .model
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&settings.openai_model)
        .to_string();

    let mut body = json!({
        "model": model,
        "messages": input.messages,
        "temperature": input.temperature.unwrap_or(0.2),
    });

    if let Some(tools) = input.tools {
        body["tools"] = tools;
    }

    if let Some(tool_choice) = input.tool_choice {
        body["tool_choice"] = tool_choice;
    }

    let response = request_chat_completion(url, settings.openai_api_key.clone(), body).await?;
    parse_chat_completion_response(response).await
}

fn validate_provider_settings(settings: &OpenAiProviderSettings) -> Result<(), AppError> {
    validate_provider_inputs(&settings.openai_base_url, &settings.openai_api_key)
}

fn validate_provider_inputs(base_url: &str, api_key: &str) -> Result<(), AppError> {
    if base_url.trim().is_empty() {
        return Err(app_err(
            ErrorCode::InvalidArgument,
            "OpenAI baseUrl 未配置",
            Some(json!({ "code": "E_AI_SETTINGS_MISSING_BASEURL" })),
            true,
        ));
    }

    if api_key.trim().is_empty() {
        return Err(app_err(
            ErrorCode::InvalidArgument,
            "OpenAI apiKey 未配置",
            Some(json!({ "code": "E_AI_SETTINGS_MISSING_APIKEY" })),
            true,
        ));
    }

    Ok(())
}

async fn request_chat_completion(
    url: String,
    api_key: String,
    body: serde_json::Value,
) -> Result<reqwest::Response, AppError> {
    Client::new()
        .post(url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            app_err(
                ErrorCode::IoError,
                &format!("请求 OpenAI 兼容接口失败: {e}"),
                Some(json!({ "code": "E_AI_HTTP_REQUEST_FAILED" })),
                true,
            )
        })
}

async fn request_model_list(url: String, api_key: String) -> Result<reqwest::Response, AppError> {
    Client::new()
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| {
            app_err(
                ErrorCode::IoError,
                &format!("请求 OpenAI 模型列表失败: {e}"),
                Some(json!({ "code": "E_AI_MODELS_REQUEST_FAILED" })),
                true,
            )
        })
}

async fn parse_chat_completion_response(
    response: reqwest::Response,
) -> Result<serde_json::Value, AppError> {
    let status = response.status();
    let raw = read_response_body(
        response,
        "E_AI_HTTP_RESPONSE_READ_FAILED",
        "读取 OpenAI 兼容接口响应失败",
    )
    .await?;

    if !status.is_success() {
        return Err(app_err(
            ErrorCode::Internal,
            &format!("OpenAI 兼容接口返回错误: HTTP {}", status.as_u16()),
            Some(json!({
                "code": "E_AI_UPSTREAM_ERROR",
                "status": status.as_u16(),
                "body": raw,
            })),
            true,
        ));
    }

    serde_json::from_str(&raw).map_err(|e| {
        app_err(
            ErrorCode::JsonParseError,
            &format!("OpenAI 响应解析失败: {e}"),
            Some(json!({ "code": "E_AI_RESPONSE_PARSE_FAILED" })),
            false,
        )
    })
}

fn default_openai_model() -> String {
    DEFAULT_OPENAI_MODEL.to_string()
}

fn default_local_embedding_base_url() -> String {
    DEFAULT_LOCAL_EMBEDDING_BASE_URL.to_string()
}

fn default_embedding_source() -> String {
    "provider".to_string()
}

fn default_embedding_enabled() -> bool {
    false
}

fn default_provider_type() -> String {
    DEFAULT_PROVIDER_TYPE.to_string()
}

fn default_planning_generation_mode() -> String {
    DEFAULT_PLANNING_GENERATION_MODE.to_string()
}
