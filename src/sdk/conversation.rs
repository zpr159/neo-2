use crate::api::conversation::{ChatResponse, ChatRequest, CreateSessionRequest, SessionInfo, HistoryEntry};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct ConversationClient {
    base_url: String,
    client: reqwest::Client,
}

impl ConversationClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn chat(&self, message: &str) -> SdkResult<ChatResponse> {
        tracing::debug!("SDK: POST {}/conversations/chat", self.base_url);
        let body = ChatRequest {
            session_id: None,
            conversation_id: None,
            message: message.to_string(),
            stream: false,
            metadata: std::collections::HashMap::new(),
        };
        let resp = self
            .client
            .post(format!("{}/conversations/chat", self.base_url))
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

    pub async fn stream(&self, message: &str) -> SdkResult<reqwest::Response> {
        tracing::debug!("SDK: POST {}/conversations/chat (stream)", self.base_url);
        let body = ChatRequest {
            session_id: None,
            conversation_id: None,
            message: message.to_string(),
            stream: true,
            metadata: std::collections::HashMap::new(),
        };
        self.client
            .post(format!("{}/conversations/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| SdkError::ConnectionFailed(e.to_string()))
    }

    pub async fn create_session(&self) -> SdkResult<SessionInfo> {
        tracing::debug!("SDK: POST {}/conversations/sessions", self.base_url);
        let body = CreateSessionRequest { user_id: None };
        let resp = self
            .client
            .post(format!("{}/conversations/sessions", self.base_url))
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

    pub async fn get_session(&self, id: &str) -> SdkResult<SessionInfo> {
        tracing::debug!("SDK: GET {}/conversations/sessions/{}", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/conversations/sessions/{}", self.base_url, id))
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

    pub async fn delete_session(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: DELETE {}/conversations/sessions/{}", self.base_url, id);
        let resp = self
            .client
            .delete(format!("{}/conversations/sessions/{}", self.base_url, id))
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

    pub async fn get_history(&self, conversation_id: &str) -> SdkResult<Vec<HistoryEntry>> {
        tracing::debug!("SDK: GET {}/conversations/{}/history", self.base_url, conversation_id);
        let resp = self
            .client
            .get(format!("{}/conversations/{}/history", self.base_url, conversation_id))
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
