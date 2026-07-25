# Graph Traversal Algorithms

## Overview

The reasoning module provides graph traversal, pathfinding, similarity computation, and subgraph extraction algorithms. All algorithms operate on the in-memory `GraphStore`.

## NeighborExpander

BFS-based neighborhood expansion.

### expand (Full Expansion)

Returns entities and relations within a given depth, traversing both incoming and outgoing edges.

```rust
let expander = NeighborExpander::new();

let (entities, relations) = expander.expand(
    &store,
    start_id,
    3,                              // depth
    None,                           // edge filter (all types)
    // Some(&[RelationType::RelatedTo])  // or filter by type
);

println!("Found {} entities, {} relations", entities.len(), relations.len());
```

**Algorithm:** BFS with visited sets for both entities and relations. Traverses outgoing edges first, then incoming. Optional `RelationType` filter. Depth is exclusive (depth=1 returns direct neighbors only).

### n_hop_neighbors

Returns all entity IDs reachable within N hops (excluding the start node).

```rust
let neighbors = expander.n_hop_neighbors(&store, start_id, 2);
// Returns all entities within 2 hops
```

**Algorithm:** Iterative BFS level-by-level. Maintains `visited` set across all levels. Returns the union of all discovered nodes (not just the last hop).

## PathSearcher

Pathfinding algorithms between two entities.

```rust
let searcher = PathSearcher::new();
```

### shortest_path (BFS)

Finds the path with fewest hops.

```rust
let result = searcher.shortest_path(&store, from_id, to_id);
// result.found: bool
// result.path: Vec<EntityId>
// result.hops: usize
// result.total_weight: f32
```

**Algorithm:** Standard BFS. Returns as soon as the target is reached. Path reconstructed via parent map.

### weighted_path (Dijkstra-like)

Finds the path minimizing total edge weight.

```rust
let result = searcher.weighted_path(&store, from_id, to_id);
```

**Algorithm:** BFS-based approximation using `VecDeque` as a priority queue. Processes lower-weight paths first. Not a true priority queue implementation, but effective for small graphs with uniform-ish weights.

### all_paths

Enumerates all paths within a depth limit.

```rust
let paths = searcher.all_paths(&store, from_id, to_id, 5);
// Vec<Vec<EntityId>>
```

**Algorithm:** DFS with backtracking. Prevents revisiting nodes within a single path. Capped at `max_depth`.

### SearchResult

```rust
pub struct SearchResult {
    pub path: Vec<EntityId>,
    pub total_weight: f32,
    pub hops: usize,
    pub found: bool,
}
```

Use `SearchResult::not_found()` for sentinel values.

## SemanticSimilarityEngine

Computes similarity between entities using multiple signals.

```rust
let engine = SemanticSimilarityEngine::new();
```

### Similarity Metrics

| Method | Algorithm | Range |
|--------|-----------|-------|
| `neighbor_similarity(a, b)` | Jaccard index on neighbor sets | [0.0, 1.0] |
| `label_similarity(a, b)` | Jaccard index on lowercased label words | [0.0, 1.0] |
| `combined_similarity(a, b)` | Weighted: 40% neighbor + 35% label + 25% type | [0.0, 1.0] |

**Type similarity:** 1.0 if same EntityType, 0.0 otherwise.

### find_similar

Linear scan over all active entities, ranked by combined similarity.

```rust
let similar = engine.find_similar(&store, &query_entity, 10);
// Vec<(Entity, f32)> -- top 10 most similar
```

## GraphTraversal

Configurable BFS and DFS traversals.

```rust
let traversal = GraphTraversal::new();

let config = TraversalConfig {
    max_depth: 5,
    max_results: 100,
    edge_filter: None,                    // or Some(vec![RelationType::RelatedTo])
    node_type_filter: None,               // or Some(vec!["person".to_string()])
};

let result = traversal.bfs(&store, start_id, &config);
let result = traversal.dfs(&store, start_id, &config);
```

### TraversalResult

```rust
pub struct TraversalResult {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
    pub paths: Vec<Vec<EntityId>>,
    pub visit_order: Vec<EntityId>,
}
```

### BFS vs DFS

| Aspect | BFS | DFS |
|--------|-----|-----|
| Edge direction | Both (outgoing + incoming) | Outgoing only |
| Order | Level-by-level | Depth-first |
| Uses | Shortest path, full exploration | Deep path discovery |

Both respect `edge_filter`, `node_type_filter`, `max_depth`, and `max_results`.

## SubgraphExtractor

Extracts subgraphs based on various criteria.

```rust
let extractor = SubgraphExtractor::new();
```

### ego_network

Multi-seed BFS subgraph extraction.

```rust
let (entities, relations) = extractor.ego_network(
    &store,
    vec![seed1, seed2],  // seed nodes
    2                     // radius (hop count)
);
```

### by_entity_types

Filters the graph to only include specified entity types, keeping only relations between matching entities.

```rust
let (entities, relations) = extractor.by_entity_types(
    &store,
    &[EntityType::Person, EntityType::Organization]
);
```

### connected_component

Extracts the full connected component containing a seed node.

```rust
let (entities, relations) = extractor.connected_component(&store, seed_id);
```

**Algorithm:** BFS from seed, both directions. Returns all reachable entities and their interconnecting relations.

## Analytics

### CentralityAnalyzer

```rust
let analyzer = CentralityAnalyzer::new();

// Degree centrality: neighbors / (n - 1)
let degrees = analyzer.degree_centrality(&store);  // HashMap<EntityId, f32>

// Betweenness centrality: Brandes' algorithm (exact)
let betweenness = analyzer.betweenness_centrality(&store);

// Top-k by degree
let top = analyzer.top_by_degree(&store, 10);
// Vec<(EntityId, f32, String)> -- (id, centrality, label)
```

### CommunityDetector

Connected-component-based community detection.

```rust
let detector = CommunityDetector::new();
let communities = detector.detect(&store);
// Vec<Community> -- sorted by size descending
// Community { members: Vec<EntityId>, density: f32 }
```

**Density:** `actual_edges / max_possible_edges` where max = `n * (n-1) / 2`.

### ClusterAnalyzer

K-core decomposition for finding densely connected subgroups.

```rust
let analyzer = ClusterAnalyzer::new();
let cores = analyzer.find_k_cores(&store, 3);  // min_degree = 3
// Vec<Vec<EntityId>> -- each core is a set of nodes
```

### ConnectedComponentAnalyzer

```rust
let analyzer = ConnectedComponentAnalyzer::new();
let components = analyzer.find_components(&store);
// Vec<ComponentInfo> -- { size, is_largest, members }

analyzer.count(&store);                     // number of components
analyzer.largest_component_size(&store);    // size of largest
```

### DensityAnalyzer

```rust
let analyzer = DensityAnalyzer::new();
let stats = analyzer.analyze(&store);
// DensityStats { entity_count, relation_count, max_possible_relations,
//                density, avg_degree, max_degree, min_degree }
```

**Density:** `relations / (n * (n-1))` for directed graphs.

### GrowthTracker

Tracks graph growth over time.

```rust
let tracker = GrowthTracker::new();
tracker.record(100, 250);
tracker.record(150, 400);

tracker.growth_rate();    // entities per hour
tracker.current_count();  // Option<(usize, usize)>
```
