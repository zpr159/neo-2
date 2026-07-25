use std::sync::Arc;

use tokio::sync::RwLock;

use super::api::{
    Finding, FetchedContent, PipelineStageResult, ResearchProgressEvent,
    ResearchRequest, ResearchTask, ResearchTaskId, ResearchTaskMetrics,
    ResearchTaskStatus, ResearchEvidence,
};
use super::citation::CitationManager;
use super::config::ResearchConfig;
use super::crawler::ResearchCrawler;
use super::deduplication::Deduplicator;
use super::error::{ResearchError, ResearchResult};
use super::extractor::InformationExtractor;
use super::fetcher::ContentFetcher;
use super::knowledge_update::KnowledgeUpdateManager;
use super::memory_update::MemoryUpdateManager;
use super::planner::ResearchPlanner;
use super::ranking::FindingRanker;
use super::synthesis::ResearchSynthesizer;
use super::validator::FactValidator;
use super::world_update::WorldUpdateManager;
use crate::time::Timestamp;

/// Events emitted during pipeline execution.
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    StageStarted {
        task_id: ResearchTaskId,
        stage: String,
    },
    StageCompleted {
        task_id: ResearchTaskId,
        result: PipelineStageResult,
    },
    Progress {
        event: ResearchProgressEvent,
    },
    TaskCompleted {
        task_id: ResearchTaskId,
    },
    TaskFailed {
        task_id: ResearchTaskId,
        error: String,
    },
}

/// Executes the full research pipeline.
pub struct ResearchWorkflow {
    config: ResearchConfig,
    planner: ResearchPlanner,
    crawler: Arc<ResearchCrawler>,
    fetcher: ContentFetcher,
    extractor: InformationExtractor,
    validator: FactValidator,
    ranker: FindingRanker,
    deduplicator: Deduplicator,
    citation_manager: CitationManager,
    synthesizer: ResearchSynthesizer,
    knowledge_update: KnowledgeUpdateManager,
    world_update: WorldUpdateManager,
    memory_update: MemoryUpdateManager,
    tasks: Arc<RwLock<std::collections::HashMap<ResearchTaskId, ResearchTask>>>,
}

impl ResearchWorkflow {
    pub fn new(config: ResearchConfig) -> ResearchResult<Self> {
        let crawler = super::crawler::initialize_crawler(
            &config.search_providers,
            config.fetcher.max_concurrent_fetches,
        );

        Ok(Self {
            planner: ResearchPlanner::new(config.search_providers.clone()),
            crawler: Arc::new(crawler),
            fetcher: ContentFetcher::new(config.fetcher.clone())?,
            extractor: InformationExtractor::new(config.extractor.clone()),
            validator: FactValidator::new(config.validator.clone()),
            ranker: FindingRanker::new(config.ranking.clone()),
            deduplicator: Deduplicator::new(config.deduplication.clone()),
            citation_manager: CitationManager::new(config.citation.clone()),
            synthesizer: ResearchSynthesizer::new(config.synthesis.clone()),
            knowledge_update: KnowledgeUpdateManager::new(config.knowledge_update.clone()),
            world_update: WorldUpdateManager::new(config.world_update.clone()),
            memory_update: MemoryUpdateManager::new(config.memory_update.clone()),
            tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            config,
        })
    }

    /// Create a new research task.
    pub async fn create_task(&self, request: ResearchRequest) -> ResearchResult<ResearchTask> {
        let task_id = ResearchTaskId::new_v4();

        let plan = self.planner.plan(&request)?;

        let task = ResearchTask {
            id: task_id,
            request,
            status: ResearchTaskStatus::Created,
            created_at: Timestamp::now(),
            started_at: None,
            completed_at: None,
            progress: 0.0,
            current_stage: None,
            result: None,
            error: None,
            metrics: ResearchTaskMetrics::default(),
        };

        self.tasks.write().await.insert(task_id, task.clone());
        let _ = plan;

        Ok(task)
    }

