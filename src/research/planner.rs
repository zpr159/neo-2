use super::api::{ResearchRequest, SearchQuery};
use super::config::SearchProviderConfig;
use super::error::ResearchResult;

/// A plan for executing a research task.
#[derive(Debug, Clone)]
pub struct ResearchPlan {
    pub objective: String,
    pub search_queries: Vec<SearchQuery>,
    pub search_providers: Vec<String>,
    pub max_sources: usize,
    pub require_citations: bool,
    pub stages: Vec<ResearchStage>,
    pub estimated_duration_ms: u64,
}

/// A single stage in the research pipeline.
#[derive(Debug, Clone)]
pub struct ResearchStage {
    pub name: String,
    pub description: String,
    pub estimated_duration_ms: u64,
    pub can_skip: bool,
}

/// Plans research execution based on a request.
pub struct ResearchPlanner {
    provider_configs: Vec<SearchProviderConfig>,
}

impl ResearchPlanner {
    pub fn new(provider_configs: Vec<SearchProviderConfig>) -> Self {
        Self { provider_configs }
    }

    /// Create a research plan from a request.
    pub fn plan(&self, request: &ResearchRequest) -> ResearchResult<ResearchPlan> {
        let search_queries = self.build_search_queries(request)?;
        let stages = self.build_stages(request);

        let estimated_duration_ms: u64 = stages.iter().map(|s| s.estimated_duration_ms).sum();

        Ok(ResearchPlan {
            objective: request.objective.clone(),
            search_queries,
            search_providers: request.search_providers.clone(),
            max_sources: request.max_sources,
            require_citations: request.require_citations,
            stages,
            estimated_duration_ms,
        })
    }

    /// Build search queries for the given request.
    fn build_search_queries(
        &self,
        request: &ResearchRequest,
    ) -> ResearchResult<Vec<SearchQuery>> {
        let search_terms = extract_key_terms(&request.objective);

        let queries: Vec<SearchQuery> = request
            .search_providers
            .iter()
            .map(|provider_name| {
                SearchQuery {
                    original_objective: request.objective.clone(),
                    search_terms: search_terms.clone(),
                    provider: provider_name.clone(),
                    max_results: request.max_sources,
                    content_type_filter: None,
                    time_range: None,
                }
            })
            .collect();

        Ok(queries)
    }

    /// Build pipeline stages for the research task.
    fn build_stages(&self, request: &ResearchRequest) -> Vec<ResearchStage> {
        let mut stages = vec![
            ResearchStage {
                name: "search".to_string(),
                description: "Execute search queries across providers".to_string(),
                estimated_duration_ms: 5000,
                can_skip: false,
            },
            ResearchStage {
                name: "fetch".to_string(),
                description: "Fetch content from search results".to_string(),
                estimated_duration_ms: 15000,
                can_skip: false,
            },
            ResearchStage {
                name: "extract".to_string(),
                description: "Extract structured information from content".to_string(),
                estimated_duration_ms: 10000,
                can_skip: false,
            },
            ResearchStage {
                name: "validate".to_string(),
                description: "Validate extracted facts against sources".to_string(),
                estimated_duration_ms: 5000,
                can_skip: false,
            },
            ResearchStage {
                name: "rank".to_string(),
                description: "Rank findings by composite score".to_string(),
                estimated_duration_ms: 1000,
                can_skip: false,
            },
            ResearchStage {
                name: "synthesize".to_string(),
                description: "Synthesize findings into structured report".to_string(),
                estimated_duration_ms: 5000,
                can_skip: false,
            },
        ];

        if request.update_knowledge || request.update_world_model || request.update_memory {
            stages.push(ResearchStage {
                name: "update_knowledge".to_string(),
                description: "Update knowledge graph with validated findings".to_string(),
                estimated_duration_ms: 3000,
                can_skip: true,
            });
        }

        if request.update_world_model {
            stages.push(ResearchStage {
                name: "update_world".to_string(),
                description: "Update world model with new information".to_string(),
                estimated_duration_ms: 2000,
                can_skip: true,
            });
        }

        if request.update_memory {
            stages.push(ResearchStage {
                name: "update_memory".to_string(),
                description: "Store research findings in memory".to_string(),
                estimated_duration_ms: 2000,
                can_skip: true,
            });
        }

        stages
    }
}

fn extract_key_terms(objective: &str) -> Vec<String> {
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
        "its", "they", "them", "their", "about", "research", "find", "information",
        "tell", "know", "explain", "describe",
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
