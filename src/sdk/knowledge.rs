use crate::api::knowledge::{
    KnowledgeEntity, KnowledgeGraph, KnowledgeQueryRequest, KnowledgeSearchResult,
};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct KnowledgeClient {
    base_url: String,
    client: reqwest::Client,
}

impl KnowledgeClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn get_entity(&self, id: &str) -> SdkResult<KnowledgeEntity> {
        tracing::debug!("SDK: GET {}/knowledge/entities/{}", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/knowledge/entities/{}", self.base_url, id))
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

    pub async fn search(&self, query: &str, limit: usize) -> SdkResult<KnowledgeSearchResult> {
        tracing::debug!("SDK: GET {}/knowledge/search?q={}&limit={}", self.base_url, query, limit);
        let resp = self
            .client
            .get(format!("{}/knowledge/search", self.base_url))
            .query(&[("q", query), ("limit", &limit.to_string())])
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

    pub async fn get_graph(&self, entity_id: &str, depth: usize) -> SdkResult<KnowledgeGraph> {
        tracing::debug!("SDK: GET {}/knowledge/graph/{}?depth={}", self.base_url, entity_id, depth);
        let resp = self
            .client
            .get(format!("{}/knowledge/graph/{}", self.base_url, entity_id))
            .query(&[("depth", &depth.to_string())])
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

    pub async fn query(&self, request: &KnowledgeQueryRequest) -> SdkResult<KnowledgeSearchResult> {
        tracing::debug!("SDK: POST {}/knowledge/query", self.base_url);
        let resp = self
            .client
            .post(format!("{}/knowledge/query", self.base_url))
            .json(request)
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
