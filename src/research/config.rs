use serde::{Deserialize, Serialize};

/// Configuration for the research subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchConfig {
    pub enabled: bool,
    pub max_concurrent_tasks: usize,
    pub default_search_provider: String,
    pub search_providers: Vec<SearchProviderConfig>,
    pub fetcher: FetcherConfig,
    pub extractor: ExtractorConfig,
    pub validator: ValidatorConfig,
    pub ranking: RankingConfig,
    pub deduplication: DeduplicationConfig,
    pub citation: CitationConfig,
    pub synthesis: SynthesisConfig,
    pub knowledge_update: KnowledgeUpdateConfig,
    pub world_update: WorldUpdateConfig,
    pub memory_update: MemoryUpdateConfig,
    pub workflow: WorkflowConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchProviderConfig {
    pub name: String,
    pub provider_type: SearchProviderType,
    pub enabled: bool,
    pub api_key_env: Option<String>,
    pub base_url: Option<String>,
    pub max_results: usize,
    pub timeout_ms: u64,
    pub rate_limit_per_second: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchProviderType {
    Web,
    LocalDocument,
    KnowledgeGraph,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetcherConfig {
    pub max_concurrent_fetches: usize,
    pub timeout_ms: u64,
    pub max_retries: usize,
    pub retry_backoff_ms: u64,
    pub user_agent: String,
    pub max_response_bytes: usize,
    pub allowed_content_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractorConfig {
    pub extract_entities: bool,
    pub extract_relationships: bool,
    pub extract_events: bool,
    pub extract_dates: bool,
    pub extract_locations: bool,
    pub extract_citations: bool,
    pub extract_facts: bool,
    pub max_entity_length: usize,
    pub max_fact_length: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub min_confidence: f32,
    pub require_provenance: bool,
    pub require_multiple_sources: bool,
    pub min_sources_for_high_confidence: usize,
    pub conflict_retention: bool,
    pub max_source_age_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingConfig {
    pub recency_weight: f32,
    pub source_authority_weight: f32,
    pub confidence_weight: f32,
    pub relevance_weight: f32,
    pub diversity_weight: f32,
    pub min_relevance_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationConfig {
    pub enabled: bool,
    pub similarity_threshold: f32,
    pub strategy: DeduplicationStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeduplicationStrategy {
    Exact,
    Fuzzy,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationConfig {
    pub require_citations: bool,
    pub min_citations_per_claim: usize,
    pub citation_format: CitationFormat,
    pub preserve_url: bool,
    pub preserve_access_date: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationFormat {
    Inline,
    Footnote,
    Academic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisConfig {
    pub merge_strategy: MergeStrategy,
    pub contradiction_resolution: ContradictionResolution,
    pub summary_max_length: usize,
    pub preserve_conflicting_evidence: bool,
    pub min_evidence_for_synthesis: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    WeightedAverage,
    MostConfident,
    MajorityVote,
    SourcePriority,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionResolution {
    KeepBoth,
    PreferConfident,
    PreferRecent,
    PreferAuthoritative,
    FlagForReview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUpdateConfig {
    pub enabled: bool,
    pub require_governance_approval: bool,
    pub min_confidence_to_update: f32,
    pub max_updates_per_task: usize,
    pub auto_merge: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldUpdateConfig {
    pub enabled: bool,
    pub require_governance_approval: bool,
    pub min_confidence_to_update: f32,
    pub max_updates_per_task: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpdateConfig {
    pub enabled: bool,
    pub importance_threshold: f32,
    pub max_memory_items_per_task: usize,
    pub consolidation_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    pub max_pipeline_stages: usize,
    pub stage_timeout_ms: u64,
    pub enable_partial_results: bool,
    pub emit_progress_events: bool,
}

impl Default for ResearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_tasks: 8,
            default_search_provider: "web".to_string(),
            search_providers: vec![SearchProviderConfig {
                name: "web".to_string(),
                provider_type: SearchProviderType::Web,
                enabled: true,
                api_key_env: None,
                base_url: None,
                max_results: 10,
                timeout_ms: 10000,
                rate_limit_per_second: 5,
            }],
            fetcher: FetcherConfig {
                max_concurrent_fetches: 16,
                timeout_ms: 30000,
                max_retries: 3,
                retry_backoff_ms: 1000,
                user_agent: "NeoResearch/0.1.0".to_string(),
                max_response_bytes: 10 * 1024 * 1024,
                allowed_content_types: vec![
                    "text/html".to_string(),
                    "application/json".to_string(),
                    "text/plain".to_string(),
                    "application/xml".to_string(),
                    "text/xml".to_string(),
                    "text/markdown".to_string(),
                    "application/pdf".to_string(),
                ],
            },
            extractor: ExtractorConfig {
                extract_entities: true,
                extract_relationships: true,
                extract_events: true,
                extract_dates: true,
                extract_locations: true,
                extract_citations: true,
                extract_facts: true,
                max_entity_length: 256,
                max_fact_length: 1024,
            },
            validator: ValidatorConfig {
                min_confidence: 0.3,
                require_provenance: true,
                require_multiple_sources: false,
                min_sources_for_high_confidence: 3,
                conflict_retention: true,
                max_source_age_days: 365,
            },
            ranking: RankingConfig {
                recency_weight: 0.2,
                source_authority_weight: 0.25,
                confidence_weight: 0.3,
                relevance_weight: 0.25,
                diversity_weight: 0.1,
                min_relevance_score: 0.1,
            },
            deduplication: DeduplicationConfig {
                enabled: true,
                similarity_threshold: 0.85,
                strategy: DeduplicationStrategy::Fuzzy,
            },
            citation: CitationConfig {
                require_citations: true,
                min_citations_per_claim: 1,
                citation_format: CitationFormat::Inline,
                preserve_url: true,
                preserve_access_date: true,
            },
            synthesis: SynthesisConfig {
                merge_strategy: MergeStrategy::WeightedAverage,
                contradiction_resolution: ContradictionResolution::KeepBoth,
                summary_max_length: 4096,
                preserve_conflicting_evidence: true,
                min_evidence_for_synthesis: 2,
            },
            knowledge_update: KnowledgeUpdateConfig {
                enabled: true,
                require_governance_approval: true,
                min_confidence_to_update: 0.7,
                max_updates_per_task: 50,
                auto_merge: false,
            },
            world_update: WorldUpdateConfig {
                enabled: true,
                require_governance_approval: true,
                min_confidence_to_update: 0.7,
                max_updates_per_task: 20,
            },
            memory_update: MemoryUpdateConfig {
                enabled: true,
                importance_threshold: 0.5,
                max_memory_items_per_task: 100,
                consolidation_delay_ms: 5000,
            },
            workflow: WorkflowConfig {
                max_pipeline_stages: 12,
                stage_timeout_ms: 60000,
                enable_partial_results: true,
                emit_progress_events: true,
            },
        }
    }
}
