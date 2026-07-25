use std::fmt;

#[derive(Debug)]
pub enum SdkError {
    Http(reqwest::Error),
    Serialization(serde_json::Error),
    NotFound(String),
    Unauthorized(String),
    BadRequest(String),
    ServerError(String),
    ConnectionFailed(String),
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SdkError::Http(e) => write!(f, "[http] {}", e),
            SdkError::Serialization(e) => write!(f, "[serialization] {}", e),
            SdkError::NotFound(msg) => write!(f, "[not found] {}", msg),
            SdkError::Unauthorized(msg) => write!(f, "[unauthorized] {}", msg),
            SdkError::BadRequest(msg) => write!(f, "[bad request] {}", msg),
            SdkError::ServerError(msg) => write!(f, "[server error] {}", msg),
            SdkError::ConnectionFailed(msg) => write!(f, "[connection failed] {}", msg),
        }
    }
}

impl std::error::Error for SdkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SdkError::Http(e) => Some(e),
            SdkError::Serialization(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for SdkError {
    fn from(e: reqwest::Error) -> Self {
        SdkError::Http(e)
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(e: serde_json::Error) -> Self {
        SdkError::Serialization(e)
    }
}

pub type SdkResult<T> = Result<T, SdkError>;

pub(crate) fn map_status(status: reqwest::StatusCode, body: &str) -> SdkError {
    match status.as_u16() {
        400 => SdkError::BadRequest(body.to_string()),
        401 | 403 => SdkError::Unauthorized(body.to_string()),
        404 => SdkError::NotFound(body.to_string()),
        500..=599 => SdkError::ServerError(body.to_string()),
        code => SdkError::ServerError(format!("[{}] {}", code, body)),
    }
}
