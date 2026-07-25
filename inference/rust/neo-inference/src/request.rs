use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an inference request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(pub Uuid);

impl RequestId {
    /// Creates a new unique RequestId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An inference request carrying input data and parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    id: RequestId,
    model_id: Uuid,
    input: serde_json::Value,
    parameters: HashMap<String, serde_json::Value>,
    timestamp: DateTime<Utc>,
}

impl InferenceRequest {
    /// Creates a new inference request for the given model and input.
    pub fn new(model_id: Uuid, input: serde_json::Value) -> Self {
        Self {
            id: RequestId::new(),
            model_id,
            input,
            parameters: HashMap::new(),
            timestamp: Utc::now(),
        }
    }

    /// Returns the request's unique identifier.
    pub fn id(&self) -> RequestId {
        self.id
    }

    /// Returns the target model ID.
    pub fn model_id(&self) -> Uuid {
        self.model_id
    }

    /// Returns a reference to the input data.
    pub fn input(&self) -> &serde_json::Value {
        &self.input
    }

    /// Returns a reference to the inference parameters.
    pub fn parameters(&self) -> &HashMap<String, serde_json::Value> {
        &self.parameters
    }

    /// Returns the timestamp of when the request was created.
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    /// Sets a parameter value on the request.
    pub fn set_parameter(&mut self, key: String, value: serde_json::Value) {
        self.parameters.insert(key, value);
    }
}
