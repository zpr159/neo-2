# Storage Model

## Overview

The storage layer provides in-memory graph storage with indexing, snapshots, compression, incremental updates, and recovery. Persistence is handled by separate SQLite and KV store backends.

## GraphStore

The primary in-memory graph store using `DashMap` for lock-free concurrent access.

### Data Structures

```
GraphStore
+-- entities:       DashMap<EntityId, Entity>
+-- relations:      DashMap<RelationId, Relation>
+-- adjacency:      DashMap<NodeAdjKey, HashSet<RelationId>>     (outgoing edges)
+-- reverse_adj:    DashMap<NodeAdjKey, HashSet<RelationId>>     (incoming edges)
+-- label_index:    DashMap<String, HashSet<EntityId>>            (lowercased labels)
+-- type_index:     DashMap<String, HashSet<EntityId>>            (entity type as string)
+-- relation_type:  DashMap<String, HashSet<RelationId>>          (relation type as string)
+-- ontology:       Ontology
```

`NodeAdjKey` is a newtype around `EntityId` implementing `Hash + Eq`.

### Entity Operations

```rust
let store = GraphStore::default_ontology();

// Insert
let id = store.insert_entity(entity);

// Read
let entity = store.get_entity(id);  // Option<Entity>

// Update (in-place with closure)
store.update_entity(id, |e| {
    e.set_property("key".to_string(), serde_json::json!("value"));
})?;

// Soft delete
store.deactivate_entity(id)?;

// Hard delete (cascades to relations)
store.remove_entity(id)?;
```

### Relation Operations

```rust
// Insert
let id = store.insert_relation(relation);

// Upsert (update or insert)
store.upsert_relation(&relation)?;

// Read
let relation = store.get_relation(id);  // Option<Relation>

// Delete
store.remove_relation(id)?;
```

### Query Operations

```rust
// Neighbors (both directions)
let neighbors: Vec<EntityId> = store.neighbors(entity_id);

// Outgoing/incoming
let outgoing: Vec<Relation> = store.get_outgoing_relations(entity_id);
let incoming: Vec<Relation> = store.get_incoming_relations(entity_id);

// By label (case-insensitive)
let entities = store.find_entities_by_label("alice");

// By type
let people = store.find_entities_by_type(&EntityType::Person);

// By relation type
let is_a_rels = store.find_relations_by_type(&RelationType::IsA);

// Bulk
let all_entities = store.all_entities();       // active only
let all_relations = store.all_relations();     // active only
let count = store.entity_count();
let active = store.active_entity_count();
```

### Cascade Behavior

Hard delete of an entity cascades:
1. Removes all outgoing edges (from `adjacency`)
2. For each outgoing edge, removes it from `reverse_adjacency` of the target
3. Removes all incoming edges (from `reverse_adjacency`)
4. For each incoming edge, removes it from `adjacency` of the source
5. Cleans up `label_index` and `type_index`

## GraphIndexes

Extended indexing for fast multi-dimensional queries.

### Index Types

| Index | Key Format | Value |
|-------|-----------|-------|
| Label | lowercased label | `Vec<EntityId>` |
| Type | entity type string | `Vec<EntityId>` |
| RelationType | relation type string | `Vec<RelationId>` |
| Namespace | namespace string | `Vec<EntityId>` |
| Confidence | bucket (0.0, 0.2, 0.4, 0.6, 0.8, 1.0) | `Vec<EntityId>` |
| Property | `"key=value"` | `Vec<EntityId>` |
| Temporal | `"YYYY-MM-DDTHH"` | `Vec<EntityId>` |

### API

```rust
let indexes = GraphIndexes::new();
indexes.index_entity(id, "alice", "person", "default", 0.9);
indexes.index_relation(rel_id, "related_to");

let by_label = indexes.by_label("alice");
let by_type = indexes.by_type("person");
let stats = indexes.stats(IndexType::Label);
```

## SnapshotManager

Full graph state copies for rollback and checkpointing.

