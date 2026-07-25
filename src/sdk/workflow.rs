use crate::api::workflow::{WorkflowInfo, WorkflowStatus};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct WorkflowClient {
    base_url: String,
    client: reqwest::Client,
}

impl WorkflowClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn start_workflow(
        &self,
        name: &str,
        parameters: std::collections::HashMap<String, serde_json::Value>,
    ) -> SdkResult<WorkflowInfo> {
        tracing::debug!("SDK: POST {}/workflows (name={})", self.base_url, name);
        let body = serde_json::json!({
            "name": name,
            "parameters": parameters,
        });
        let resp = self
            .client
            .post(format!("{}/workflows", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status(status, &text));
        }
        resp.json().await.map_err(SdkError::Http)
    }

    pub async fn cancel_workflow(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: POST {}/workflows/{}/cancel", self.base_url, id);
        let resp = self
            .client
            .post(format!("{}/workflows/{}/cancel", self.base_url, id))
            .send()
            .await
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status(status, &text));
        }
        Ok(())
    }

    pub async fn get_status(&self, id: &str) -> SdkResult<WorkflowStatus> {
        tracing::debug!("SDK: GET {}/workflows/{}/status", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/workflows/{}/status", self.base_url, id))
            .send()
            .await
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status(status, &text));
        }
        resp.json().await.map_err(SdkError::Http)
    }
}
