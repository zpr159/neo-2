use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::api::{SearchQuery, SearchResult};
use super::error::ResearchResult;

/// Capabilities of a search provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchCapabilities {
    pub supports_web_search: bool,
    pub supports_local_document_search: bool,
    pub supports_knowledge_search: bool,
    pub supports_time_range: bool,
    pub supports_content_type_filter: bool,
    pub max_results_per_query: usize,
    pub rate_limit_per_second: u32,
}

/// A search provider that can execute queries against a specific data source.
#[async_trait]
pub trait SearchProvider: Send + Sync {
    /// Returns the unique name of this provider.
    fn name(&self) -> &str;

    /// Returns the capabilities of this provider.
    fn capabilities(&self) -> SearchCapabilities;

    /// Execute a search query and return results.
    async fn search(&self, query: &SearchQuery) -> ResearchResult<Vec<SearchResult>>;

    /// Health check for the provider.
    async fn health_check(&self) -> ResearchResult<bool>;
}

/// Registry of available search providers.
pub struct SearchProviderRegistry {
    providers: std::collections::HashMap<String, std::sync::Arc<dyn SearchProvider>>,
}

impl SearchProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: std::sync::Arc<dyn SearchProvider>) {
        self.providers
            .insert(provider.name().to_string(), provider);
    }

    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<dyn SearchProvider>> {
        self.providers.get(name)
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
    }

    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

impl Default for SearchProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A built-in web search provider using configurable HTTP endpoints.
pub struct HttpSearchProvider {
    name: String,
    base_url: String,
    api_key: Option<String>,
    max_results: usize,
    timeout_ms: u64,
}

impl HttpSearchProvider {
    pub fn new(
        name: String,
        base_url: String,
        api_key: Option<String>,
        max_results: usize,
        timeout_ms: u64,
    ) -> Self {
        Self {
            name,
            base_url,
            api_key,
            max_results,
            timeout_ms,
        }
    }
}

#[async_trait]
impl SearchProvider for HttpSearchProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_web_search: true,
            supports_local_document_search: false,
            supports_knowledge_search: false,
            supports_time_range: true,
            supports_content_type_filter: false,
            max_results_per_query: self.max_results,
            rate_limit_per_second: 5,
        }
    }

    async fn search(&self, query: &SearchQuery) -> ResearchResult<Vec<SearchResult>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build()
            .map_err(|e| super::error::ResearchError::FetchFailed(e.to_string()))?;

        let mut request = client.get(&self.base_url).query(&[
            ("q", query.search_terms.join(" ")),
            ("limit", query.max_results.to_string()),
        ]);

        if let Some(ref key) = self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                super::error::ResearchError::ProviderTimeout(e.to_string())
            } else {
                super::error::ResearchError::SearchFailed(e.to_string())
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(super::error::ResearchError::SearchFailed(format!(
                "HTTP {} from {}",
                status, self.name
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            super::error::ResearchError::SerializationFailed(e.to_string())
        })?;

        let mut results = Vec::new();
        if let Some(items) = body.get("results").and_then(|v| v.as_array()) {
            for (i, item) in items.iter().enumerate() {
                results.push(SearchResult {
                    url: item
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    title: item
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    snippet: item
                        .get("snippet")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    provider: self.name.clone(),
                    rank: i + 1,
                    estimated_relevance: 1.0 - (i as f32 * 0.1),
                });
            }
        }

        Ok(results)
    }

    async fn health_check(&self) -> ResearchResult<bool> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| super::error::ResearchError::FetchFailed(e.to_string()))?;

        let resp = client
            .get(&self.base_url)
            .send()
            .await;
        Ok(resp.is_ok())
    }
}

/// A local document search provider for searching indexed local files.
pub struct LocalDocumentSearchProvider {
    name: String,
    index_path: String,
    max_results: usize,
}

impl LocalDocumentSearchProvider {
    pub fn new(name: String, index_path: String, max_results: usize) -> Self {
        Self {
            name,
            index_path,
            max_results,
        }
    }
}

#[async_trait]
impl SearchProvider for LocalDocumentSearchProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_web_search: false,
            supports_local_document_search: true,
            supports_knowledge_search: false,
            supports_time_range: false,
            supports_content_type_filter: false,
            max_results_per_query: self.max_results,
            rate_limit_per_second: 100,
        }
    }

    async fn search(&self, query: &SearchQuery) -> ResearchResult<Vec<SearchResult>> {
        let path = std::path::Path::new(&self.index_path);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let search_lower = query.search_terms.join(" ").to_lowercase();

        if path.is_dir() {
            let entries = std::fs::read_dir(path)
                .map_err(|e| super::error::ResearchError::SearchFailed(e.to_string()))?;

            for entry in entries.flatten() {
                if results.len() >= self.max_results {
                    break;
                }
                let file_path = entry.path();
                if file_path.is_file() {
                    let content = std::fs::read_to_string(&file_path)
                        .unwrap_or_default()
                        .to_lowercase();
                    if content.contains(&search_lower) {
                        results.push(SearchResult {
                            url: file_path.to_string_lossy().to_string(),
                            title: file_path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default(),
                            snippet: content.chars().take(200).collect(),
                            provider: self.name.clone(),
                            rank: results.len() + 1,
                            estimated_relevance: 0.5,
                        });
                    }
                }
            }
        }

        Ok(results)
    }

    async fn health_check(&self) -> ResearchResult<bool> {
        Ok(std::path::Path::new(&self.index_path).exists())
    }
}

/// A knowledge graph search provider for searching existing Neo knowledge.
pub struct KnowledgeGraphSearchProvider {
    name: String,
}

impl KnowledgeGraphSearchProvider {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl SearchProvider for KnowledgeGraphSearchProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> SearchCapabilities {
        SearchCapabilities {
            supports_web_search: false,
            supports_local_document_search: false,
            supports_knowledge_search: true,
            supports_time_range: false,
            supports_content_type_filter: false,
            max_results_per_query: 100,
            rate_limit_per_second: 50,
        }
    }

    async fn search(&self, query: &SearchQuery) -> ResearchResult<Vec<SearchResult>> {
        let _ = query;
        Ok(Vec::new())
    }

    async fn health_check(&self) -> ResearchResult<bool> {
        Ok(true)
    }
}
