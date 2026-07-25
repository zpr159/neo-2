# Neo Knowledge System - Architecture

## Overview

The Neo Knowledge System (`neo-knowledge-graph`) is a dynamic knowledge representation engine for the Neo AGI Operating System. It transforms raw memories into structured, reusable knowledge with a world model, providing storage, reasoning, search, validation, evolution, and inference integration capabilities.

## System Diagram

```text
+--------------------------------------------------------------------------+
|                     NeoKnowledgeGraph (Orchestrator)                      |
|  Central entry point. Composes all subsystems and exposes a unified API. |
+----------+----------+----------+----------+----------+-------------------+
| Extraction| Reasoning | Search   |Validation| Evolution| Inference Integ  |
| Concept   | Neighbor | Hybrid   | Source   | Concept  | KnowledgeAware   |
| Extractor | Expander | Search   | Tracker  | Merger   | Prompter         |
| Entity    | Path     | Keyword  | Evidence | Concept  | ContextEnricher  |
| Extractor | Searcher | Search   | Tracker  | Splitter | FactRetriever    |
| Relation  | Semantic | Metadata | Contra-  | Taxonomy | FactRanker       |
| Extractor | Similarity| Search  | diction  | Refiner  | PromptAssembler  |
| Duplicate | Graph    | Temporal | Conflict | Relation |                  |
| Merger    | Traversal| Search   | Resolver | Discovery|                  |
| Confidence| Subgraph | Confidence|        | Knowledge|                  |
| Estimator | Extractor| Ranker   |          | Pruner   |                  |
+----------+----------+----------+----------+----------+-------------------+
|                         World Model                                       |
|  Person, Place, Organization, Object, Event, Task, Goal, Skill, Project  |
+--------------------------------------------------------------------------+
|  Ontology System  |  Knowledge Storage  |  Graph Analytics               |
|  (types, taxonomy,|  (graph, indexes,   |  (centrality, community,       |
|   schema)         |   snapshots, comp.) |   cluster, density, growth)    |
+-------------------+---------------------+--------------------------------+
|  Security          |  Persistence         |  Monitoring                   |
|  (namespace perms, |  (SQLite, KV store,  |  (metrics, health,            |
|   access control,  |   distributed hooks) |   latency tracking)           |
|   encryption,      |                      |                               |
|   audit trail)     |                      |                               |
+-------------------+---------------------+--------------------------------+
```

## Core Types

### Identity

Every knowledge element has a unique identity:

| Type | Wrapper | Display Format |
|------|---------|---------------|
| `EntityId` | `Uuid` | `entity:{uuid}` |
| `RelationId` | `Uuid` | `relation:{uuid}` |
| `AttributeId` | `Uuid` | `attribute:{uuid}` |
| `KnowledgeId` | `Uuid` + `IdType` | `{type}:{uuid}` |

UUIDs are v4 (random). Newtype wrappers prevent accidental ID mixing at compile time.

### Entity

The fundamental unit of knowledge:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `EntityId` | Unique identifier |
| `entity_type` | `EntityType` | Semantic type (Person, Place, Organization, Object, Event, Concept, Task, Goal, Skill, Project, Document, Idea, Rule, Custom) |
| `label` | `String` | Human-readable name |
| `description` | `String` | Detailed description |
| `properties` | `HashMap<String, Value>` | Flexible schema for arbitrary key-value data |
| `aliases` | `Vec<String>` | Alternative names |
| `namespace` | `String` | Logical grouping (default: "default") |
| `confidence` | `f32` | Confidence score [0.0, 1.0] |
| `importance` | `f32` | Importance score [0.0, 1.0] |
| `sources` | `Vec<String>` | Source attributions |
| `created_at` | `DateTime<Utc>` | Creation timestamp |
| `updated_at` | `DateTime<Utc>` | Last modification timestamp |
| `version` | `u64` | Auto-incremented on every mutation |
| `active` | `bool` | Soft-delete flag (default: true) |

All mutation methods call `touch()`, which increments `version` and updates `updated_at`.

### Relation

