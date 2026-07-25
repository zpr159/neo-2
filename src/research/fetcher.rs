use std::sync::Arc;

use tokio::sync::Semaphore;

use super::api::{FetchedContent, SearchResult};
use super::config::FetcherConfig;
use super::error::{ResearchError, ResearchResult};
use crate::time::Timestamp;

/// Asynchronous content fetcher supporting multiple content types.
pub struct ContentFetcher {
    client: reqwest::Client,
    config: FetcherConfig,
    semaphore: Arc<Semaphore>,
}

impl ContentFetcher {
    pub fn new(config: FetcherConfig) -> ResearchResult<Self> {
        let max_concurrent = config.max_concurrent_fetches;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .user_agent(&config.user_agent)
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .map_err(|e| ResearchError::FetchFailed(e.to_string()))?;

        Ok(Self {
            client,
            config,
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }

    /// Fetch content from a single search result.
    pub async fn fetch(&self, result: &SearchResult) -> ResearchResult<FetchedContent> {
        let _permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| {
                ResearchError::PipelineError("fetch semaphore closed".to_string())
            })?;

        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let backoff = self.config.retry_backoff_ms * attempt as u64;
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
            }

            match self.fetch_single(&result.url).await {
                Ok(content) => return Ok(content),
                Err(e) => {
                    if !e.is_retriable() {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ResearchError::FetchFailed("max retries exceeded".to_string())
        }))
    }

    /// Fetch content from multiple search results concurrently.
    pub async fn fetch_many(
        &self,
        results: &[SearchResult],
    ) -> Vec<Result<FetchedContent, ResearchError>> {
        let mut handles = Vec::with_capacity(results.len());

        for result in results {
            let result = result.clone();
            let client = self.client.clone();
            let config = self.config.clone();
            let semaphore = Arc::clone(&self.semaphore);

            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await.map_err(|_| {
                    ResearchError::PipelineError("fetch semaphore closed".to_string())
                })?;

                fetch_with_client(&client, &result.url, &config).await
            }));
        }

        let mut outputs = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok(result) => outputs.push(result),
                Err(e) => {
                    outputs.push(Err(ResearchError::FetchFailed(format!(
                        "task join error: {}",
                        e
                    ))));
                }
            }
        }

        outputs
    }

    async fn fetch_single(&self, url: &str) -> ResearchResult<FetchedContent> {
        fetch_with_client(&self.client, url, &self.config).await
    }
}

async fn fetch_with_client(
    client: &reqwest::Client,
    url: &str,
    config: &FetcherConfig,
) -> ResearchResult<FetchedContent> {
    let response = client
        .get(url)
        .header("Accept", config.allowed_content_types.join(", "))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                ResearchError::ProviderTimeout(e.to_string())
            } else if e.is_connect() {
                ResearchError::ProviderUnavailable(e.to_string())
            } else {
                ResearchError::FetchFailed(e.to_string())
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(ResearchError::FetchFailed(format!(
            "HTTP {} for {}",
            status, url
        )));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/plain")
        .to_string();

    if !is_allowed_content_type(&content_type, &config.allowed_content_types) {
        return Err(ResearchError::UnsupportedContentType(format!(
            "{} is not in allowed types",
            content_type
        )));
    }

    let raw_content = response
        .bytes()
        .await
        .map_err(|e| ResearchError::FetchFailed(e.to_string()))?;

    let size_bytes = raw_content.len();
    if size_bytes > config.max_response_bytes {
        return Err(ResearchError::FetchFailed(format!(
            "response too large: {} bytes (max {})",
            size_bytes, config.max_response_bytes
        )));
    }

    let text_content = if content_type.contains("json") {
        let json_val: serde_json::Value = serde_json::from_slice(&raw_content)
            .map_err(|e| ResearchError::SerializationFailed(e.to_string()))?;
        serde_json::to_string_pretty(&json_val)
            .map_err(|e| ResearchError::SerializationFailed(e.to_string()))?
    } else if content_type.contains("html") {
        extract_text_from_html(&raw_content)
    } else if content_type.contains("xml") {
        extract_text_from_xml(&raw_content)
    } else {
        String::from_utf8_lossy(&raw_content).to_string()
    };

    Ok(FetchedContent {
        url: url.to_string(),
        content_type,
        raw_content: raw_content.to_vec(),
        text_content,
        metadata: std::collections::HashMap::new(),
        fetched_at: Timestamp::now(),
        size_bytes,
    })
}

fn is_allowed_content_type(content_type: &str, allowed: &[String]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    let ct_lower = content_type.to_lowercase();
    allowed.iter().any(|a| ct_lower.contains(&a.to_lowercase()))
}

fn extract_text_from_html(raw: &[u8]) -> String {
    let html = String::from_utf8_lossy(raw);

    let mut text = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if in_tag => {
                let tag_start = text.len().saturating_sub(20);
                let recent: String = text.chars().skip(tag_start).collect();
                let recent_lower = recent.to_lowercase();
                if recent_lower.ends_with("script") {
                    in_script = true;
                } else if recent_lower.ends_with("style") {
                    in_style = true;
                } else if recent_lower.ends_with("/script") {
                    in_script = false;
                } else if recent_lower.ends_with("/style") {
                    in_style = false;
                }
            }
            _ if in_script || in_style => {}
            '\n' | '\r' | '\t' => {
                text.push(' ');
            }
            _ => {
                text.push(ch);
            }
        }
    }

    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn extract_text_from_xml(raw: &[u8]) -> String {
    let xml = String::from_utf8_lossy(raw);
    let mut text = String::new();
    let mut in_tag = false;

    for ch in xml.chars() {
        match ch {
            '<' => {
                in_tag = true;
                text.push(' ');
            }
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => {
                text.push(ch);
            }
            _ => {}
        }
    }

    text.split_whitespace().collect::<Vec<&str>>().join(" ")
}
