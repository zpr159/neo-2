use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::request::RequestId;

/// Status of an inference response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InferenceStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
}

impl std::fmt::Display for InferenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceStatus::Success => write!(f, "Success"),
            InferenceStatus::Error => write!(f, "Error"),
            InferenceStatus::Timeout => write!(f, "Timeout"),
            InferenceStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

/// Error information included in a failed inference response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// The result of running inference on a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    request_id: RequestId,
    status: InferenceStatus,
    output: Option<serde_json::Value>,
    latency_ms: f64,
    error: Option<InferenceError>,
    metadata: HashMap<String, String>,
}

impl InferenceResponse {
    /// Creates a successful response.
    pub fn success(
        request_id: RequestId,
        output: serde_json::Value,
        latency_ms: f64,
    ) -> Self {
        Self {
            request_id,
            status: InferenceStatus::Success,
            output: Some(output),
            latency_ms,
            error: None,
            metadata: HashMap::new(),
        }
    }

    /// Creates an error response.
    pub fn error(
        request_id: RequestId,
        error: InferenceError,
        latency_ms: f64,
    ) -> Self {
        Self {
            request_id,
            status: InferenceStatus::Error,
            output: None,
            latency_ms,
            error: Some(error),
            metadata: HashMap::new(),
        }
    }

    /// Creates a timeout response.
    pub fn timeout(request_id: RequestId, latency_ms: f64) -> Self {
        Self {
            request_id,
            status: InferenceStatus::Timeout,
            output: None,
            latency_ms,
            error: Some(InferenceError {
                code: "TIMEOUT".to_string(),
                message: "Inference timed out".to_string(),
            }),
            metadata: HashMap::new(),
        }
    }

    /// Returns the original request ID.
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Returns the inference status.
    pub fn status(&self) -> InferenceStatus {
        self.status
    }

    /// Returns the output, if successful.
    pub fn output(&self) -> Option<&serde_json::Value> {
        self.output.as_ref()
    }

    /// Returns the latency in milliseconds.
    pub fn latency_ms(&self) -> f64 {
        self.latency_ms
    }

    /// Returns the error, if any.
    pub fn error(&self) -> Option<&InferenceError> {
        self.error.as_ref()
    }

    /// Returns a reference to the metadata.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Inserts a metadata entry.
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}
