# API Reference

## Overview

The knowledge graph exposes two layers of API: low-level `*Api` structs that directly wrap `GraphStore`, and the high-level `NeoKnowledgeGraph` orchestrator that composes all subsystems.

## NeoKnowledgeGraph (Orchestrator)

The central entry point. Composes all subsystems and provides a unified interface.

```rust
use neo_knowledge_graph::*;

let kg = NeoKnowledgeGraph::new();                    // default ontology
let kg = NeoKnowledgeGraph::with_ontology(ontology);  // custom ontology
```

### Entity CRUD

```rust
// Create
let alice = kg.create_entity(EntityType::Person, "Alice");

// Create with builder
let bob = kg.create_entity_with(
    Entity::builder(EntityType::Person, "Bob")
        .description("A software engineer")
        .confidence(0.9)
        .alias("Robert")
);

// Read
let entity = kg.get_entity(alice.id);  // Option<Entity>

// Update
kg.update_entity(alice.id, |e| {
    e.set_property("role".to_string(), serde_json::json!("engineer"));
})?;

// Soft delete
kg.delete_entity(alice.id)?;

// Hard delete (cascades)
kg.remove_entity(alice.id)?;
```

### Relation CRUD

```rust
// Create (validates both endpoints exist)
let rel = kg.create_relation(
    RelationType::RelatedTo,
    alice.id,
    bob.id,
    "colleague"
)?;

// Read
let rel = kg.get_relation(rel.id);  // Option<Relation>

// Remove
kg.remove_relation(rel.id)?;
```

### Search

```rust
// Hybrid search (keyword + similarity + confidence)
let results = kg.search("machine learning", 10);
// Vec<RankedResult> -- { entity_id, label, score, explanation }

// Keyword-only search
let results = kg.keyword_search("Alice");
// Vec<(Entity, f32)>

// Property search
let entities = kg.search_by_property("role", &serde_json::json!("engineer"));
```

### Traversal

```rust
// Expand neighbors
let neighbors = kg.expand_neighbors(alice.id, 2);
// Vec<EntityId> -- all reachable within 2 hops

// Shortest path
let path = kg.shortest_path(alice.id, charlie.id);
// SearchResult { path, hops, total_weight, found }

// BFS traversal
let result = kg.bfs(alice.id, TraversalConfig {
    max_depth: 5,
    max_results: 100,
    edge_filter: None,
    node_type_filter: None,
});
```

### Extraction

```rust
// Extract a concept from text
let concept = kg.extract_from_text(
    "Machine Learning is a subset of AI",
    "conversation"
);

// Extract entities from text
let entities = kg.extract_entities(
    "Dr. Smith works at Google",
    "conversation"
);
// Vec<ExtractedEntity>

// Merge duplicates
let merges = kg.merge_duplicates(0.7)?;
```

### Validation

```rust
// Detect contradictions
let contradictions = kg.detect_contradictions();

// Record provenance
kg.record_source(entity_id, "user_input", 0.9);

// Add evidence
kg.add_evidence(entity_id, "Confirmed by multiple sources", "verification", 0.85);
```

### Snapshots

```rust
let snapshot = kg.create_snapshot("before migration");
// GraphSnapshot { id, entity_count, relation_count, ... }

kg.restore_snapshot(&snapshot.id)?;
```

### Inference Integration

```rust
// Build a knowledge-aware prompt
let prompt = kg.build_prompt("What is Rust?", 4000);

// Retrieve facts
let facts = kg.retrieve_facts("Rust programming", 10);

// Enrich context
let enriched = kg.enrich_context("Rust programming", 5);
```

### World Model

```rust
let wm = kg.world_model();
let people = wm.people();      // Vec<PersonEntity>
let tasks = wm.tasks();        // Vec<TaskEntity>
let goals = wm.goals();        // Vec<GoalEntity>
let projects = wm.projects();  // Vec<ProjectEntity>
```

### Analytics

```rust
let density = kg.density_analysis();
// DensityStats { entity_count, relation_count, density, avg_degree, ... }

let centrality = kg.centrality_analysis();
// HashMap<EntityId, f32>

let components = kg.connected_components();
// Vec<ComponentInfo>
```

### Export / Import

```rust
let json = kg.export_json()?;
kg.import_json(&json)?;
```

### Persistence

```rust
kg.save_to_sqlite(Path::new("knowledge.db"))?;
```

### Metrics

```rust
let metrics = kg.metrics();
// KnowledgeMetrics { entity_count, relation_count, avg_entity_confidence, ... }
```

## Low-Level APIs

### EntityApi

```rust
let api = EntityApi::new(&store);
let entity = api.create(EntityType::Person, "Alice");
let entity = api.create_with(builder);
let entity = api.get(id);
api.update(id, |e| { ... })?;
api.delete(id)?;    // soft
api.remove(id)?;    // hard
let entities = api.search_by_label("alice");
let entities = api.search_by_type(&EntityType::Person);
let count = api.count();
```

### RelationApi

```rust
let api = RelationApi::new(&store);
let rel = api.create(RelationType::RelatedTo, source, target, "label")?;
let rel = api.get(id);
api.remove(id)?;
let outgoing = api.outgoing(entity_id);
let incoming = api.incoming(entity_id);
let by_type = api.by_type(&RelationType::IsA);
let count = api.count();
```

