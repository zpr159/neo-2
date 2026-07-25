use crate::builders::{EntityBuilder, RelationshipBuilder};
use crate::causal::{CausalLink, CausalModel, CausalStrength};
use crate::confidence::{apply_decay, merge_confidences, ConfidenceAccumulator, Evidence};
use crate::config::WorldConfig;
use crate::distributed::{DistributedManager, DistributedNode};
use crate::entity::{EntityTracker, WorldEntity};
use crate::environment::{Environment, EnvironmentManager};
use crate::error::WorldError;
use crate::history::{HistoryEntry, HistoryManager};
use crate::integration::IntegrationLayer;
use crate::lifecycle::{is_valid_transition, LifecycleEvent, LifecycleManager};
use crate::metrics::WorldMetrics;
use crate::observation::{Observation, ObservationPipeline, ObservationType};
use crate::ontology::EntityTypeRegistry;
use crate::perception::{Perception, PerceptionBuffer, PerceptionFusion, PerceptionProcessor};
use crate::persistence::PersistenceManager;
use crate::prediction::PredictionEngine;
use crate::relationships::{Relationship, RelationshipManager, RelationshipStrength, RelationshipType};
use crate::simulation::{SimulationAction, SimulationEngine, SimulationScenario};
use crate::spatial::{Coordinates, Location, SpatialModel};
use crate::state::WorldStateManager;
use crate::synchronization::SynchronizationManager;
use crate::temporal::{TemporalEvent, TemporalModel, TimeWindow};
use crate::types::*;
use crate::uncertainty::{UncertaintyCategory, UncertaintyTracker};

// ── types.rs ────────────────────────────────────────────────────────────────

#[test]
fn entity_id_random_uniqueness() {
    let a = EntityId::random();
    let b = EntityId::random();
    assert_ne!(a, b);
}

#[test]
fn entity_id_from_str() {
    let id = EntityId::new("test-123");
    assert_eq!(id.as_str(), "test-123");
}

#[test]
fn entity_type_label() {
    assert_eq!(EntityType::Human.label(), "Human");
    assert_eq!(EntityType::Custom("Foo".into()).label(), "Foo");
}

#[test]
fn entity_type_from_str_loose() {
    assert_eq!(EntityType::from_str_loose("agent"), EntityType::Agent);
    assert_eq!(
        EntityType::from_str_loose("customtype"),
        EntityType::Custom("customtype".into())
    );
}

#[test]
fn event_type_label() {
    assert_eq!(EventType::Creation.label(), "creation");
    assert_eq!(EventType::Custom("foo".into()).label(), "foo");
}

#[test]
fn entity_state_properties() {
    assert!(EntityState::Deleted.is_terminal());
    assert!(EntityState::Archived.is_terminal());
    assert!(!EntityState::Active.is_terminal());
    assert!(EntityState::Active.is_active());
    assert!(EntityState::Updating.is_active());
    assert!(!EntityState::Created.is_active());
}

#[test]
fn confidence_level_classification() {
    assert_eq!(Confidence::CERTAIN.level(), ConfidenceLevel::Certain);
    assert_eq!(Confidence::HIGH.level(), ConfidenceLevel::High);
    assert_eq!(Confidence::MEDIUM.level(), ConfidenceLevel::Medium);
    assert_eq!(Confidence::LOW.level(), ConfidenceLevel::Low);
    assert_eq!(Confidence::SPECULATIVE.level(), ConfidenceLevel::Speculative);
    assert_eq!(Confidence::UNKNOWN.level(), ConfidenceLevel::Unknown);
}

#[test]
fn confidence_from_f32_clamps() {
    let c: Confidence = 1.5f32.into();
    assert_eq!(c, Confidence::CERTAIN);
    let c: Confidence = (-0.5f32).into();
    assert_eq!(c, Confidence::UNKNOWN);
}

#[test]
fn confidence_threshold() {
    assert!(Confidence::HIGH.is_high_enough(0.5));
    assert!(!Confidence::LOW.is_high_enough(0.5));
}

