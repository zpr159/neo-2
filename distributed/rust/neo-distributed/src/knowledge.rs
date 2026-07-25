//! Distributed knowledge graph — partitioning, replication, distributed
//! traversal, and distributed queries across cluster nodes.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{DistributedError, NeoResult};
use crate::types::NodeId;

// ---------------------------------------------------------------------------
// GraphEntity
// ---------------------------------------------------------------------------

/// An entity (node) in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEntity {
    pub id: Uuid,
    pub entity_type: String,
    pub name: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub partition: usize,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// GraphRelation
// ---------------------------------------------------------------------------

/// A relation (edge) in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelation {
    pub id: Uuid,
    pub from_entity: Uuid,
    pub to_entity: Uuid,
    pub relation_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub weight: f64,
    pub partition: usize,
    pub version: u64,
    pub created_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// GraphPartition
// ---------------------------------------------------------------------------

/// A partition of the distributed knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPartition {
    pub id: usize,
    pub primary_node: NodeId,
    pub replica_nodes: Vec<NodeId>,
    pub entity_count: usize,
    pub relation_count: usize,
}

// ---------------------------------------------------------------------------
// DistributedKnowledgeGraph
// ---------------------------------------------------------------------------

/// Distributed knowledge graph with partitioning and replication.
pub struct DistributedKnowledgeGraph {
    /// Entities keyed by ID.
    entities: DashMap<Uuid, GraphEntity>,
    /// Relations keyed by ID.
    relations: DashMap<Uuid, GraphRelation>,
    /// Entity index: name → IDs.
    by_name: DashMap<String, Vec<Uuid>>,
    /// Relation index: (from, type) → relation IDs.
    by_source: DashMap<(Uuid, String), Vec<Uuid>>,
    /// Partitions.
    partitions: RwLock<Vec<GraphPartition>>,
    /// Total reads/writes.
    reads: std::sync::atomic::AtomicU64,
    writes: std::sync::atomic::AtomicU64,
}

