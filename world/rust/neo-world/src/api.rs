use std::sync::Arc;

use crate::builders::EntityBuilder;
use crate::causal::{CausalLink, CausalModel};
use crate::config::WorldConfig;
use crate::distributed::DistributedManager;
use crate::entity::{EntityTracker, WorldEntity};
use crate::environment::EnvironmentManager;
use crate::error::{WorldError, WorldResult};
use crate::history::HistoryManager;
use crate::metrics::WorldMetrics;
use crate::observation::{Observation, ObservationPipeline};
use crate::ontology::EntityTypeRegistry;
use crate::perception::{Perception, PerceptionBuffer};
use crate::persistence::PersistenceManager;
use crate::prediction::PredictionEngine;
use crate::relationships::{Relationship, RelationshipManager};
use crate::simulation::{SimulationEngine, SimulationScenario};
use crate::spatial::{Location, SpatialModel};
use crate::synchronization::SynchronizationManager;
use crate::temporal::{TemporalEvent, TemporalModel, TimeWindow};
use crate::types::{
    EntityId, EntityQuery, EntityType, EntityState,
    PredictionType, QueryResult, SimulationId, WorldContext, WorldSnapshot,
};
use crate::uncertainty::UncertaintyTracker;

/// The internal world model — Neo's representation of reality.
pub struct WorldModel {
    config: WorldConfig,
    entities: Arc<EntityTracker>,
    relationships: Arc<RelationshipManager>,
    spatial: Arc<tokio::sync::RwLock<SpatialModel>>,
    temporal: Arc<tokio::sync::RwLock<TemporalModel>>,
    causal: Arc<tokio::sync::RwLock<CausalModel>>,
    environments: Arc<EnvironmentManager>,
    observation_pipeline: Arc<ObservationPipeline>,
    perception_buffer: Arc<tokio::sync::RwLock<PerceptionBuffer>>,
    uncertainty: Arc<UncertaintyTracker>,
    prediction: Arc<PredictionEngine>,
    simulation: Arc<SimulationEngine>,
    history: Arc<tokio::sync::RwLock<HistoryManager>>,
    state: Arc<tokio::sync::RwLock<crate::state::WorldStateManager>>,
    ontology: Arc<tokio::sync::RwLock<EntityTypeRegistry>>,
    persistence: Arc<PersistenceManager>,
    synchronization: Arc<SynchronizationManager>,
    distributed: Arc<DistributedManager>,
    metrics: Arc<WorldMetrics>,
    initialized: bool,
}

impl WorldModel {
    pub fn new(config: WorldConfig) -> Self {
        let sim_max = config.max_concurrent_simulations;
        let hist_max = config.max_events;
        let snap_max = config.max_snapshots;

        Self {
            entities: Arc::new(EntityTracker::new()),
            relationships: Arc::new(RelationshipManager::new()),
            spatial: Arc::new(tokio::sync::RwLock::new(SpatialModel::new())),
            temporal: Arc::new(tokio::sync::RwLock::new(TemporalModel::new())),
            causal: Arc::new(tokio::sync::RwLock::new(CausalModel::new())),
            environments: Arc::new(EnvironmentManager::new()),
            observation_pipeline: Arc::new(ObservationPipeline::new()),
            perception_buffer: Arc::new(tokio::sync::RwLock::new(
                PerceptionBuffer::new(config.max_perception_queue),
            )),
            uncertainty: Arc::new(UncertaintyTracker::new()),
            prediction: Arc::new(PredictionEngine::new()),
            simulation: Arc::new(SimulationEngine::new(sim_max)),
            history: Arc::new(tokio::sync::RwLock::new(HistoryManager::new(hist_max))),
            state: Arc::new(tokio::sync::RwLock::new(
                crate::state::WorldStateManager::new(snap_max),
            )),
            ontology: Arc::new(tokio::sync::RwLock::new(EntityTypeRegistry::with_defaults())),
            persistence: Arc::new(PersistenceManager::new(config.persistence_path.clone())),
            synchronization: Arc::new(SynchronizationManager::new()),
            distributed: Arc::new(DistributedManager::new("local")),
            metrics: Arc::new(WorldMetrics::default()),
            config,
            initialized: false,
        }
    }