#[test]
fn world_version_monotonic() {
    let v0 = WorldVersion::initial();
    let v1 = v0.next();
    assert!(v1 > v0);
    assert_eq!(v1.number(), 1);
}

#[test]
fn world_snapshot_creation() {
    let snap = WorldSnapshot::new(WorldVersion(5), "test snapshot");
    assert_eq!(snap.version, WorldVersion(5));
    assert_eq!(snap.summary, "test snapshot");
    assert_eq!(snap.entity_count, 0);
}

#[test]
fn attribute_value_display() {
    assert_eq!(AttributeValue::Boolean(true).to_string(), "true");
    assert_eq!(AttributeValue::Integer(42).to_string(), "42");
    assert_eq!(AttributeValue::Text("hello".into()).to_string(), "hello");
    assert_eq!(
        AttributeValue::Vector(vec![1.0, 2.0]).to_string(),
        "[2 dims]"
    );
}

#[test]
fn world_context_default() {
    let ctx = WorldContext::default();
    assert_eq!(ctx.version, WorldVersion::initial());
    assert_eq!(ctx.entity_count, 0);
}

// ── entity.rs ───────────────────────────────────────────────────────────────

#[test]
fn entity_creation_and_transitions() {
    let mut entity = WorldEntity::new("Test Agent", EntityType::Agent);
    assert_eq!(entity.state, EntityState::Created);
    assert_eq!(entity.name, "Test Agent");
    assert!(entity.transition(
        EntityState::Active,
        "activated",
        WorldVersion(1)
    ));
    assert_eq!(entity.state, EntityState::Active);
    assert_eq!(entity.version, 2);
}

#[test]
fn entity_invalid_transition() {
    let mut entity = WorldEntity::new("Test", EntityType::Task);
    assert!(!entity.transition(
        EntityState::Archived,
        "skip to archived",
        WorldVersion(1)
    ));
    assert_eq!(entity.state, EntityState::Created);
}

#[test]
fn entity_attributes() {
    let mut entity = WorldEntity::new("Test", EntityType::Object);
    entity.set_attribute("color", AttributeValue::Text("red".into()));
    entity.set_attribute("weight", AttributeValue::Float(1.5));
    assert_eq!(entity.attributes.len(), 2);
    let val = entity.get_attribute("color").unwrap();
    assert_eq!(val, &AttributeValue::Text("red".into()));
}

#[test]
fn entity_tracker_lifecycle() {
    let tracker = EntityTracker::new();
    let entity = WorldEntity::new("Alice", EntityType::Human);
    let id = tracker.add(entity);
    assert!(tracker.contains(&id));
    assert_eq!(tracker.count(), 1);
    assert!(tracker.remove(&id));
    assert!(!tracker.contains(&id));
}

#[test]
fn entity_tracker_by_type() {
    let tracker = EntityTracker::new();
    tracker.add(WorldEntity::new("Alice", EntityType::Human));
    tracker.add(WorldEntity::new("Bob", EntityType::Agent));
    assert_eq!(tracker.by_type(&EntityType::Human).len(), 1);
    assert_eq!(tracker.by_type(&EntityType::Agent).len(), 1);
}

// ── relationships.rs ────────────────────────────────────────────────────────

#[test]
fn relationship_creation_and_indexing() {
    let manager = RelationshipManager::new();
    let src = EntityId::new("a");
    let tgt = EntityId::new("b");
    let rel = Relationship::new(src.clone(), tgt.clone(), RelationshipType::ParentOf);
    let id = manager.add(rel);
    assert!(manager.get(&id).is_some());
    assert_eq!(manager.from_source(&src).len(), 1);
    assert_eq!(manager.to_target(&tgt).len(), 1);
}

#[test]
fn relationship_type_display() {
    assert_eq!(RelationshipType::ParentOf.to_string(), "parent_of");
    assert_eq!(RelationshipType::Custom("foo".into()).to_string(), "foo");
}

#[test]
fn relationship_strength_display() {
    assert_eq!(RelationshipStrength::Strong.to_string(), "strong");
    assert_eq!(RelationshipStrength::Weak.to_string(), "weak");
}

// ── spatial.rs ──────────────────────────────────────────────────────────────

