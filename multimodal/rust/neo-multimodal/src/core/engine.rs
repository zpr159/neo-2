#!\[forbid(unsafe_code)\]
#![deny(
    missing_docs,
    warnings,
    trivial_casts,
    trivial_numeric_casts,
    unused_import_braces,
    unused_extern_crates
)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque, HashSet};
use async_trait::async_trait;
use tracing::info;

use crate::types::*;
use crate::error::MultimodalResult;
use crate::events::{EventBus, LearningEvent, LearningEventType};
use crate::integration::{PlanningIntegration, ReasoningIntegration, LearningIntegration};
use crate::security::{MediaSecurity, SecurityPolicy};

#[derive(Debug)]
pub struct MultimodalEngine {
    pub config: MultimodalConfiguration,
    pub router: MultimodalRouter,
    pub repository: MultimodalRepository,
    pub event_bus: EventBus,
    pub analytics: MultimodalAnalytics,
    pub security: MediaSecurity,
    pub session_manager: SessionManager,
}

impl MultimodalEngine {
    pub async fn new(config: MultimodalConfiguration) -> Self {
        let event_bus = EventBus::new(1024);
        let repository = MultimodalRepository::new(&config.storage_config).await;
        let security = MediaSecurity::new(config.security_policy.clone());
        let session_manager = SessionManager::new();
        let router = MultimodalRouter::new(repository.clone());
        let analytics = MultimodalAnalytics::new();

        let engine = Self {
            config,
            router,
            repository,
            event_bus,
            analytics,
            security,
            session_manager,
        };

        engine.register_default_processors();
        engine
    }

    pub fn builder() -> MultimodalEngineBuilder {
        MultimodalEngineBuilder::default()
    }

    fn register_default_processors(&self) {
        self.router.register_processor(Box::new(TextProcessor::new()));
        self.router.register_processor(Box::new(ImageProcessor::new()));
        self.router.register_processor(Box::new(AudioProcessor::new()));
        self.router.register_processor(Box::new(VideoProcessor::new()));
        self.router.register_processor(Box::new(DocumentProcessor::new()));
        self.router.register_processor(Box::new(OCRProcessor::new()));
        self.router.register_processor(Box::new(SpeechProcessor::new()));
        self.router.register_processor(Box::new(UIPipeline::new()));
    }

    pub async fn process(&self, request: ProcessingRequest) -> MultimodalResult<ProcessingResponse> {
        let session_id = self.session_manager.create_session(request.clone()).await?;

        info!("Processing request: session={}, modality={:?}, format={}", 
               session_id, request.modality, request.format);

        self.event_bus.publish(ProcessingEvent::new(
            ProcessingEventType::RequestReceived,
            "engine",
            session_id,
            request.clone(),
        )?);

        self.security.validate_request(&request).await?;

        let source_asset = self.repository.get_asset(request.source).await?;

        let mut response = ProcessingResponse {
            session_id,
            status: ProcessingStatus::Processing,
            result: None,
            metadata: ProcessingMetadata {
                start_time: Utc::now(),
                end_time: None,
                processing_time_ms: 0,
                confidence_score: 0.0,
                warnings: Vec::new(),
            },
            progress: ProcessingProgress::new(),
        };

        let processor = self.router.get_processor(&request.modality, &request.format)?;
        let intermediate_result = processor.process(request.clone(), response.progress.clone()).await?;

        let output = intermediate_result.output;
        let output_format = match request.output_format {
            Some(fmt) => fmt,
            None => request.format,
        };

        let output_asset = self.repository.store_asset(output, MediaMetadata {
            id: MediaAssetId::new(),
            name: format!("processed_{}", session_id),
            description: Some(format!("Processed from {} to {:?}", request.source, output_format)),
            modality: request.output_modality.unwrap_or(request.modality),
            format: output_format,
            source: MediaSource::Generated {
                generator: "multimodal_engine".to_string(),
                parameters: HashMap::new(),
            },
            size_bytes: 0,
            mime_type: output_format.to_mime_type(),
            encoding: "binary".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            modified_by: None,
            metadata: HashMap::new(),
            security_level: SecurityLevel::Internal,
            retention_days: None,
        }).await?;

        response.result = Some(output_asset);
        response.metadata.end_time = Some(Utc::now());
        response.metadata.processing_time_ms = (response.metadata.end_time.unwrap() - response.metadata.start_time).num_milliseconds() as u64;
        response.status = ProcessingStatus::Completed;

        self.event_bus.publish(ProcessingEvent::new(
            ProcessingEventType::ProcessingCompleted,
            "engine",
            session_id,
            response.clone(),
        )?);

        self.analytics.record_processing(&response).await?;

        Ok(response)
    }

    pub async fn analyze(&self, asset_id: MediaAssetId) -> MultimodalResult<AnalysisResult> {
        let asset = self.repository.get_asset(asset_id).await?;

        self.event_bus.publish(AnalysisEvent::new(
            AnalysisEventType::AnalysisStarted,
            "engine",
            asset.id,
        )?);

        let modality = asset.metadata.modality;
        let mut processor = self.router.get_processor(&modality, &asset.metadata.format)?;

        let analysis = processor.analyze(asset).await?;

        self.event_bus.publish(AnalysisEvent::new(
            AnalysisEventType::AnalysisCompleted,
            "engine",
            asset.id,
            analysis.clone(),
        )?);

        self.analytics.record_analysis(&analysis).await?;

        Ok(analysis)
    }