    #[must_use]
    pub fn default_config() -> Self {
        Self::new(WorldConfig::default())
    }

    pub async fn initialize(&mut self) -> WorldResult<()> {
        tracing::info!("Initializing world model");
        {
            let mut state = self.state.write().await;
            state.snapshot("World model initialized");
        }
        self.initialized = true;
        tracing::info!("World model initialized");
        Ok(())
    }

    fn ensure_initialized(&self) -> WorldResult<()> {
        if self.initialized {
            Ok(())
        } else {
            Err(WorldError::NotInitialized)
        }
    }

    // ── Entity Operations ──────────────────────────────────────────────

    pub async fn add_entity(&self, entity: WorldEntity) -> EntityId {
        self.metrics.entities_created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.entities.add(entity)
    }

    pub async fn create_entity(&self, name: impl Into<String>, entity_type: EntityType) -> EntityId {
        let entity = WorldEntity::new(name, entity_type);
        self.add_entity(entity).await
    }

    pub async fn build_entity(
        &self,
        name: impl Into<String>,
        entity_type: EntityType,
    ) -> EntityBuilder {
        EntityBuilder::new(name, entity_type)
    }

    pub async fn get_entity(&self, id: &EntityId) -> Option<WorldEntity> {
        self.entities.get(id)
    }

    pub async fn update_entity_state(
        &self,
        id: &EntityId,
        state: EntityState,
        reason: impl Into<String>,
    ) -> WorldResult<()> {
        let version = self.state.read().await.current_version();
        if let Some(mut entity) = self.entities.get_mut(id) {
            entity.transition(state, reason, version);
            self.metrics.entities_updated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        } else {
            Err(WorldError::EntityNotFound(id.to_string()))
        }
    }

    pub async fn find_entity(&self, query: &EntityQuery) -> Vec<WorldEntity> {
        let mut results = if let Some(ref name) = query.name {
            self.entities.by_name(name)
        } else if let Some(ref et) = query.entity_type {
            self.entities.by_type(et)
        } else if let Some(ref tag) = query.tag {
            self.entities.by_tag(tag)
        } else if let Some(ref state) = query.state {
            self.entities.by_state(state)
        } else {
            self.entities.all()
        };

        if let Some(min_conf) = query.min_confidence {
            results.retain(|e| e.confidence.value() >= min_conf);
        }
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        results
    }

    pub async fn find_entities(&self, query: &EntityQuery) -> QueryResult<WorldEntity> {
        let version = self.state.read().await.current_version();
        let items = self.find_entity(query).await;
        let total = items.len();
        QueryResult {
            items,
            total_count: total,
            version,
            query_time_ms: 0,
        }
    }