#[test]
fn spatial_model_add_and_query() {
    let mut model = SpatialModel::new();
    let mut loc = Location::new("Office", EnvironmentType::Physical);
    loc.coordinates = Some(Coordinates { x: 10.0, y: 20.0, z: 0.0 });
    let id = model.add_location(loc);
    assert!(model.get_location(&id).is_some());
}

#[test]
fn spatial_region_contains() {
    let region = crate::spatial::SpatialRegion {
        min: Coordinates { x: 0.0, y: 0.0, z: 0.0 },
        max: Coordinates { x: 100.0, y: 100.0, z: 100.0 },
        name: "Zone A".into(),
        region_type: "office".into(),
    };
    assert!(region.contains(&Coordinates { x: 50.0, y: 50.0, z: 50.0 }));
    assert!(!region.contains(&Coordinates { x: 150.0, y: 50.0, z: 50.0 }));
}

#[test]
fn spatial_region_intersects() {
    let a = crate::spatial::SpatialRegion {
        min: Coordinates { x: 0.0, y: 0.0, z: 0.0 },
        max: Coordinates { x: 10.0, y: 10.0, z: 10.0 },
        name: "A".into(),
        region_type: "test".into(),
    };
    let b = crate::spatial::SpatialRegion {
        min: Coordinates { x: 5.0, y: 5.0, z: 5.0 },
        max: Coordinates { x: 15.0, y: 15.0, z: 15.0 },
        name: "B".into(),
        region_type: "test".into(),
    };
    assert!(a.intersects(&b));
    let c = crate::spatial::SpatialRegion {
        min: Coordinates { x: 20.0, y: 20.0, z: 20.0 },
        max: Coordinates { x: 30.0, y: 30.0, z: 30.0 },
        name: "C".into(),
        region_type: "test".into(),
    };
    assert!(!a.intersects(&c));
}

// ── temporal.rs ─────────────────────────────────────────────────────────────

#[test]
fn temporal_model_record_and_query() {
    let mut model = TemporalModel::new();
    let event = TemporalEvent::new("Entity created", EventType::Creation);
    let id = model.record_event(event);
    assert!(model.get_event(&id).is_some());
}

#[test]
fn time_window_contains() {
    use chrono::Utc;
    let now = Utc::now();
    let window = TimeWindow::closed("test", now - chrono::Duration::hours(1), now + chrono::Duration::hours(1));
    assert!(window.contains(&now));
    assert!(!window.contains(&(now - chrono::Duration::hours(2))));
}

#[test]
fn time_window_open_ended() {
    use chrono::Utc;
    let now = Utc::now();
    let window = TimeWindow::open("open", now - chrono::Duration::hours(1));
    assert!(window.contains(&now));
    assert!(window.contains(&(now + chrono::Duration::hours(100))));
}

// ── causal.rs ───────────────────────────────────────────────────────────────

#[test]
fn causal_model_add_and_query() {
    let mut model = CausalModel::new();
    let cause = EventId::new("e1");
    let effect = EventId::new("e2");
    let link = CausalLink {
        id: CausalLinkId::random(),
        cause: cause.clone(),
        effect: effect.clone(),
        strength: CausalStrength::Strong,
        explanation: "because".into(),
        confidence: Confidence::HIGH,
        probability: 0.8,
        properties: Default::default(),
    };
    model.add_link(link);
    assert_eq!(model.effects_of(&cause).len(), 1);
    assert_eq!(model.causes_of(&effect).len(), 1);
}

// ── confidence.rs ───────────────────────────────────────────────────────────

#[test]
fn confidence_accumulator_bayes() {
    let mut acc = ConfidenceAccumulator::new(0.5);
    let mut evidence = Evidence::new("test evidence", "test_source");
    evidence.weight = 3.0;
    evidence.source_reliability = 1.0;
    acc.add_evidence(evidence);
    let posterior = acc.posterior();
    assert!(posterior.0 > 0.5);
    assert!(posterior.0 <= 1.0);
}

