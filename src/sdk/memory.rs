use crate::api::memory::{MemorySearchRequest, MemorySearchResult, MemoryStatistics};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct MemoryClient {
    base_url: String,
    client: reqwest::Client,
}

impl MemoryClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn search(&self, query: &str, limit: usize) -> SdkResult<Vec<MemorySearchResult>> {
        tracing::debug!("SDK: POST {}/memory/search (query={})", self.base_url, query);
        let body = MemorySearchRequest {
            query: query.to_string(),
            memory_type: None,
            limit,
            min_relevance: 0.0,
        };
        let resp = self
            .client
            .post(format!("{}/memory/search", self.base_url))
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

    pub async fn store(
        &self,
        content: &str,
        memory_type: &str,
    ) -> SdkResult<String> {
        tracing::debug!("SDK: POST {}/memory/store (type={})", self.base_url, memory_type);
        let body = crate::api::memory::MemoryStoreRequest {
            content: content.to_string(),
            memory_type: memory_type.to_string(),
            metadata: std::collections::HashMap::new(),
            importance: 1.0,
        };
        let resp = self
            .client
            .post(format!("{}/memory/store", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(map_status(status, &text));
        }
        resp.text().await.map_err(SdkError::Http)
    }

    pub async fn delete(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: DELETE {}/memory/{}", self.base_url, id);
        let resp = self
            .client
            .delete(format!("{}/memory/{}", self.base_url, id))
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

    pub async fn statistics(&self) -> SdkResult<MemoryStatistics> {
        tracing::debug!("SDK: GET {}/memory/statistics", self.base_url);
        let resp = self
            .client
            .get(format!("{}/memory/statistics", self.base_url))
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