### Configuration

```rust
let config = SnapshotConfig {
    max_snapshots: 10,   // FIFO eviction
    compress: false,
};
let manager = SnapshotManager::new(config);
```

### Operations

```rust
// Create snapshot
let snapshot = manager.create_snapshot(entities, relations, "pre-migration");

// List snapshots
let list: Vec<(String, DateTime)> = manager.list();

// Restore
let (entities, relations) = manager.restore(&snapshot_id)?;

// Get latest
let latest = manager.latest();

// Delete
manager.delete(&snapshot_id);
```

### GraphSnapshot

```rust
pub struct GraphSnapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub entity_count: usize,
    pub relation_count: usize,
    pub description: String,
    pub size_bytes: u64,
}
```

Snapshots are full clones. When max_snapshots is exceeded, the oldest snapshot is evicted.

## GraphCompressor

Identifies entities eligible for compression/removal.

### Configuration

```rust
let config = CompressionConfig {
    min_age_days: 30,          // entity must be at least 30 days old
    max_importance: 0.3,       // importance must be <= 0.3
    max_access_count: 5,       // (reserved for future use)
    preserve_sources: true,    // entities with sources are preserved
};
let compressor = GraphCompressor::new(config);
```

### API

```rust
// Find candidates
let candidates = compressor.compression_candidates(&entities);
// Returns Vec<String> of entity IDs eligible for compression

// Report compression results
let result = compressor.compress(&candidate_ids, &entities);
// Returns CompressionResult with estimated bytes saved
```

## IncrementalUpdater

Delta-based change tracking for sync and replication.

```rust
let updater = IncrementalUpdater::new();

// Record changes (static factory methods)
updater.record_change(IncrementalUpdater::entity_created(&id, data));
updater.record_change(IncrementalUpdater::relation_updated(&id, data));

// Check pending
let count = updater.pending_count();
let changes = updater.pending_changes();

// Flush
updater.flush();  // clears pending, increments counter
```

### DeltaChange

```rust
pub struct DeltaChange {
    pub id: String,
    pub change_kind: DeltaChangeKind,  // EntityCreated|Updated|Removed, RelationCreated|Updated|Removed
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}
```

## RecoveryManager

Step-based recovery plans for crash recovery and migration.

```rust
let manager = RecoveryManager::new();

// Create a plan
let plan = manager.create_plan("database migration");

// Add steps
let step_idx = manager.add_step(plan.id, "backup existing data", false);  // required
let step_idx = manager.add_step(plan.id, "verify backup", true);          // optional

// Complete steps
manager.complete_step(plan.id, step_idx)?;
// Auto-marks plan as Completed when all required steps finish

// Query
let plan = manager.get_plan(plan_id);
let plans = manager.list_plans();
```

### RecoveryPlan

```rust
pub struct RecoveryPlan {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub steps: Vec<RecoveryStep>,
    pub status: RecoveryStatus,  // NotStarted|InProgress|Completed|Failed
    pub description: String,
}
```

## Persistence Backends

### SqliteStore

Relational persistence using SQLite.

```rust
let store = SqliteStore::open(Path::new("knowledge.db"))?;

// Save entire graph (transactional: DELETE all + INSERT all)
store.save_graph(&graph_store)?;

// Schema: entities table + relations table with indexes on type, label, namespace, source, target
```

Properties, aliases, and sources are serialized as JSON strings. Timestamps stored as RFC3339. Active flag stored as INTEGER (0/1).

### RocksDbStore

Key-value persistence using `sled` (pure-Rust embedded KV store).

```rust
let store = RocksDbStore::open(Path::new("knowledge.sled"))?;

store.put(b"key", b"value")?;
let value = store.get(b"key");       // Option<Vec<u8>>
store.remove(b"key")?;

let entries = store.scan_prefix(b"prefix:");
let count = store.count();
let size = store.size_on_disk();
```

Generic key-value interface; serialization of graph structures is left to the caller.