#[test]
fn apply_confidence_decay() {
    let original = 1.0f32;
    let half_life = 3600.0;
    let after_one_hour = apply_decay(original, 3600.0, half_life);
    assert!((after_one_hour - 0.5).abs() < 0.01);
    let after_zero = apply_decay(original, 0.0, half_life);
    assert!((after_zero - 1.0).abs() < 0.01);
}

#[test]
fn merge_confidences_weighted() {
    let scores = [(0.8f32, 1.0f32), (0.6f32, 0.5f32)];
    let merged = merge_confidences(&scores);
    assert!(merged.0 > 0.5);
    assert!(merged.0 < 1.0);
}

// ── observation.rs ──────────────────────────────────────────────────────────

#[test]
fn observation_creation() {
    let obs = Observation::new("test content", ObservationType::StateReport, ObservationSource::Conversation);
    assert_eq!(obs.content, "test content");
    assert_eq!(obs.observation_type, ObservationType::StateReport);
}

#[test]
fn observation_type_display() {
    assert_eq!(ObservationType::StateReport.to_string(), "state_report");
    assert_eq!(ObservationType::EntityUpdate.to_string(), "entity_update");
}

// ── perception.rs ───────────────────────────────────────────────────────────

#[test]
fn perception_from_observation() {
    let obs = Observation::new("Alice is at office", ObservationType::EntityUpdate, ObservationSource::Sensor);
    let perception = Perception::from_observation(&obs);
    assert_eq!(perception.content, "Alice is at office");
    assert_eq!(perception.fused_from.len(), 1);
}

#[test]
fn perception_processor_entity_extraction() {
    let mentions = PerceptionProcessor::extract_entity_mentions("@Alice @Bob met today");
    assert_eq!(mentions.len(), 2);
    assert!(mentions.contains(&"Alice".to_string()));
    assert!(mentions.contains(&"Bob".to_string()));
}

#[test]
fn perception_processor_location_extraction() {
    let mentions = PerceptionProcessor::extract_location_mentions("meeting at #office and #lab");
    assert_eq!(mentions.len(), 2);
    assert!(mentions.contains(&"office".to_string()));
}

#[test]
fn perception_buffer_add_and_pending() {
    let mut buffer = PerceptionBuffer::new(10);
    let p = Perception {
        id: PerceptionId::random(),
        content: "test".into(),
        location: None,
        entities: vec![],
        relationships: vec![],
        events: vec![],
        properties: Default::default(),
        source: ObservationSource::Conversation,
        confidence: Confidence::MEDIUM,
        observed_at: chrono::Utc::now(),
        recorded_at: chrono::Utc::now(),
        raw_data: None,
        fused_from: vec![],
    };
    buffer.add(p);
    assert_eq!(buffer.pending_count(), 1);
    buffer.clear_processed();
    assert_eq!(buffer.pending_count(), 0);
}

// ── prediction.rs ───────────────────────────────────────────────────────────

#[test]
fn prediction_engine_predict_and_get() {
    let engine = PredictionEngine::new();
    let id = engine.predict("Task will complete", PredictionType::TaskCompletion, Confidence::HIGH, "because of reasons");
    assert!(engine.get(&id).is_some());
}

#[test]
fn prediction_engine_record_outcome() {
    let engine = PredictionEngine::new();
    let id = engine.predict("Test prediction", PredictionType::FutureState, Confidence::MEDIUM, "reasoning");
    assert!(engine.record_outcome(&id, "it happened", true));
    let pred = engine.get(&id).unwrap();
    assert_eq!(pred.was_correct, Some(true));
}

#[test]
fn prediction_engine_accuracy() {
    let engine = PredictionEngine::new();
    let id1 = engine.predict("p1", PredictionType::TaskCompletion, Confidence::HIGH, "r");
    let id2 = engine.predict("p2", PredictionType::TaskCompletion, Confidence::HIGH, "r");
    engine.record_outcome(&id1, "yes", true);
    engine.record_outcome(&id2, "no", false);
    let accuracy = engine.accuracy();
    assert!((accuracy - 0.5).abs() < 0.01);
}

// ── simulation.rs ───────────────────────────────────────────────────────────

