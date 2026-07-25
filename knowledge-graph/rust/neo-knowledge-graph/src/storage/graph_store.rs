use std::collections::HashSet;

use dashmap::DashMap;
use tracing::info;

use crate::core::entity::{Entity, EntityId, EntityType};
use crate::core::relation::{Relation, RelationId, RelationType};
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::ontology::types::Ontology;

/// Central in-memory graph store for the knowledge system.
#[derive(Debug)]
pub struct GraphStore {
    entities: DashMap<EntityId, Entity>,
    relations: DashMap<RelationId, Relation>,
    adjacency: DashMap<NodeAdjKey, HashSet<RelationId>>,
    reverse_adjacency: DashMap<NodeAdjKey, HashSet<RelationId>>,
    label_index: DashMap<String, HashSet<EntityId>>,
    type_index: DashMap<String, HashSet<EntityId>>,
    relation_type_index: DashMap<String, HashSet<RelationId>>,
    ontology: Ontology,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NodeAdjKey(EntityId);

impl GraphStore {
    /// Create a new graph store with the given ontology.
    #[must_use]
    pub fn new(ontology: Ontology) -> Self {
        Self {
            entities: DashMap::new(),
            relations: DashMap::new(),
            adjacency: DashMap::new(),
            reverse_adjacency: DashMap::new(),
            label_index: DashMap::new(),
            type_index: DashMap::new(),
            relation_type_index: DashMap::new(),
            ontology,
        }
    }

    /// Create a new graph store with default ontology.
    #[must_use]
    pub fn default_ontology() -> Self {
        Self::new(Ontology::default())
    }

    /// Get the ontology.
    #[must_use]
    pub fn ontology(&self) -> &Ontology {
        &self.ontology
    }

    /// Insert an entity into the graph.
    pub fn insert_entity(&self, entity: Entity) -> EntityId {
        let id = entity.id;
        let type_key = entity.entity_type.to_string();
        let label_key = entity.label.to_lowercase();

        self.adjacency.entry(NodeAdjKey(id)).or_default();
        self.reverse_adjacency.entry(NodeAdjKey(id)).or_default();
        self.entities.insert(id, entity);

        self.label_index
            .entry(label_key)
            .or_default()
            .insert(id);
        self.type_index
            .entry(type_key)
            .or_default()
            .insert(id);

        info!(entity_id = %id, "Inserted entity");
        id
    }

    /// Get an entity by id.
    #[must_use]
    pub fn get_entity(&self, id: EntityId) -> Option<Entity> {
        self.entities.get(&id).map(|e| e.value().clone())
    }

    /// Update an entity.
    pub fn update_entity(
        &self,
        id: EntityId,
        updater: impl FnOnce(&mut Entity),
    ) -> KnowledgeResult<()> {
        if let Some(mut entity) = self.entities.get_mut(&id) {
            updater(&mut entity);
            entity.touch();
            Ok(())
        } else {
            Err(KnowledgeError::EntityNotFound(id.to_string()))
        }
    }

    /// Deactivate (soft-delete) an entity.
    pub fn deactivate_entity(&self, id: EntityId) -> KnowledgeResult<()> {
        if let Some(mut entity) = self.entities.get_mut(&id) {
            entity.active = false;
            entity.touch();
            Ok(())
        } else {
            Err(KnowledgeError::EntityNotFound(id.to_string()))
        }
    }

    /// Remove an entity and all its connected relations.
    pub fn remove_entity(&self, id: EntityId) -> KnowledgeResult<bool> {
        if self.entities.remove(&id).is_none() {
            return Ok(false);
        }

        // Remove outgoing edges
        if let Some((_, out_edges)) = self.adjacency.remove(&NodeAdjKey(id)) {
            for edge_id in out_edges {
                if let Some((_, edge)) = self.relations.remove(&edge_id) {
                    if let Some(mut rev) = self.reverse_adjacency.get_mut(&NodeAdjKey(edge.target)) {
                        rev.remove(&edge_id);
                    }
                    let rt = edge.relation_type.to_string();
                    if let Some(mut idx) = self.relation_type_index.get_mut(&rt) {
                        idx.remove(&edge_id);
                    }
                }
            }
        }

        // Remove incoming edges
        if let Some((_, in_edges)) = self.reverse_adjacency.remove(&NodeAdjKey(id)) {
            for edge_id in in_edges {
                if let Some((_, edge)) = self.relations.remove(&edge_id) {
                    if let Some(mut fwd) = self.adjacency.get_mut(&NodeAdjKey(edge.source)) {
                        fwd.remove(&edge_id);
                    }
                    let rt = edge.relation_type.to_string();
                    if let Some(mut idx) = self.relation_type_index.get_mut(&rt) {
                        idx.remove(&edge_id);
                    }
                }
            }
        }

        // Clean up indexes
        if let Some(entity) = self.entities.get(&id) {
            let label_key = entity.label.to_lowercase();
            if let Some(mut idx) = self.label_index.get_mut(&label_key) {
                idx.remove(&id);
            }
            let type_key = entity.entity_type.to_string();
            if let Some(mut idx) = self.type_index.get_mut(&type_key) {
                idx.remove(&id);
            }
        }

        Ok(true)
    }

