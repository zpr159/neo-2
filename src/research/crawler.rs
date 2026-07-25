use std::sync::Arc;

use super::api::{SearchQuery, SearchResult};
use super::config::SearchProviderConfig;
use super::error::ResearchResult;
use super::search::{SearchProvider, SearchProviderRegistry};

/// Coordinates search execution across multiple providers.
pub struct ResearchCrawler {
    registry: SearchProviderRegistry,
}

impl ResearchCrawler {
    pub fn new(_max_concurrent: usize) -> Self {
        Self {
            registry: SearchProviderRegistry::new(),
        }
    }

    pub fn register_provider(&mut self, provider: Arc<dyn SearchProvider>) {
        self.registry.register(provider);
    }

    pub fn has_provider(&self, name: &str) -> bool {
        self.registry.has_provider(name)
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.registry.list_providers()
    }

    /// Execute a search across specified providers and merge results.
    pub async fn search(
        &self,
        query: &SearchQuery,
        provider_names: &[String],
    ) -> ResearchResult<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        let mut handles = Vec::new();

        for provider_name in provider_names {
            if let Some(provider) = self.registry.get(provider_name) {
                let provider = Arc::clone(provider);
                let query = query.clone();

                handles.push(tokio::spawn(async move {
                    provider.search(&query).await
                }));
            }
        }

        for handle in handles {
            match handle.await {
                Ok(Ok(results)) => all_results.extend(results),
                Ok(Err(e)) => {
                    if !e.is_retriable() {
                        tracing::warn!("search provider error (non-retriable): {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("search task join error: {}", e);
                }
            }
        }

        all_results.sort_by(|a, b| {
            b.estimated_relevance
                .partial_cmp(&a.estimated_relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let max = query.max_results;
        all_results.truncate(max);

        Ok(all_results)
    }

    /// Build search queries from a research objective.
    pub fn build_queries(
        &self,
        objective: &str,
        provider_configs: &[SearchProviderConfig],
        max_results: usize,
    ) -> Vec<(String, SearchQuery)> {
        provider_configs
            .iter()
            .filter(|c| c.enabled)
            .map(|config| {
                let search_terms = extract_search_terms(objective);
                let query = SearchQuery {
                    original_objective: objective.to_string(),
                    search_terms: search_terms.clone(),
                    provider: config.name.clone(),
                    max_results: max_results.min(config.max_results),
                    content_type_filter: None,
                    time_range: None,
                };
                (config.name.clone(), query)
            })
            .collect()
    }
}

fn extract_search_terms(objective: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "shall", "can", "need", "dare", "ought",
        "used", "to", "of", "in", "for", "on", "with", "at", "by", "from",
        "as", "into", "through", "during", "before", "after", "above", "below",
        "between", "out", "off", "over", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all", "each",
        "every", "both", "few", "more", "most", "other", "some", "such", "no",
        "nor", "not", "only", "own", "same", "so", "than", "too", "very",
        "just", "because", "but", "and", "or", "if", "while", "what", "which",
        "who", "whom", "this", "that", "these", "those", "i", "me", "my",
        "we", "our", "you", "your", "he", "him", "his", "she", "her", "it",
        "its", "they", "them", "their",
    ]
    .iter()
    .cloned()
    .collect();

    objective
        .split_whitespace()
        .filter(|word| !stop_words.contains(word.to_lowercase().as_str()))
        .map(|word| word.to_string())
        .collect()
}

/// Initialize a crawler with providers from configuration.
pub fn initialize_crawler(
    provider_configs: &[SearchProviderConfig],
    max_concurrent: usize,
) -> ResearchCrawler {
    let mut crawler = ResearchCrawler::new(max_concurrent);

    for config in provider_configs {
        if !config.enabled {
            continue;
        }
        let provider: Arc<dyn SearchProvider> = match config.provider_type {
            super::config::SearchProviderType::Web => {
                let api_key = config
                    .api_key_env
                    .as_ref()
                    .and_then(|env_var| std::env::var(env_var).ok());

                Arc::new(super::search::HttpSearchProvider::new(
                    config.name.clone(),
                    config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "https://api.search.neo.local/v1".to_string()),
                    api_key,
                    config.max_results,
                    config.timeout_ms,
                ))
            }
            super::config::SearchProviderType::LocalDocument => Arc::new(
                super::search::LocalDocumentSearchProvider::new(
                    config.name.clone(),
                    config
                        .base_url
                        .clone()
                        .unwrap_or_else(|| "./data/documents".to_string()),
                    config.max_results,
                ),
            ),
            super::config::SearchProviderType::KnowledgeGraph => {
                Arc::new(super::search::KnowledgeGraphSearchProvider::new(
                    config.name.clone(),
                ))
            }
            super::config::SearchProviderType::Plugin => {
                continue;
            }
        };
        crawler.register_provider(provider);
    }

    crawler
}