#[test]
fn simulation_engine_run() {
    let engine = SimulationEngine::new(10);
    let scenario = SimulationScenario {
        name: "test sim".into(),
        description: "test".into(),
        initial_state: serde_json::json!({"count": 0}),
        actions: vec![SimulationAction {
            action_type: "increment".into(),
            target: None,
            parameters: serde_json::json!({}),
        }],
        expected_outcome: None,
    };
    let world_state = serde_json::json!({"count": 0});
    let result = engine.run(&scenario, &world_state);
    assert!(result.is_ok());
    let sim_id = result.unwrap();
    assert!(engine.get_result(&sim_id).is_some());
}

#[test]
fn simulation_engine_max_concurrent() {
    let engine = SimulationEngine::new(0);
    let scenario = SimulationScenario {
        name: "test".into(),
        description: "test".into(),
        initial_state: serde_json::json!({}),
        actions: vec![],
        expected_outcome: None,
    };
    let result = engine.run(&scenario, &serde_json::json!({}));
    assert!(result.is_err());
}

// ── uncertainty.rs ──────────────────────────────────────────────────────────

#[test]
fn uncertainty_tracker_register_and_resolve() {
    let mut tracker = UncertaintyTracker::new();
    tracker.register("u1", "missing data", UncertaintyCategory::MissingData);
    assert_eq!(tracker.unresolved_count(), 1);
    assert!(tracker.resolve("u1", "data found"));
    assert_eq!(tracker.unresolved_count(), 0);
}

// ── history.rs ──────────────────────────────────────────────────────────────

#[test]
fn history_manager_record_and_query() {
    let mut manager = HistoryManager::new(100);
    let entry = HistoryEntry::new(HistoryEntryType::EntityCreated, "created Alice", WorldVersion(1));
    manager.record(entry);
    assert_eq!(manager.recent(10).len(), 1);
    let by_type = manager.by_type(&HistoryEntryType::EntityCreated);
    assert_eq!(by_type.len(), 1);
}

#[test]
fn history_manager_max_entries() {
    let mut manager = HistoryManager::new(3);
    for i in 0..5 {
        manager.record(HistoryEntry::new(
            HistoryEntryType::EntityCreated,
            format!("entry {i}"),
            WorldVersion(i),
        ));
    }
    assert_eq!(manager.recent(100).len(), 3);
}

// ── environment.rs ──────────────────────────────────────────────────────────

#[test]
fn environment_manager_add_and_find() {
    let mgr = EnvironmentManager::new();
    let env = Environment::new("Office", EnvironmentType::Physical);
    let id = mgr.add(env);
    assert!(mgr.get(&id).is_some());
    let found = mgr.find_by_name("office");
    assert_eq!(found.len(), 1);
}

#[test]
fn environment_entity_add_remove() {
    let mut env = Environment::new("Test", EnvironmentType::Digital);
    let eid = EntityId::new("e1");
    env.add_entity(eid.clone());
    assert_eq!(env.entities.len(), 1);
    env.remove_entity(&eid);
    assert_eq!(env.entities.len(), 0);
}

// ── ontology.rs ─────────────────────────────────────────────────────────────

#[test]
fn ontology_registry_defaults() {
    let registry = EntityTypeRegistry::with_defaults();
    assert!(registry.is_registered(&EntityType::Human));
    assert!(registry.is_registered(&EntityType::Agent));
    assert!(registry.is_registered(&EntityType::Tool));
    assert!(registry.count() >= 27);
}

// ── lifecycle.rs ────────────────────────────────────────────────────────────

#[test]
fn lifecycle_valid_transitions() {
    assert!(is_valid_transition(&EntityState::Created, &EntityState::Active));
    assert!(is_valid_transition(&EntityState::Active, &EntityState::Suspended));
    assert!(is_valid_transition(&EntityState::Active, &EntityState::Deleted));
    assert!(!is_valid_transition(&EntityState::Created, &EntityState::Suspended));
    assert!(!is_valid_transition(&EntityState::Deleted, &EntityState::Active));
}