Connects two entities with typed, directed (or undirected) edges:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `RelationId` | Unique identifier |
| `relation_type` | `RelationType` | Type (IsA, HasA, PartOf, RelatedTo, Causes, Enables, Prevents, DependsOn, LocatedAt, MemberOf, AuthorOf, CreatedBy, Uses, InheritsFrom, Implements, Contradicts, Supports, TemporallyFollows, SpatiallyNear, Custom) |
| `source` | `EntityId` | Source entity |
| `target` | `EntityId` | Target entity |
| `directedness` | `Directed | Undirected` | Edge directionality |
| `weight` | `f32` | Relationship weight (default: 1.0) |
| `confidence` | `f32` | Confidence score |
| `label` | `String` | Human-readable label |
| `properties` | `HashMap<String, Value>` | Flexible properties |

Built-in relation properties: `IsA`, `Contradicts`, `Supports`, `SpatiallyNear` are symmetric. `PartOf` is transitive.

### Namespaces

Logical grouping of knowledge:

| Type | Description |
|------|-------------|
| `KnowledgeNamespace` | Name, description, config (read_only, max_entities, max_relations, encrypted) |
| `NamespaceRegistry` | Manages namespaces. Auto-creates "default". Prevents removal of "default". |

### Versioning

| Type | Description |
|------|-------------|
| `VersionVector` | Global + per-namespace version counters |
| `VersionTracker` | Append-only history of `VersionedChange` records (max 1000) |
| `VersionedChange` | version, change_type (Created/Updated/Deleted/Merged/Split/Pruned/Relocated), description, actor, checksum |

## Storage Architecture

### GraphStore (In-Memory)

The primary storage engine uses `DashMap` for lock-free concurrent access:

```
GraphStore
+-- entities:       DashMap<EntityId, Entity>
+-- relations:      DashMap<RelationId, Relation>
+-- adjacency:      DashMap<NodeAdjKey, HashSet<RelationId>>     (outgoing)
+-- reverse_adj:    DashMap<NodeAdjKey, HashSet<RelationId>>     (incoming)
+-- label_index:    DashMap<String, HashSet<EntityId>>            (lowercased)
+-- type_index:     DashMap<String, HashSet<EntityId>>
+-- relation_type:  DashMap<String, HashSet<RelationId>>
+-- ontology:       Ontology
```

All indexes are maintained on insert/remove. Hard delete cascades through both adjacency lists and all indexes.

### SnapshotManager

Full graph state copies (clone-based). Configurable max snapshots (default: 10). FIFO eviction of oldest.

### GraphIndexes

Extended indexing with: label, type, relation_type, namespace, confidence (bucketed at 0.2 intervals), property (composite "key=value"), temporal (hour buckets).

### GraphCompressor

Candidate identification for compression based on: age >= min_age_days, importance <= threshold, no source attributions. Reports estimated space savings.

### IncrementalUpdater

Delta-based change tracking with `DeltaChange` records. Records entity/relation CRUD operations. Supports flush to clear pending changes.

### RecoveryManager

Step-based recovery plans with optional steps. Auto-marks plan as Completed when all required steps finish.

## Concurrency Model

| Component | Mechanism | Notes |
|-----------|-----------|-------|
| GraphStore | `DashMap` | Lock-free concurrent reads/writes |
| SnapshotManager | `parking_lot::RwLock` | Read-heavy workload |
| SourceTracker | `parking_lot::RwLock` | Append-only |
| EvidenceTracker | `parking_lot::RwLock` | Append-only |
| AuditTrail | `parking_lot::RwLock` | Append-only, bounded |
| KnowledgeMonitor | `AtomicU64` | Lock-free with CAS loop for f64 accumulation |
| NamespacePermissions | `parking_lot::RwLock` | Read-heavy |

## Error Handling

`KnowledgeError` provides 30 error codes (4000-4029) covering all subsystems. Implements `std::error::Error` and `Display`. Conversions from `serde_json::Error`, `std::io::Error`, and `neo_core::error::NeoError`.

```rust
pub type KnowledgeResult<T> = Result<T, KnowledgeError>;
```

## Design Principles

1. **Flexible Schema**: `HashMap<String, serde_json::Value>` on entities and relations allows arbitrary properties without schema migration.
2. **Soft Delete**: All entities and relations support deactivation without data loss.
3. **Provenance Tracking**: Sources and evidence are first-class concepts.
4. **No External ML Dependencies**: All extraction is pattern/keyword-based. The crate provides the knowledge structure that ML systems consume via the inference integration API.
5. **Token Estimation**: All prompt/context builders use `chars / 4` as a rough token estimate.
6. **Consistent Similarity**: Jaccard similarity on word sets is used across merger, similarity engine, and discovery modules.
