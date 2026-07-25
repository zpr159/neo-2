use serde::{Deserialize, Serialize};

use crate::time::Timestamp;

use super::config::CitationFormat;

/// Unique identifier for a research task.
pub type ResearchTaskId = uuid::Uuid;

/// Status of a research task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchTaskStatus {
    Created,
    Planning,
    Searching,
    Fetching,
    Extracting,
    Validating,
    Ranking,
    Synthesizing,
    UpdatingKnowledge,
    UpdatingWorld,
    UpdatingMemory,
    Completed,
    Failed,
    Cancelled,
}

/// Priority level for research tasks.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// A request to perform research on a given objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRequest {
    pub objective: String,
    pub priority: ResearchPriority,
    pub max_sources: usize,
    pub search_providers: Vec<String>,
    pub require_citations: bool,
    pub update_knowledge: bool,
    pub update_world_model: bool,
    pub update_memory: bool,
    pub timeout_secs: Option<u64>,
    pub context: Option<String>,
    pub tags: Vec<String>,
}

impl Default for ResearchRequest {
    fn default() -> Self {
        Self {
            objective: String::new(),
            priority: ResearchPriority::Normal,
            max_sources: 10,
            search_providers: vec!["web".to_string()],
            require_citations: true,
            update_knowledge: true,
            update_world_model: true,
            update_memory: true,
            timeout_secs: None,
            context: None,
            tags: Vec::new(),
        }
    }
}

/// A research task in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchTask {
    pub id: ResearchTaskId,
    pub request: ResearchRequest,
    pub status: ResearchTaskStatus,
    pub created_at: Timestamp,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub progress: f32,
    pub current_stage: Option<String>,
    pub result: Option<ResearchOutput>,
    pub error: Option<String>,
    pub metrics: ResearchTaskMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResearchTaskMetrics {
    pub sources_searched: usize,
    pub sources_fetched: usize,
    pub sources_failed: usize,
    pub facts_extracted: usize,
    pub facts_validated: usize,
    pub facts_rejected: usize,
    pub duplicates_removed: usize,
    pub citations_generated: usize,
    pub knowledge_updates_proposed: usize,
    pub knowledge_updates_approved: usize,
    pub world_updates_proposed: usize,
    pub memory_updates_proposed: usize,
    pub total_duration_ms: u64,
    pub stage_durations_ms: std::collections::HashMap<String, u64>,
}

/// The final result of a research task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchOutput {
    pub summary: String,
    pub findings: Vec<Finding>,
    pub citations: Vec<Citation>,
    pub contradictions: Vec<ResearchContradiction>,
    pub knowledge_updates: Vec<KnowledgeUpdateProposal>,
    pub world_updates: Vec<WorldUpdateProposal>,
    pub memory_updates: Vec<MemoryUpdateProposal>,
    pub confidence: f32,
    pub sources_count: usize,
    pub evidence_count: usize,
}

/// A single validated finding from research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: uuid::Uuid,
    pub statement: String,
    pub confidence: f32,
    pub supporting_citations: Vec<uuid::Uuid>,
    pub evidence: Vec<ResearchEvidence>,
    pub provenance: ResearchProvenance,
    pub timestamp: Timestamp,
}

/// Evidence gathered during research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchEvidence {
    pub id: uuid::Uuid,
    pub content: String,
    pub source_url: Option<String>,
    pub source_name: String,
    pub content_type: String,
    pub confidence: f32,
    pub extracted_at: Timestamp,
    pub relevance_score: f32,
}

/// Provenance chain for research findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProvenance {
    pub chain: Vec<ProvenanceEntry>,
    pub root_source: String,
    pub derivation_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    pub source: String,
    pub operation: String,
    pub timestamp: Timestamp,
    pub confidence: f32,
}

/// A citation pointing to a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub id: uuid::Uuid,
    pub source_url: Option<String>,
    pub source_name: String,
    pub title: Option<String>,
    pub access_date: Timestamp,
    pub snippet: Option<String>,
    pub reliability_score: f32,
    pub citation_format: CitationFormat,
}

/// A contradiction detected between findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContradiction {
    pub finding_a_id: uuid::Uuid,
    pub finding_b_id: uuid::Uuid,
    pub statement_a: String,
    pub statement_b: String,
    pub severity: ContradictionSeverity,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionSeverity {
    Minor,
    Moderate,
    Critical,
}

/// A proposed update to the Knowledge Graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeUpdateProposal {
    pub entity_name: String,
    pub entity_type: String,
    pub relationships: Vec<RelationshipProposal>,
    pub facts: Vec<FactProposal>,
    pub confidence: f32,
    pub source_citations: Vec<uuid::Uuid>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipProposal {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    pub confidence: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactProposal {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
}

/// A proposed update to the World Model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldUpdateProposal {
    pub entity_name: String,
    pub entity_type: String,
    pub state_changes: std::collections::HashMap<String, String>,
    pub location: Option<String>,
    pub events: Vec<WorldEventProposal>,
    pub confidence: f32,
    pub source_citations: Vec<uuid::Uuid>,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEventProposal {
    pub description: String,
    pub event_type: String,
    pub participants: Vec<String>,
    pub significance: f32,
}

/// A proposed update to Memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpdateProposal {
    pub content: String,
    pub memory_type: String,
    pub importance: f32,
    pub context: std::collections::HashMap<String, String>,
    pub source_citations: Vec<uuid::Uuid>,
}

/// Summary of a completed research pipeline stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageResult {
    pub stage_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub items_processed: usize,
    pub items_output: usize,
    pub error: Option<String>,
}

/// Progress event emitted during pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchProgressEvent {
    pub task_id: ResearchTaskId,
    pub stage: String,
    pub progress: f32,
    pub message: String,
    pub timestamp: Timestamp,
}

/// Search query constructed from a research objective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub original_objective: String,
    pub search_terms: Vec<String>,
    pub provider: String,
    pub max_results: usize,
    pub content_type_filter: Option<String>,
    pub time_range: Option<(Timestamp, Timestamp)>,
}

/// A single search result before fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub provider: String,
    pub rank: usize,
    pub estimated_relevance: f32,
}

/// Fetched content from a URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedContent {
    pub url: String,
    pub content_type: String,
    pub raw_content: Vec<u8>,
    pub text_content: String,
    pub metadata: std::collections::HashMap<String, String>,
    pub fetched_at: Timestamp,
    pub size_bytes: usize,
}

/// Extracted information from content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedInformation {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub events: Vec<ExtractedEvent>,
    pub dates: Vec<ExtractedDate>,
    pub locations: Vec<ExtractedLocation>,
    pub citations: Vec<Citation>,
    pub facts: Vec<ExtractedFact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub context: String,
    pub confidence: f32,
    pub mentions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    pub context: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEvent {
    pub description: String,
    pub event_type: String,
    pub participants: Vec<String>,
    pub date: Option<String>,
    pub location: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDate {
    pub original_text: String,
    pub parsed_value: Option<String>,
    pub date_type: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedLocation {
    pub name: String,
    pub location_type: String,
    pub context: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFact {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
    pub source_url: Option<String>,
    pub supporting_text: String,
}

/// Validated fact with provenance and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedFact {
    pub fact: ExtractedFact,
    pub confidence: f32,
    pub provenance: ResearchProvenance,
    pub supporting_evidence_count: usize,
    pub conflicting_evidence_count: usize,
    pub is_conflict: bool,
    pub validated_at: Timestamp,
}

/// Ranked and deduplicated finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedFinding {
    pub finding: Finding,
    pub rank: usize,
    pub composite_score: f32,
    pub diversity_contribution: f32,
}