    /// Execute a research task through the full pipeline.
    pub async fn execute_task(
        &self,
        task_id: ResearchTaskId,
    ) -> ResearchResult<super::api::ResearchOutput> {
        let task = {
            let tasks = self.tasks.read().await;
            tasks
                .get(&task_id)
                .cloned()
                .ok_or_else(|| ResearchError::TaskNotFound(task_id.to_string()))?
        };

        self.update_status(task_id, ResearchTaskStatus::Planning, 0.0)
            .await;

        let plan = self.planner.plan(&task.request)?;

        let mut metrics = ResearchTaskMetrics::default();
        let stage_start = std::time::Instant::now();

        self.update_status(task_id.clone(), ResearchTaskStatus::Searching, 0.1)
            .await;

        let mut all_search_results = Vec::new();
        for query in &plan.search_queries {
            match self.crawler.search(query, &plan.search_providers).await {
                Ok(results) => {
                    metrics.sources_searched += results.len();
                    all_search_results.extend(results);
                }
                Err(e) => {
                    tracing::warn!("search failed for provider {}: {}", query.provider, e);
                }
            }
        }

        let search_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("search".to_string(), search_duration);

        self.update_status(
            task_id.clone(),
            ResearchTaskStatus::Fetching,
            0.2,
        )
        .await;

        let stage_start = std::time::Instant::now();
        let fetch_results = self.fetcher.fetch_many(&all_search_results).await;

        let mut fetched_contents: Vec<FetchedContent> = Vec::new();
        for result in fetch_results {
            match result {
                Ok(content) => {
                    metrics.sources_fetched += 1;
                    fetched_contents.push(content);
                }
                Err(e) => {
                    metrics.sources_failed += 1;
                    tracing::warn!("fetch failed: {}", e);
                }
            }
        }

        let fetch_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("fetch".to_string(), fetch_duration);

        self.update_status(
            task_id.clone(),
            ResearchTaskStatus::Extracting,
            0.4,
        )
        .await;

        let stage_start = std::time::Instant::now();
        let extracted = self
            .extractor
            .extract_many(&fetched_contents)?;
        metrics.facts_extracted = extracted.facts.len();

        let extraction_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("extract".to_string(), extraction_duration);

        self.update_status(
            task_id.clone(),
            ResearchTaskStatus::Validating,
            0.5,
        )
        .await;

        let stage_start = std::time::Instant::now();
        let source_urls: Vec<String> = fetched_contents.iter().map(|c| c.url.clone()).collect();
        let source_names: Vec<String> = fetched_contents
            .iter()
            .map(|c| extract_domain_from_url(&c.url))
            .collect();
        let validated_facts = self.validator.validate_cross_source(
            &extracted.facts,
            &source_urls,
            &source_names,
        )?;

        let filtered_facts = self.validator.filter_by_confidence(validated_facts, None);
        metrics.facts_validated = filtered_facts.len();

        let validation_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("validate".to_string(), validation_duration);

        self.update_status(task_id.clone(), ResearchTaskStatus::Ranking, 0.6)
            .await;

        let stage_start = std::time::Instant::now();
        let findings = super::workflow::build_findings_from_facts(&filtered_facts, &fetched_contents);
        let deduplicated = self.deduplicator.deduplicate(findings);
        metrics.duplicates_removed = metrics.facts_extracted.saturating_sub(deduplicated.len());

        let ranked = self.ranker.rank(deduplicated);
        metrics.citations_generated = extracted.citations.len();

        let ranking_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("rank".to_string(), ranking_duration);

        self.update_status(
            task_id.clone(),
            ResearchTaskStatus::Synthesizing,
            0.7,
        )
        .await;

        let stage_start = std::time::Instant::now();
        let result = self.synthesizer.synthesize(
            ranked,
            &filtered_facts,
            &extracted.citations,
            &task.request.objective,
        )?;

        let synthesis_duration = stage_start.elapsed().as_millis() as u64;
        metrics
            .stage_durations_ms
            .insert("synthesis".to_string(), synthesis_duration);

        if task.request.update_knowledge {
            self.update_status(
                task_id.clone(),
                ResearchTaskStatus::UpdatingKnowledge,
                0.8,
            )
            .await;
            let approved_knowledge = self
                .knowledge_update
                .filter_approved(result.knowledge_updates.clone());
            metrics.knowledge_updates_proposed = result.knowledge_updates.len();
            metrics.knowledge_updates_approved = approved_knowledge.len();
        }

        if task.request.update_world_model {
            self.update_status(
                task_id.clone(),
                ResearchTaskStatus::UpdatingWorld,
                0.85,
            )
            .await;
            let approved_world = self
                .world_update
                .filter_approved(result.world_updates.clone());
            metrics.world_updates_proposed = result.world_updates.len();
            let _ = approved_world;
        }

        if task.request.update_memory {
            self.update_status(
                task_id.clone(),
                ResearchTaskStatus::UpdatingMemory,
                0.9,
            )
            .await;
            let approved_memory = self
                .memory_update
                .filter_approved(result.memory_updates.clone());
            let _ = approved_memory;
        }

        metrics.total_duration_ms = (task.created_at.elapsed_secs() * 1000.0) as u64;

        self.update_status(task_id.clone(), ResearchTaskStatus::Completed, 1.0)
            .await;

        {
            let mut tasks = self.tasks.write().await;
            if let Some(task) = tasks.get_mut(&task_id) {
                task.completed_at = Some(Timestamp::now());
                task.metrics = metrics;
                task.result = Some(result.clone());
            }
        }

        Ok(result)
    }