#[test]
fn lifecycle_manager_record_and_query() {
    let mgr = LifecycleManager::new();
    let event = LifecycleEvent::new(
        EntityState::Created,
        EntityState::Active,
        "activated",
        WorldVersion(1),
    );
    mgr.record_transition("entity-1", event);
    assert_eq!(mgr.history("entity-1").len(), 1);
    assert_eq!(mgr.transition_count("entity-1"), 1);
}

// ── state.rs ────────────────────────────────────────────────────────────────

#[test]
fn world_state_manager_advance_and_snapshot() {
    let mut mgr = WorldStateManager::new(10);
    let v1 = mgr.advance();
    assert_eq!(v1, WorldVersion(1));
    let snap = mgr.snapshot("initial state");
    assert_eq!(snap.version, WorldVersion(2));
    assert_eq!(mgr.latest_snapshot().unwrap().version, WorldVersion(2));
}

// ── metrics.rs ──────────────────────────────────────────────────────────────

#[test]
fn world_metrics_snapshot() {
    let metrics = WorldMetrics::default();
    metrics.entities_created.fetch_add(5, std::sync::atomic::Ordering::Relaxed);
    metrics.record_query(100);
    let snap = metrics.snapshot();
    assert_eq!(snap.entities_created, 5);
    assert_eq!(snap.queries_processed, 1);
    assert_eq!(metrics.average_query_time_ms(), 100);
}

// ── config.rs ───────────────────────────────────────────────────────────────

#[test]
fn world_config_defaults() {
    let config = WorldConfig::default();
    assert_eq!(config.max_entities, 100_000);
    assert_eq!(config.max_relationships, 500_000);
    assert!(config.enable_spatial);
    assert!(config.enable_temporal);
    assert!(!config.enable_distributed);
}

// ── error.rs ────────────────────────────────────────────────────────────────

#[test]
fn world_error_display() {
    let err = WorldError::EntityNotFound("abc".into());
    assert!(err.to_string().contains("abc"));
    let err = WorldError::VersionMismatch { expected: 1, actual: 2 };
    assert!(err.to_string().contains("1"));
    assert!(err.to_string().contains("2"));
}

// ── persistence.rs ──────────────────────────────────────────────────────────

#[test]
fn persistence_disabled_is_noop() {
    let mgr = PersistenceManager::new(None);
    assert!(!mgr.is_enabled());
    let snap = WorldSnapshot::new(WorldVersion(1), "test");
    assert!(mgr.save_snapshot(&snap).is_ok());
    assert!(mgr.load_latest_snapshot().unwrap().is_none());
}

// ── distributed.rs ──────────────────────────────────────────────────────────

#[test]
fn distributed_manager_nodes() {
    let mgr = DistributedManager::new("local-node");
    assert_eq!(mgr.local_node_id(), "local-node");
    assert_eq!(mgr.node_count(), 0);
    let node = DistributedNode {
        node_id: "node-1".into(),
        address: "127.0.0.1:8080".into(),
        region: "us-east".into(),
        current_version: WorldVersion(1),
        last_heartbeat: chrono::Utc::now(),
        is_healthy: true,
        capabilities: vec![],
        metadata: Default::default(),
    };
    mgr.register_node(node);
    assert_eq!(mgr.node_count(), 1);
    assert_eq!(mgr.healthy_nodes().len(), 1);
}

// ── synchronization.rs ─────────────────────────────────────────────────────

#[test]
fn synchronization_manager_peers() {
    let mgr = SynchronizationManager::new();
    mgr.register_peer("peer-1");
    assert_eq!(mgr.online_peers().len(), 1);
    mgr.peer_offline("peer-1");
    assert_eq!(mgr.online_peers().len(), 0);
}

// ── builders.rs ─────────────────────────────────────────────────────────────

#[test]
fn entity_builder_fluent() {
    let entity = EntityBuilder::new("Agent-1", EntityType::Agent)
        .state(EntityState::Active)
        .confidence(Confidence::HIGH)
        .tag("primary")
        .attribute("speed", AttributeValue::Float(1.5))
        .source("test-system")
        .build();
    assert_eq!(entity.name, "Agent-1");
    assert_eq!(entity.state, EntityState::Active);
    assert_eq!(entity.confidence, Confidence::HIGH);
    assert!(entity.tags.contains(&"primary".to_string()));
    assert_eq!(entity.source_system, "test-system");
}

