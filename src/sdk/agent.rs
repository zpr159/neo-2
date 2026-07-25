use crate::api::agent::{AgentInfo, AgentStatusDetail};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct AgentClient {
    base_url: String,
    client: reqwest::Client,
}

impl AgentClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn list_agents(&self) -> SdkResult<Vec<AgentInfo>> {
        tracing::debug!("SDK: GET {}/agents", self.base_url);
        let resp = self
            .client
            .get(format!("{}/agents", self.base_url))
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

    pub async fn start_agent(&self, id: &str) -> SdkResult<AgentInfo> {
        tracing::debug!("SDK: POST {}/agents/{}/start", self.base_url, id);
        let resp = self
            .client
            .post(format!("{}/agents/{}/start", self.base_url, id))
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

    pub async fn stop_agent(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: POST {}/agents/{}/stop", self.base_url, id);
        let resp = self
            .client
            .post(format!("{}/agents/{}/stop", self.base_url, id))
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

    pub async fn get_status(&self, id: &str) -> SdkResult<AgentStatusDetail> {
        tracing::debug!("SDK: GET {}/agents/{}/status", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/agents/{}/status", self.base_url, id))
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
