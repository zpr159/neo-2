use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::api::ApiError;
use crate::NeoError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RestError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    Internal(String),
    ServiceUnavailable(String),
}

impl RestError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            RestError::NotFound(_) => StatusCode::NOT_FOUND,
            RestError::BadRequest(_) => StatusCode::BAD_REQUEST,
            RestError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            RestError::Forbidden(_) => StatusCode::FORBIDDEN,
            RestError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            RestError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for RestError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            RestError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            RestError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            RestError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            RestError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            RestError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            RestError::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
        };

        let body = serde_json::json!({
            "error": {
                "code": status.as_u16(),
                "message": message,
            }
        });

        (status, axum::Json(body)).into_response()
    }
}

impl From<ApiError> for RestError {
    fn from(err: ApiError) -> Self {
        match err.code {
            404 => RestError::NotFound(err.message),
            400 => RestError::BadRequest(err.message),
            401 => RestError::Unauthorized(err.message),
            403 => RestError::Forbidden(err.message),
            503 => RestError::ServiceUnavailable(err.message),
            _ => RestError::Internal(err.message),
        }
    }
}

impl From<NeoError> for RestError {
    fn from(err: NeoError) -> Self {
        match err {
            NeoError::NotFound(msg) => RestError::NotFound(msg),
            NeoError::InvalidInput(msg) => RestError::BadRequest(msg),
            NeoError::PermissionDenied(msg) => RestError::Forbidden(msg),
            NeoError::AlreadyExists(msg) => RestError::BadRequest(msg),
            NeoError::Timeout(msg) => RestError::ServiceUnavailable(msg),
            NeoError::ResourceExhausted(msg) => RestError::ServiceUnavailable(msg),
            NeoError::NotImplemented(msg) => RestError::Internal(msg),
            _ => RestError::Internal(err.to_string()),
        }
    }
}

impl std::fmt::Display for RestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            RestError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            RestError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            RestError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            RestError::Internal(msg) => write!(f, "Internal Server Error: {}", msg),
            RestError::ServiceUnavailable(msg) => write!(f, "Service Unavailable: {}", msg),
        }
    }
}

impl std::error::Error for RestError {}