    pub async fn search(&self, query: SearchQuery) -> MultimodalResult<SearchResult> {
        self.event_bus.publish(SearchEvent::new(
            SearchEventType::SearchStarted,
            "engine",
            query.clone(),
        )?);

        let results = match query.search_type {
            SearchType::Embedding { query_embedding } => {
                self.repository.search_by_embedding(query_embedding, query.modality, query.limit).await?
            }
            SearchType::Keyword { keywords } => {
                self.repository.search_by_keyword(&keywords, query.modality, query.limit).await?
            }
            SearchType::Similarity { reference_id } => {
                self.repository.search_by_similarity(reference_id, query.modality, query.threshold, query.limit).await?
            }
            SearchType::CrossModal { query_embedding } => {
                self.repository.search_cross_modal(query_embedding, query.modality, query.limit).await?
            }
        };

        self.event_bus.publish(SearchEvent::new(
            SearchEventType::SearchCompleted,
            "engine",
            query.clone(),
            SearchResult {
                query: query.clone(),
                results,
                total_results: results.len(),
                execution_time_ms: 0,
            },
        )?);

        self.analytics.record_search(&results).await?;

        Ok(SearchResult {
            query,
            results,
            total_results: results.len(),
            execution_time_ms: 0,
        })
    }

    pub async fn embed(&self, asset_id: MediaAssetId) -> MultimodalResult<EmbeddingResult> {
        self.event_bus.publish(EmbeddingEvent::new(
            EmbeddingEventType::EmbeddingStarted,
            "engine",
            asset_id,
        )?);

        let asset = self.repository.get_asset(asset_id).await?;
        let modality = asset.metadata.modality;
        let format = asset.metadata.format;

        let mut processor = self.router.get_processor(&modality, &format)?;
        let embedding = processor.embed(asset).await?;

        self.event_bus.publish(EmbeddingEvent::new(
            EmbeddingEventType::EmbeddingCompleted,
            "engine",
            asset_id,
            embedding.clone(),
        )?);

        self.analytics.record_embedding(&embedding).await?;

        Ok(EmbeddingResult {
            asset_id,
            embedding: embedding.clone(),
            modality,
            model: embedding.model,
            execution_time_ms: 0,
        })
    }

    pub async fn compare(&self, asset_id1: MediaAssetId, asset_id2: MediaAssetId) -> MultimodalResult<ComparisonResult> {
        let asset1 = self.repository.get_asset(asset_id1).await?;
        let asset2 = self.repository.get_asset(asset_id2).await?;

        let modality1 = asset1.metadata.modality;
        let modality2 = asset2.metadata.modality;

        if modality1 != modality2 {
            return Err(MultimodalError::ProcessingError(
                "Assets have different modalities".to_string()
            ));
        }

        let processor = self.router.get_processor(&modality1, &asset1.metadata.format)?;
        let comparison = processor.compare(asset1, asset2).await?;

        Ok(comparison)
    }

    pub async fn process_stream(&self, request: StreamingRequest) -> MultimodalResult<StreamResponse> {
        self.event_bus.publish(StreamEvent::new(
            StreamEventType::StreamStarted,
            "engine",
            request.clone(),
        )?);

        let processor = self.router.get_processor(&request.modality, &request.format)?;
        let stream_response = processor.process_stream(request).await?;

        self.event_bus.publish(StreamEvent::new(
            StreamEventType::StreamCompleted,
            "engine",
            request.clone(),
            stream_response.clone(),
        )?);

        self.analytics.record_stream(&stream_response).await?;

        Ok(stream_response)
    }

    pub async fn create_pipeline(&self, pipeline: MediaPipeline) -> MultimodalResult<MediaPipelineId> {
        self.security.validate_pipeline(&pipeline).await?;
        let pipeline_id = self.repository.store_pipeline(pipeline).await?;

        self.event_bus.publish(PipelineEvent::new(
            PipelineEventType::PipelineCreated,
            "engine",
            pipeline_id,
        )?);

        Ok(pipeline_id)
    }

    pub async fn get_status(&self) -> MultimodalResult<EngineStatus> {
        let session_stats = self.session_manager.get_stats().await?;
        let processing_stats = self.analytics.get_processing_stats().await?;
        let security_stats = self.security.get_stats().await?;

        Ok(EngineStatus {
            sessions: session_stats,
            processing: processing_stats,
            security: security_stats,
            uptime: self.config.uptime,
        })
    }

    pub async fn shutdown(&self) -> MultimodalResult<()> {
        self.event_bus.publish(SystemEvent::new(
            SystemEventType::Shutdown,
            "engine",
        )?);

        self.repository.shutdown().await?;

        info!("Multimodal Engine shutdown completed");

        Ok(())
    }
}

#[derive(Debug)]
pub struct MultimodalEngineBuilder {
    config: Option<MultimodalConfiguration>,
    storage_config: Option<StorageConfiguration>,
    security_policy: Option<SecurityPolicy>,
    register_processors: bool,
}

impl Default for MultimodalEngineBuilder {
    fn default() -> Self {
        Self {
            config: None,
            storage_config: None,
            security_policy: None,
            register_processors: true,
        }
    }
}

impl MultimodalEngineBuilder {
    pub fn config(mut self, config: MultimodalConfiguration) -> Self {
        self.config = Some(config);
        self
    }

    pub fn storage_config(mut self, config: StorageConfiguration) -> Self {
        self.storage_config = Some(config);
        self
    }

    pub fn security_policy(mut self, policy: SecurityPolicy) -> Self {
        self.security_policy = Some(policy);
        self
    }

    pub fn register_processors(mut self, register: bool) -> Self {
        self.register_processors = register;
        self
    }

    pub async fn build(self) -> MultimodalResult<MultimodalEngine> {
        let config = self.config.unwrap_or_else(MultimodalConfiguration::default);
        let storage_config = self.storage_config.unwrap_or_else(StorageConfiguration::default);
        let security_policy = self.security_policy.unwrap_or_else(SecurityPolicy::default);

        MultimodalEngine::new(config).await
    }
}