impl DistributedKnowledgeGraph {
    pub fn new() -> Self {
        tracing::info!("distributed knowledge graph created");
        Self {
            entities: DashMap::new(),
            relations: DashMap::new(),
            by_name: DashMap::new(),
            by_source: DashMap::new(),
            partitions: RwLock::new(Vec::new()),
            reads: std::sync::atomic::AtomicU64::new(0),
            writes: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // -- Entity operations --

    pub fn add_entity(&self, mut entity: GraphEntity) -> NeoResult<Uuid> {
        let id = entity.id;
        self.by_name
            .entry(entity.name.clone())
            .or_default()
            .push(id);
        self.entities.insert(id, entity);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(id)
    }

    pub fn get_entity(&self, id: Uuid) -> Option<GraphEntity> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.entities.get(&id).map(|r| r.value().clone())
    }

    pub fn find_entities_by_name(&self, name: &str) -> Vec<GraphEntity> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.by_name
            .get(name)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.entities.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn update_entity(&self, id: Uuid, properties: HashMap<String, serde_json::Value>) -> NeoResult<()> {
        let mut entity = self
            .entities
            .get_mut(&id)
            .ok_or_else(|| DistributedError::node(format!("entity not found: {id}")))?;
        entity.properties.extend(properties);
        entity.modified_at = Utc::now();
        entity.version += 1;
        self.writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn remove_entity(&self, id: Uuid) -> bool {
        self.entities.remove(&id).is_some()
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // -- Relation operations --

    pub fn add_relation(&self, relation: GraphRelation) -> NeoResult<Uuid> {
        let id = relation.id;
        let key = (relation.from_entity, relation.relation_type.clone());
        self.by_source.entry(key).or_default().push(id);
        self.relations.insert(id, relation);
        self.writes.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(id)
    }

    pub fn get_relation(&self, id: Uuid) -> Option<GraphRelation> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.relations.get(&id).map(|r| r.value().clone())
    }

    pub fn outgoing_relations(&self, entity_id: Uuid, relation_type: &str) -> Vec<GraphRelation> {
        self.reads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let key = (entity_id, relation_type.to_string());
        self.by_source
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.relations.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    // -- Partitioning --

    pub fn create_partitions(&self, nodes: Vec<NodeId>) {
        let partitions: Vec<GraphPartition> = nodes
            .iter()
            .enumerate()
            .map(|(i, &node_id)| GraphPartition {
                id: i,
                primary_node: node_id,
                replica_nodes: Vec::new(),
                entity_count: 0,
                relation_count: 0,
            })
            .collect();
        *self.partitions.write() = partitions;
    }

    pub fn partitions(&self) -> Vec<GraphPartition> {
        self.partitions.read().clone()
    }

    // -- Traversal --

    pub fn traverse(
        &self,
        start_id: Uuid,
        relation_type: &str,
        depth: usize,
    ) -> Vec<(GraphEntity, GraphRelation)> {
        let mut results = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut frontier = vec![start_id];

        for _ in 0..depth {
            let mut next_frontier = Vec::new();
            for entity_id in frontier {
                if visited.contains(&entity_id) {
                    continue;
                }
                visited.insert(entity_id);

                let outgoing = self.outgoing_relations(entity_id, relation_type);
                for relation in outgoing {
                    if let Some(target) = self.get_entity(relation.to_entity) {
                        results.push((target.clone(), relation.clone()));
                        next_frontier.push(relation.to_entity);
                    }
                }
            }
            frontier = next_frontier;
        }

        results
    }

    // -- Statistics --

    pub fn stats(&self) -> KnowledgeGraphStats {
        KnowledgeGraphStats {
            entities: self.entity_count(),
            relations: self.relation_count(),
            partitions: self.partitions.read().len(),
            reads: self.reads.load(std::sync::atomic::Ordering::Relaxed),
            writes: self.writes.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl Default for DistributedKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphStats {
    pub entities: usize,
    pub relations: usize,
    pub partitions: usize,
    pub reads: u64,
    pub writes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get_entity() {
        let kg = DistributedKnowledgeGraph::new();
        let entity = GraphEntity {
            id: Uuid::new_v4(),
            entity_type: "concept".to_string(),
            name: "rust".to_string(),
            properties: HashMap::new(),
            partition: 0,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let id = entity.id;
        kg.add_entity(entity).unwrap();
        assert!(kg.get_entity(id).is_some());
        assert_eq!(kg.entity_count(), 1);
    }

    #[test]
    fn find_by_name() {
        let kg = DistributedKnowledgeGraph::new();
        let entity = GraphEntity {
            id: Uuid::new_v4(),
            entity_type: "concept".to_string(),
            name: "ai".to_string(),
            properties: HashMap::new(),
            partition: 0,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        kg.add_entity(entity).unwrap();
        let found = kg.find_entities_by_name("ai");
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn add_relation() {
        let kg = DistributedKnowledgeGraph::new();
        let e1 = Uuid::new_v4();
        let e2 = Uuid::new_v4();
        let relation = GraphRelation {
            id: Uuid::new_v4(),
            from_entity: e1,
            to_entity: e2,
            relation_type: "related_to".to_string(),
            properties: HashMap::new(),
            weight: 1.0,
            partition: 0,
            version: 1,
            created_at: Utc::now(),
        };
        kg.add_relation(relation).unwrap();
        assert_eq!(kg.relation_count(), 1);
    }

    #[test]
    fn traversal() {
        let kg = DistributedKnowledgeGraph::new();
        let e1 = GraphEntity {
            id: Uuid::new_v4(),
            entity_type: "concept".to_string(),
            name: "a".to_string(),
            properties: HashMap::new(),
            partition: 0,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let e2 = GraphEntity {
            id: Uuid::new_v4(),
            entity_type: "concept".to_string(),
            name: "b".to_string(),
            properties: HashMap::new(),
            partition: 0,
            version: 1,
            created_at: Utc::now(),
            modified_at: Utc::now(),
        };
        let e1_id = e1.id;
        let e2_id = e2.id;
        kg.add_entity(e1).unwrap();
        kg.add_entity(e2).unwrap();
        kg.add_relation(GraphRelation {
            id: Uuid::new_v4(),
            from_entity: e1_id,
            to_entity: e2_id,
            relation_type: "link".to_string(),
            properties: HashMap::new(),
            weight: 1.0,
            partition: 0,
            version: 1,
            created_at: Utc::now(),
        }).unwrap();

        let results = kg.traverse(e1_id, "link", 2);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn stats() {
        let kg = DistributedKnowledgeGraph::new();
        let stats = kg.stats();
        assert_eq!(stats.entities, 0);
    }
}
