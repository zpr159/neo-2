use crate::api::world_model::{
    WorldEntity, WorldEvent, WorldSnapshot, SimulationRequest, SimulationResult,
    PredictionRequest, PredictionResult,
};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct WorldClient {
    base_url: String,
    client: reqwest::Client,
}

impl WorldClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn list_entities(&self) -> SdkResult<Vec<WorldEntity>> {
        tracing::debug!("SDK: GET {}/world/entities", self.base_url);
        let resp = self
            .client
            .get(format!("{}/world/entities", self.base_url))
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

    pub async fn get_entity(&self, id: &str) -> SdkResult<WorldEntity> {
        tracing::debug!("SDK: GET {}/world/entities/{}", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/world/entities/{}", self.base_url, id))
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

    pub async fn create_entity(&self, entity: &WorldEntity) -> SdkResult<WorldEntity> {
        tracing::debug!("SDK: POST {}/world/entities", self.base_url);
        let resp = self
            .client
            .post(format!("{}/world/entities", self.base_url))
            .json(entity)
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

    pub async fn update_entity(&self, id: &str, entity: &WorldEntity) -> SdkResult<WorldEntity> {
        tracing::debug!("SDK: PUT {}/world/entities/{}", self.base_url, id);
        let resp = self
            .client
            .put(format!("{}/world/entities/{}", self.base_url, id))
            .json(entity)
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

    pub async fn delete_entity(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: DELETE {}/world/entities/{}", self.base_url, id);
        let resp = self
            .client
            .delete(format!("{}/world/entities/{}", self.base_url, id))
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

    pub async fn list_events(&self) -> SdkResult<Vec<WorldEvent>> {
        tracing::debug!("SDK: GET {}/world/events", self.base_url);
        let resp = self
            .client
            .get(format!("{}/world/events", self.base_url))
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

    pub async fn get_snapshot(&self) -> SdkResult<WorldSnapshot> {
        tracing::debug!("SDK: GET {}/world/snapshot", self.base_url);
        let resp = self
            .client
            .get(format!("{}/world/snapshot", self.base_url))
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

    pub async fn simulate(&self, request: &SimulationRequest) -> SdkResult<SimulationResult> {
        tracing::debug!("SDK: POST {}/world/simulate", self.base_url);
        let resp = self
            .client
            .post(format!("{}/world/simulate", self.base_url))
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

    pub async fn predict(&self, request: &PredictionRequest) -> SdkResult<PredictionResult> {
        tracing::debug!("SDK: POST {}/world/predict", self.base_url);
        let resp = self
            .client
            .post(format!("{}/world/predict", self.base_url))
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
