//! REST API types for tool management.

use serde::{Deserialize, Serialize};

use crate::analytics::AggregateAnalytics;
use crate::health::HealthSummary;
use crate::lifecycle::ToolLifecycleState;
use crate::types::{ToolConfiguration, ToolManifest, ToolMetrics, ToolVersion};

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request to register a new tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterToolRequest {
    pub manifest: ToolManifest,
}

/// Request to execute a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteToolRequest {
    pub operation: String,
    pub parameters: serde_json::Value,
    pub timeout_ms: Option<u64>,
}

/// Request to update tool configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfigRequest {
    pub config: ToolConfiguration,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Generic API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            request_id: uuid::Uuid::new_v4().to_string(),
        }
    }
}

/// Tool list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListResponse {
    pub tools: Vec<ToolSummary>,
    pub total: usize,
}

/// Summary of a tool for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSummary {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: ToolVersion,
    pub tool_type: String,
    pub category: String,
    pub state: ToolLifecycleState,
    pub enabled: bool,
}

impl ToolSummary {
    pub fn from_manifest(manifest: &ToolManifest, state: ToolLifecycleState) -> Self {
        Self {
            name: manifest.metadata.name.clone(),
            display_name: manifest.metadata.display_name.clone(),
            description: manifest.metadata.description.clone(),
            version: manifest.metadata.version.clone(),
            tool_type: format!("{:?}", manifest.metadata.tool_type),
            category: format!("{:?}", manifest.metadata.category),
            state,
            enabled: manifest.config.enabled,
        }
    }
}

/// Tool detail response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDetailResponse {
    pub manifest: ToolManifest,
    pub state: ToolLifecycleState,
    pub metrics: ToolMetrics,
}

/// Search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<ToolSummary>,
    pub count: usize,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub summary: HealthSummary,
    pub tools: Vec<crate::types::ToolHealth>,
}

/// Analytics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
    pub aggregate: AggregateAnalytics,
}

/// Metrics response for a single tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub tool_name: String,
    pub metrics: ToolMetrics,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() {
        let resp = ApiResponse::ok("data");
        assert!(resp.success);
        assert!(resp.data.is_some());

        let err: ApiResponse<String> = ApiResponse::err("bad request");
        assert!(!err.success);
        assert!(err.error.is_some());
    }
}
