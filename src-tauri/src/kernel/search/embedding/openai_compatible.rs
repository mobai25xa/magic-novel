use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use crate::models::{AppError, ErrorCode};

const REQUEST_TIMEOUT_SECS: u64 = 10;
const MAX_RETRIES: usize = 2;

#[derive(Debug, Clone)]
pub struct EmbeddingProviderConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingDataItem>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingDataItem {
    index: usize,
    embedding: Vec<f32>,
}

pub fn embed_texts_openai_compatible(
    config: &EmbeddingProviderConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, AppError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let config_owned = config.clone();
        let texts_owned = texts.to_vec();

        return std::thread::spawn(move || {
            embed_texts_openai_compatible_inner(&config_owned, &texts_owned)
        })
        .join()
        .map_err(|_| {
            AppError::internal("E_SEARCH_EMBEDDINGS_RUNTIME_PANIC: embedding worker panicked")
        })?;
    }

    embed_texts_openai_compatible_inner(config, texts)
}

fn embed_texts_openai_compatible_inner(
    config: &EmbeddingProviderConfig,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, AppError> {
    if config.base_url.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "OpenAI baseUrl 未配置".to_string(),
            details: Some(json!({ "code": "E_AI_SETTINGS_MISSING_BASEURL" })),
            recoverable: Some(true),
        });
    }

    if config.api_key.trim().is_empty() {
        return Err(AppError {
            code: ErrorCode::InvalidArgument,
            message: "OpenAI apiKey 未配置".to_string(),
            details: Some(json!({ "code": "E_AI_SETTINGS_MISSING_APIKEY" })),
            recoverable: Some(true),
        });
    }

    if texts.is_empty() {
        return Ok(vec![]);
    }

    let url = build_embeddings_url(&config.base_url);
    let client = embedding_http_client();

    let mut attempts = 0usize;
    loop {
        match send_embeddings_request(client, &url, &config.api_key, &config.model, texts) {
            Ok(vectors) => return normalize_embeddings(vectors),
            Err(err) => {
                if attempts >= MAX_RETRIES {
                    return Err(err);
                }
                attempts += 1;
                let delay_ms = 150u64 * (1u64 << attempts);
                thread::sleep(Duration::from_millis(delay_ms));
            }
        }
    }
}

fn send_embeddings_request(
    client: &Client,
    url: &str,
    api_key: &str,
    model: &str,
    texts: &[String],
) -> Result<Vec<Vec<f32>>, AppError> {
    let body = json!({
        "model": model,
        "input": texts,
        "encoding_format": "float",
    });

    let response = post_embeddings_request(client, url, api_key, &body)?;
    let status = response.status();
    let raw = read_embeddings_response(response)?;

    if !status.is_success() {
        return Err(upstream_http_error(status.as_u16(), raw));
    }

    let parsed = parse_embeddings_response(&raw)?;
    response_to_vectors(parsed.data, texts.len())
}

fn post_embeddings_request(
    client: &Client,
    url: &str,
    api_key: &str,
    body: &serde_json::Value,
) -> Result<reqwest::blocking::Response, AppError> {
    client
        .post(url)
        .bearer_auth(api_key)
        .json(body)
        .send()
        .map_err(|e| AppError {
            code: ErrorCode::IoError,
            message: format!("请求 Embeddings 接口失败: {e}"),
            details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
            recoverable: Some(true),
        })
}

fn read_embeddings_response(response: reqwest::blocking::Response) -> Result<String, AppError> {
    response.text().map_err(|e| AppError {
        code: ErrorCode::IoError,
        message: format!("读取 Embeddings 响应失败: {e}"),
        details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
        recoverable: Some(true),
    })
}

fn upstream_http_error(status: u16, body: String) -> AppError {
    AppError {
        code: ErrorCode::Internal,
        message: format!("Embeddings 接口返回错误: HTTP {status}"),
        details: Some(json!({
            "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR",
            "status": status,
            "body": body,
        })),
        recoverable: Some(true),
    }
}

fn parse_embeddings_response(raw: &str) -> Result<EmbeddingResponse, AppError> {
    serde_json::from_str(raw).map_err(|e| AppError {
        code: ErrorCode::JsonParseError,
        message: format!("Embeddings 响应解析失败: {e}"),
        details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
        recoverable: Some(false),
    })
}

fn response_to_vectors(
    data: Vec<EmbeddingDataItem>,
    expected: usize,
) -> Result<Vec<Vec<f32>>, AppError> {
    if data.is_empty() {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: "Embeddings 响应为空".to_string(),
            details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
            recoverable: Some(true),
        });
    }

    let mut by_index = data;
    by_index.sort_by_key(|item| item.index);

    let vectors: Vec<Vec<f32>> = by_index.into_iter().map(|item| item.embedding).collect();

    if vectors.len() != expected {
        return Err(AppError {
            code: ErrorCode::Internal,
            message: format!(
                "Embeddings 数量不匹配: request={}, response={}",
                expected,
                vectors.len()
            ),
            details: Some(json!({ "code": "E_SEARCH_EMBEDDINGS_UPSTREAM_ERROR" })),
            recoverable: Some(true),
        });
    }

    Ok(vectors)
}

fn normalize_embeddings(vectors: Vec<Vec<f32>>) -> Result<Vec<Vec<f32>>, AppError> {
    let mut out = Vec::with_capacity(vectors.len());

    for mut vector in vectors {
        let norm = vector
            .iter()
            .fold(0.0f32, |acc, value| acc + value * value)
            .sqrt();

        if norm == 0.0 {
            return Err(AppError {
                code: ErrorCode::InvalidArgument,
                message: "Embedding 向量范数为 0".to_string(),
                details: Some(json!({ "code": "E_SEARCH_EMBEDDING_ZERO_VECTOR" })),
                recoverable: Some(false),
            });
        }

        for value in &mut vector {
            *value /= norm;
        }

        out.push(vector);
    }

    Ok(out)
}

fn embedding_http_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn build_embeddings_url(base_url: &str) -> String {
    let normalized = base_url.trim().trim_end_matches('/');

    if normalized.ends_with("/v1/embeddings") {
        normalized.to_string()
    } else if normalized.ends_with("/v1") {
        format!("{normalized}/embeddings")
    } else {
        format!("{normalized}/v1/embeddings")
    }
}

#[cfg(test)]
mod tests {
    use super::build_embeddings_url;

    #[test]
    fn build_embeddings_url_handles_v1_embeddings_suffix() {
        let url = build_embeddings_url("http://localhost:11434/v1/embeddings");
        assert_eq!(url, "http://localhost:11434/v1/embeddings");
    }

    #[test]
    fn build_embeddings_url_handles_v1_suffix() {
        let url = build_embeddings_url("http://localhost:11434/v1");
        assert_eq!(url, "http://localhost:11434/v1/embeddings");
    }

    #[test]
    fn build_embeddings_url_handles_plain_base_url() {
        let url = build_embeddings_url("http://localhost:11434");
        assert_eq!(url, "http://localhost:11434/v1/embeddings");
    }
}