#[test]
fn relationship_builder_fluent() {
    let src = EntityId::new("a");
    let tgt = EntityId::new("b");
    let rel = RelationshipBuilder::new(src, tgt, RelationshipType::ParentOf)
        .strength(RelationshipStrength::Strong)
        .confidence(Confidence::HIGH)
        .source("test")
        .build();
    assert_eq!(rel.strength, RelationshipStrength::Strong);
    assert_eq!(rel.confidence, Confidence::HIGH);
    assert_eq!(rel.source_system, "test");
}

// ── integration.rs ──────────────────────────────────────────────────────────

#[test]
fn integration_reasoning_context_empty() {
    let ctx = IntegrationLayer::reasoning_context(&[], &[]);
    assert!(ctx.contains("No entities"));
}

#[test]
fn integration_reasoning_context_with_entities() {
    let ids = vec![EntityId::new("a"), EntityId::new("b")];
    let descs = vec!["Alice".into(), "Bob".into()];
    let ctx = IntegrationLayer::reasoning_context(&ids, &descs);
    assert!(ctx.contains("Alice"));
    assert!(ctx.contains("Bob"));
}

#[test]
fn integration_conversation_context() {
    let ctx = IntegrationLayer::conversation_context(&["Alice".into()], &["spoke".into()]);
    assert!(ctx.contains("Alice"));
    assert!(ctx.contains("spoke"));
}

#[test]
fn integration_planning_context_empty() {
    let ctx = IntegrationLayer::planning_context(&[], "");
    assert!(ctx.contains("No planning context"));
}

#[test]
fn integration_memory_context() {
    let ctx = IntegrationLayer::memory_context(&["changed X".into()], 5);
    assert!(ctx.contains("v5"));
    assert!(ctx.contains("changed X"));
}

// ── types.rs serialization ──────────────────────────────────────────────────

#[test]
fn entity_type_serialization_roundtrip() {
    let et = EntityType::Agent;
    let json = serde_json::to_string(&et).unwrap();
    let back: EntityType = serde_json::from_str(&json).unwrap();
    assert_eq!(et, back);
}

#[test]
fn confidence_serialization_roundtrip() {
    let c = Confidence::HIGH;
    let json = serde_json::to_string(&c).unwrap();
    let back: Confidence = serde_json::from_str(&json).unwrap();
    assert_eq!(c, back);
}

#[test]
fn world_version_serialization_roundtrip() {
    let v = WorldVersion(42);
    let json = serde_json::to_string(&v).unwrap();
    let back: WorldVersion = serde_json::from_str(&json).unwrap();
    assert_eq!(v, back);
}

#[test]
fn reference_frame_display() {
    assert_eq!(ReferenceFrame::Global.to_string(), "global");
    assert_eq!(
        ReferenceFrame::Grid { cell_size: 1.0 }.to_string(),
        "grid:1"
    );
}

#[test]
fn history_entry_type_display() {
    assert_eq!(HistoryEntryType::EntityCreated.to_string(), "entity_created");
    assert_eq!(HistoryEntryType::SimulationRun.to_string(), "simulation_run");
}

#[test]
fn observation_source_display() {
    assert_eq!(ObservationSource::Conversation.to_string(), "conversation");
    assert_eq!(ObservationSource::Sensor.to_string(), "sensor");
}

#[test]
fn prediction_type_display() {
    assert_eq!(PredictionType::NextAction.to_string(), "next_action");
    assert_eq!(PredictionType::SystemFailure.to_string(), "system_failure");
}

#[test]
fn simulation_state_display() {
    assert_eq!(SimulationState::Created.to_string(), "created");
    assert_eq!(SimulationState::Completed.to_string(), "completed");
}

#[test]
fn environment_type_display() {
    assert_eq!(EnvironmentType::Physical.to_string(), "physical");
    assert_eq!(EnvironmentType::Digital.to_string(), "digital");
}