    /// Get the status of a research task.
    pub async fn get_task(&self, task_id: &ResearchTaskId) -> Option<ResearchTask> {
        self.tasks.read().await.get(task_id).cloned()
    }

    /// Get all tasks.
    pub async fn list_tasks(&self) -> Vec<ResearchTask> {
        self.tasks.read().await.values().cloned().collect()
    }

    /// Cancel a running task.
    pub async fn cancel_task(&self, task_id: ResearchTaskId) -> ResearchResult<()> {
        let mut tasks = self.tasks.write().await;
        let task = tasks
            .get_mut(&task_id)
            .ok_or_else(|| ResearchError::TaskNotFound(task_id.to_string()))?;

        task.status = ResearchTaskStatus::Cancelled;
        task.completed_at = Some(Timestamp::now());
        Ok(())
    }

    async fn update_status(
        &self,
        task_id: ResearchTaskId,
        status: ResearchTaskStatus,
        progress: f32,
    ) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            if task.status == ResearchTaskStatus::Cancelled {
                return;
            }
            task.status = status.clone();
            task.progress = progress;
            task.current_stage = Some(format!("{:?}", status).to_lowercase());
        }
    }

    /// Get the research configuration.
    pub fn config(&self) -> &ResearchConfig {
        &self.config
    }
}

fn extract_domain_from_url(url: &str) -> String {
    url.split('/')
        .nth(2)
        .unwrap_or("unknown")
        .to_string()
}

/// Build Finding objects from validated facts for the synthesis stage.
pub fn build_findings_from_facts(
    facts: &[super::api::ValidatedFact],
    contents: &[FetchedContent],
) -> Vec<Finding> {
    facts
        .iter()
        .map(|vf| {
            let supporting_evidence: Vec<ResearchEvidence> = contents
                .iter()
                .filter(|c| {
                    vf.fact
                        .source_url
                        .as_ref()
                        .map(|url| url == &c.url)
                        .unwrap_or(false)
                })
                .map(|c| ResearchEvidence {
                    id: uuid::Uuid::new_v4(),
                    content: vf.fact.supporting_text.clone(),
                    source_url: Some(c.url.clone()),
                    source_name: extract_domain_from_url(&c.url),
                    content_type: c.content_type.clone(),
                    confidence: vf.confidence,
                    extracted_at: Timestamp::now(),
                    relevance_score: vf.confidence,
                })
                .collect();

            let statement = format!(
                "{} {} {}",
                vf.fact.subject, vf.fact.predicate, vf.fact.object
            );

            Finding {
                id: uuid::Uuid::new_v4(),
                statement,
                confidence: vf.confidence,
                supporting_citations: Vec::new(),
                evidence: if supporting_evidence.is_empty() {
                    vec![ResearchEvidence {
                        id: uuid::Uuid::new_v4(),
                        content: vf.fact.supporting_text.clone(),
                        source_url: vf.fact.source_url.clone(),
                        source_name: vf
                            .fact
                            .source_url
                            .as_ref()
                            .map(|u| extract_domain_from_url(u))
                            .unwrap_or_else(|| "unknown".to_string()),
                        content_type: "validated_fact".to_string(),
                        confidence: vf.confidence,
                        extracted_at: Timestamp::now(),
                        relevance_score: vf.confidence,
                    }]
                } else {
                    supporting_evidence
                },
                provenance: vf.provenance.clone(),
                timestamp: vf.validated_at,
            }
        })
        .collect()
}
