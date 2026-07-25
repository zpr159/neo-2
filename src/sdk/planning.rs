use crate::api::planning::{CreatePlanRequest, Plan};
use super::error::{SdkError, SdkResult, map_status};

#[derive(Clone)]
pub struct PlanningClient {
    base_url: String,
    client: reqwest::Client,
}

impl PlanningClient {
    pub(crate) fn new(base_url: String, client: reqwest::Client) -> Self {
        Self { base_url, client }
    }

    pub async fn create_plan(&self, goal: &str, constraints: Vec<String>) -> SdkResult<Plan> {
        tracing::debug!("SDK: POST {}/planning/plans (goal={})", self.base_url, goal);
        let body = CreatePlanRequest {
            goal: goal.to_string(),
            constraints,
            max_depth: None,
        };
        let resp = self
            .client
            .post(format!("{}/planning/plans", self.base_url))
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

    pub async fn get_plan(&self, id: &str) -> SdkResult<Plan> {
        tracing::debug!("SDK: GET {}/planning/plans/{}", self.base_url, id);
        let resp = self
            .client
            .get(format!("{}/planning/plans/{}", self.base_url, id))
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

    pub async fn delete_plan(&self, id: &str) -> SdkResult<()> {
        tracing::debug!("SDK: DELETE {}/planning/plans/{}", self.base_url, id);
        let resp = self
            .client
            .delete(format!("{}/planning/plans/{}", self.base_url, id))
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
}
