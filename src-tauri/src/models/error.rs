use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidArgument,
    NotFound,
    PermissionDenied,
    IoError,
    JsonParseError,
    SchemaValidationError,
    SchemaVersionUnsupported,
    MigrationRequired,
    MigrationFailed,
    ImportParseFailed,
    ExportFailed,
    Conflict,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recoverable: Option<bool>,
}

impl AppError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::NotFound,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }

    pub fn io_error(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::IoError,
            message: msg.into(),
            details: None,
            recoverable: Some(true),
        }
    }

    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidArgument,
            message: msg.into(),
            details: None,
            recoverable: Some(true),
        }
    }

    pub fn json_parse_error(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::JsonParseError,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }

    #[allow(dead_code)]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Internal,
            message: msg.into(),
            details: None,
            recoverable: Some(false),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::io_error(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::json_parse_error(err.to_string())
    }
}