    pub async fn remove_entity(&self, id: &EntityId) -> bool {
        let result = self.entities.remove(id);
        if result {
            self.metrics.entities_deleted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }

    pub async fn entity_count(&self) -> usize {
        self.entities.count()
    }

    // ── Relationship Operations ────────────────────────────────────────

    pub async fn add_relationship(&self, relationship: Relationship) -> crate::types::RelationshipId {
        self.metrics.relationships_created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.relationships.add(relationship)
    }

    pub async fn find_related(&self, entity_id: &EntityId) -> Vec<Relationship> {
        self.relationships.involving(entity_id)
    }

    pub async fn relationships_between(
        &self,
        source: &EntityId,
        target: &EntityId,
    ) -> Vec<Relationship> {
        self.relationships.between(source, target)
    }

    // ── Spatial Operations ─────────────────────────────────────────────

    pub async fn add_location(&self, location: Location) -> crate::types::LocationId {
        let mut spatial = self.spatial.write().await;
        spatial.add_location(location)
    }

    pub async fn find_nearby(
        &self,
        coords: &crate::spatial::Coordinates,
        radius: f64,
    ) -> Vec<(Location, f64)> {
        let spatial = self.spatial.read().await;
        spatial.nearby(coords, radius)
    }

    pub async fn occupants_at(&self, location_id: &crate::types::LocationId) -> Vec<EntityId> {
        let spatial = self.spatial.read().await;
        spatial.occupants_at(location_id)
    }

    // ── Temporal Operations ────────────────────────────────────────────

    pub async fn record_event(&self, event: TemporalEvent) -> crate::types::EventId {
        let mut temporal = self.temporal.write().await;
        self.metrics.events_recorded.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        temporal.record_event(event)
    }

    pub async fn timeline(&self, count: usize) -> Vec<TemporalEvent> {
        let temporal = self.temporal.read().await;
        temporal.recent(count)
    }

    pub async fn events_in_window(&self, window: &TimeWindow) -> Vec<TemporalEvent> {
        let temporal = self.temporal.read().await;
        temporal.events_in_window(window)
    }

    // ── Causal Operations ──────────────────────────────────────────────

    pub async fn root_cause(&self, event_id: &crate::types::EventId) -> Vec<crate::types::EventId> {
        let causal = self.causal.read().await;
        causal.root_causes(event_id)
    }

    pub async fn add_causal_link(&self, link: CausalLink) {
        let mut causal = self.causal.write().await;
        causal.add_link(link);
    }

    // ── Prediction ─────────────────────────────────────────────────────

    pub async fn predict(
        &self,
        description: impl Into<String>,
        prediction_type: PredictionType,
        confidence: crate::types::Confidence,
        reasoning: impl Into<String>,
    ) -> crate::types::PredictionId {
        self.metrics.predictions_made.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.prediction.predict(description, prediction_type, confidence, reasoning)
    }

    // ── Simulation ─────────────────────────────────────────────────────

    pub async fn simulate(
        &self,
        scenario: &SimulationScenario,
    ) -> Result<SimulationId, String> {
        self.metrics.simulations_run.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let state = serde_json::to_value(&*self.entities.all())
            .unwrap_or(serde_json::Value::Null);
        self.simulation.run(scenario, &state)
    }

    // ── Perception & Observation ───────────────────────────────────────

    pub async fn process_perception(&self, perception: &Perception) -> WorldResult<()> {
        self.ensure_initialized()?;
        let mut buffer = self.perception_buffer.write().await;
        buffer.add(perception.clone());
        Ok(())
    }

    pub async fn submit_observation(&self, observation: Observation) {
        self.metrics.observations_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.observation_pipeline.submit(observation);
    }

    // ── Snapshot & State ───────────────────────────────────────────────

    pub async fn snapshot(&self, summary: impl Into<String>) -> WorldResult<WorldSnapshot> {
        self.ensure_initialized()?;
        let mut state = self.state.write().await;
        Ok(state.snapshot(summary))
    }

    pub async fn current_state(&self) -> WorldContext {
        let version = self.state.read().await.current_version();
        let entity_count = self.entities.count();
        let active = self.entities.active().len();

        WorldContext {
            version,
            entity_count,
            active_entity_count: active,
            recent_events_count: self.temporal.read().await.count(),
            environment_summary: format!("{} environments", self.environments.count()),
            key_entities: Vec::new(),
            active_goals: Vec::new(),
            pending_predictions: self.prediction.unresolved().len(),
        }
    }

    // ── Query API ──────────────────────────────────────────────────────

    pub async fn world_summary(&self) -> String {
        let version = self.state.read().await.current_version();
        let ec = self.entities.count();
        let rc = self.relationships.count();
        let spatial = self.spatial.read().await;
        let loc_count = spatial.count();
        let temporal = self.temporal.read().await;
        let evt_count = temporal.count();
        let causal = self.causal.read().await;
        let causal_count = causal.count();

        format!(
            "World {version}: {ec} entities, {rc} relationships, {loc_count} locations, {evt_count} events, {causal_count} causal links, {} environments",
            self.environments.count()
        )
    }

    // ── Accessors ──────────────────────────────────────────────────────

    #[must_use]
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    pub fn entities(&self) -> &EntityTracker {
        &self.entities
    }

    pub fn relationships_manager(&self) -> &RelationshipManager {
        &self.relationships
    }

    pub fn environments_manager(&self) -> &EnvironmentManager {
        &self.environments
    }

    pub fn metrics(&self) -> &WorldMetrics {
        &self.metrics
    }
}

impl std::fmt::Debug for WorldModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldModel")
            .field("initialized", &self.initialized)
            .finish()
    }
}