    /// Insert a relation into the graph.
    pub fn insert_relation(&self, relation: Relation) -> RelationId {
        let id = relation.id;
        self.adjacency
            .entry(NodeAdjKey(relation.source))
            .or_default()
            .insert(id);
        self.reverse_adjacency
            .entry(NodeAdjKey(relation.target))
            .or_default()
            .insert(id);

        let rt = relation.relation_type.to_string();
        self.relations.insert(id, relation);
        self.relation_type_index
            .entry(rt)
            .or_default()
            .insert(id);

        info!(relation_id = %id, "Inserted relation");
        id
    }

    /// Upsert a relation (update existing or insert new).
    pub fn upsert_relation(&self, relation: &Relation) -> KnowledgeResult<()> {
        if let Some(mut existing) = self.relations.get_mut(&relation.id) {
            *existing = relation.clone();
            existing.touch();
            Ok(())
        } else {
            self.insert_relation(relation.clone());
            Ok(())
        }
    }

    /// Get a relation by id.
    #[must_use]
    pub fn get_relation(&self, id: RelationId) -> Option<Relation> {
        self.relations.get(&id).map(|r| r.value().clone())
    }

    /// Remove a relation.
    pub fn remove_relation(&self, id: RelationId) -> KnowledgeResult<bool> {
        if let Some((_, edge)) = self.relations.remove(&id) {
            if let Some(mut fwd) = self.adjacency.get_mut(&NodeAdjKey(edge.source)) {
                fwd.remove(&id);
            }
            if let Some(mut rev) = self.reverse_adjacency.get_mut(&NodeAdjKey(edge.target)) {
                rev.remove(&id);
            }
            let rt = edge.relation_type.to_string();
            if let Some(mut idx) = self.relation_type_index.get_mut(&rt) {
                idx.remove(&id);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get outgoing relation ids for an entity.
    #[must_use]
    pub fn get_outgoing_relation_ids(&self, id: EntityId) -> Vec<RelationId> {
        self.adjacency
            .get(&NodeAdjKey(id))
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get incoming relation ids for an entity.
    #[must_use]
    pub fn get_incoming_relation_ids(&self, id: EntityId) -> Vec<RelationId> {
        self.reverse_adjacency
            .get(&NodeAdjKey(id))
            .map(|s| s.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get outgoing relations for an entity.
    #[must_use]
    pub fn get_outgoing_relations(&self, id: EntityId) -> Vec<Relation> {
        self.get_outgoing_relation_ids(id)
            .iter()
            .filter_map(|rid| self.get_relation(*rid))
            .collect()
    }

    /// Get incoming relations for an entity.
    #[must_use]
    pub fn get_incoming_relations(&self, id: EntityId) -> Vec<Relation> {
        self.get_incoming_relation_ids(id)
            .iter()
            .filter_map(|rid| self.get_relation(*rid))
            .collect()
    }

    /// Get all neighbor entity ids (both directions).
    #[must_use]
    pub fn neighbors(&self, id: EntityId) -> Vec<EntityId> {
        let mut result = HashSet::new();
        for rid in self.get_outgoing_relation_ids(id) {
            if let Some(rel) = self.get_relation(rid) {
                result.insert(rel.target);
            }
        }
        for rid in self.get_incoming_relation_ids(id) {
            if let Some(rel) = self.get_relation(rid) {
                result.insert(rel.source);
            }
        }
        result.into_iter().collect()
    }

    /// Find entities by label (case-insensitive).
    #[must_use]
    pub fn find_entities_by_label(&self, label: &str) -> Vec<Entity> {
        let key = label.to_lowercase();
        self.label_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_entity(*id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find entities by type.
    #[must_use]
    pub fn find_entities_by_type(&self, entity_type: &EntityType) -> Vec<Entity> {
        let key = entity_type.to_string();
        self.type_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_entity(*id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find relations by type.
    #[must_use]
    pub fn find_relations_by_type(&self, relation_type: &RelationType) -> Vec<Relation> {
        let key = relation_type.to_string();
        self.relation_type_index
            .get(&key)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.get_relation(*id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all active entities.
    #[must_use]
    pub fn all_entities(&self) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|e| e.active)
            .map(|e| e.value().clone())
            .collect()
    }

    /// Get all active relations.
    #[must_use]
    pub fn all_relations(&self) -> Vec<Relation> {
        self.relations
            .iter()
            .filter(|r| r.active)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Total entity count.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Total relation count.
    #[must_use]
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    /// Active entity count.
    #[must_use]
    pub fn active_entity_count(&self) -> usize {
        self.entities.iter().filter(|e| e.active).count()
    }

    /// Active relation count.
    #[must_use]
    pub fn active_relation_count(&self) -> usize {
        self.relations.iter().filter(|r| r.active).count()
    }

    /// Get all entity ids.
    #[must_use]
    pub fn all_entity_ids(&self) -> Vec<EntityId> {
        self.entities.iter().map(|e| *e.key()).collect()
    }

    /// Get all relation ids.
    #[must_use]
    pub fn all_relation_ids(&self) -> Vec<RelationId> {
        self.relations.iter().map(|r| *r.key()).collect()
    }
}
