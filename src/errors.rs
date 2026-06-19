use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;
use serde_json::json;
use tracing::error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    #[error("Checksum verification failed")]
    ChecksumError,

    #[error("Signature verification failed")]
    SignatureError,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal server error: {0}")]
    InternalError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Hex decode error: {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Invalid UTF-8: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::DatabaseError(e) => {
                error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal database error".to_string(),
                )
            }
            AppError::ParseError(msg) => {
                error!("Parse error: {}", msg);
                (StatusCode::BAD_REQUEST, format!("Parse error: {}", msg))
            }
            AppError::InvalidFormat(msg) => {
                error!("Invalid format: {}", msg);
                (StatusCode::BAD_REQUEST, format!("Invalid format: {}", msg))
            }
            AppError::ChecksumError => {
                error!("Checksum verification failed");
                (StatusCode::BAD_REQUEST, "Checksum verification failed".to_string())
            }
            AppError::SignatureError => {
                error!("Signature verification failed");
                (StatusCode::UNAUTHORIZED, "Signature verification failed".to_string())
            }
            AppError::ValidationError(msg) => {
                error!("Validation error: {}", msg);
                (StatusCode::BAD_REQUEST, format!("Validation error: {}", msg))
            }
            AppError::NotFound(msg) => {
                error!("Not found: {}", msg);
                (StatusCode::NOT_FOUND, format!("Not found: {}", msg))
            }
            AppError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::SerializationError(e) => {
                error!("Serialization error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Serialization error".to_string(),
                )
            }
            AppError::Base64Error(e) => {
                error!("Base64 error: {:?}", e);
                (StatusCode::BAD_REQUEST, "Invalid Base64 encoding".to_string())
            }
            AppError::HexError(e) => {
                error!("Hex error: {:?}", e);
                (StatusCode::BAD_REQUEST, "Invalid hex encoding".to_string())
            }
            AppError::Utf8Error(e) => {
                error!("UTF-8 error: {:?}", e);
                (StatusCode::BAD_REQUEST, "Invalid UTF-8 encoding".to_string())
            }
        };

        let body = Json(json!({
            "success": false,
            "error": message,
            "status_code": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