### SearchApi

```rust
let api = SearchApi::new(&store);
let results = api.search("query", 10);
let results = api.keyword_search("query");
let results = api.by_property("key", &value);
let results = api.by_namespace("default");
let results = api.by_confidence(0.5);
let results = api.recently_updated(3600);  // last hour
let results = api.created_today();
```

### TraverseApi

```rust
let api = TraverseApi::new(&store);
let neighbors = api.expand_neighbors(id, 2);
let path = api.shortest_path(from, to);
let path = api.weighted_path(from, to);
let paths = api.all_paths(from, to, 5);
let result = api.bfs(start, config);
let result = api.dfs(start, config);
```

### GraphExporter / GraphImporter

```rust
// Export
let json = GraphExporter::to_json(&store)?;
let csv = GraphExporter::entities_to_csv(&entities);

// Import
let result = GraphImporter::from_json(&json, &store)?;
// ImportResult { entities_imported, relations_imported }
```

## Validation Module

### SourceTracker

```rust
let tracker = SourceTracker::new();
tracker.record_source("entity_1", "user_input", 0.9);
let sources = tracker.get_sources("entity_1");  // Vec<&ProvenanceRecord>
let count = tracker.source_count("entity_1");    // distinct sources
```

### EvidenceTracker

```rust
let tracker = EvidenceTracker::new();
tracker.add_supporting("entity_1", "Confirmed by tests", "qa", 0.9);
tracker.add_contradicting("entity_1", "Contradicts report", "report", 0.7);
let score = tracker.net_score("entity_1");  // weighted net: [-1.0, 1.0]
```

### ContradictionDetector

```rust
let detector = ContradictionDetector::new();
let contradictions = detector.detect_entity_contradictions(&entities);
let contradictions = detector.detect_relation_contradictions(&relations);
let contradictions = detector.detect_all(&entities, &relations);
```

### ConflictResolver

```rust
let resolver = ConflictResolver::new(ResolutionStrategy::HighestConfidence);

let result = resolver.resolve_contradiction(&entity_a, &entity_b, None);
// ResolutionResult { strategy, kept: Vec<String>, removed: Vec<String>, description }

let results = resolver.resolve_all(&contradictions, &entities, None);
```

## Evolution Module

### ConceptMerger

```rust
let merger = ConceptMerger::new();
let outcome = merger.merge_pair(&entity_a, &entity_b);
let outcomes = merger.merge_duplicates(&store, 0.7)?;
```

### ConceptSplitter

```rust
let splitter = ConceptSplitter::new();
let outcome = splitter.split_by_labels(&original, "Part A", "Part B");
// Caller must insert new entities and deactivate original
```

### TaxonomyRefiner

```rust
let refiner = TaxonomyRefiner::new();
let suggestions = refiner.suggest_refinements(&store, &taxonomy);
// Vec<TaxonomySuggestion> -- { child_type, parent_type, confidence, reason }
```

### RelationshipDiscovery

```rust
let discovery = RelationshipDiscovery::new();
let discovered = discovery.discover_by_shared_neighbors(&store, 2, 0.5);
// Vec<DiscoveredRelation> -- { source, target, relation_type, confidence, reason }
```

### KnowledgePruner

```rust
let pruner = KnowledgePruner::new(PruningConfig {
    min_confidence: 0.1,
    min_importance: 0.1,
    preserve_connected: true,
});
let candidates = pruner.candidates(&entities, &store);
// Caller must remove candidates from store
```

## Security

### NamespacePermissions

```rust
let perms = NamespacePermissions::new();
perms.set("secret", PermissionLevel::Read);
assert!(perms.check("secret", PermissionLevel::Read));
assert!(!perms.check("secret", PermissionLevel::Write));
```

### AccessController

```rust
let controller = AccessController::new();
assert!(controller.can_read("default"));
assert!(controller.can_write("default"));
assert!(controller.can_admin("default"));
```

### GraphEncryption

```rust
let hash = GraphEncryption::checksum("hello");  // SHA-256 hex
```

### AuditTrail

```rust
let trail = AuditTrail::new(10_000);
trail.log(AuditAction::EntityCreate, "user", "entity_1", "default", true, None);
assert_eq!(trail.count(), 1);
let entries = trail.entries_for_target("entity_1");
```

## Error Handling

All fallible operations return `KnowledgeResult<T>`:

```rust
pub type KnowledgeResult<T> = Result<T, KnowledgeError>;
```

`KnowledgeError` has 30 error codes (4000-4029) and implements `std::error::Error` + `Display`. Common variants:

| Variant | Code | Description |
|---------|------|-------------|
| `EntityNotFound` | 4000 | Entity ID not in store |
| `RelationNotFound` | 4001 | Relation ID not in store |
| `StorageError` | 4002 | I/O or persistence error |
| `SerializationError` | 4003 | JSON serialization failure |
| `PermissionDenied` | 4010 | Namespace access denied |
| `InvalidEntity` | 4020 | Entity fails validation |
| `InvalidRelation` | 4021 | Relation fails validation |
| `NamespaceNotFound` | 4022 | Namespace doesn't exist |
| `SnapshotNotFound` | 4025 | Snapshot ID not found |
| `InternalError` | 4029 | Unexpected internal error |
