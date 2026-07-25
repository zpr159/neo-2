use std::path::Path;

use rusqlite::{params, Connection};
use tracing::info;

use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::storage::graph_store::GraphStore;

/// SQLite-backed persistent storage for the knowledge graph.
pub struct SqliteStore {
    conn: parking_lot::Mutex<Connection>,
}

impl SqliteStore {
    /// Open or create a SQLite store at the given path.
    pub fn open(path: &Path) -> KnowledgeResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                label TEXT NOT NULL,
                description TEXT DEFAULT '',
                properties TEXT DEFAULT '{}',
                aliases TEXT DEFAULT '[]',
                namespace TEXT DEFAULT 'default',
                confidence REAL DEFAULT 1.0,
                importance REAL DEFAULT 0.5,
                sources TEXT DEFAULT '[]',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 0,
                active INTEGER DEFAULT 1
            );
            CREATE TABLE IF NOT EXISTS relations (
                id TEXT PRIMARY KEY,
                relation_type TEXT NOT NULL,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                directedness TEXT DEFAULT 'Directed',
                weight REAL DEFAULT 1.0,
                confidence REAL DEFAULT 1.0,
                properties TEXT DEFAULT '{}',
                label TEXT DEFAULT '',
                sources TEXT DEFAULT '[]',
                namespace TEXT DEFAULT 'default',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                version INTEGER DEFAULT 0,
                active INTEGER DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
            CREATE INDEX IF NOT EXISTS idx_entities_label ON entities(label);
            CREATE INDEX IF NOT EXISTS idx_entities_namespace ON entities(namespace);
            CREATE INDEX IF NOT EXISTS idx_relations_type ON relations(relation_type);
            CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source);
            CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target);
            ",
        )
        .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;

        info!("SQLite store opened at {}", path.display());
        Ok(Self {
            conn: parking_lot::Mutex::new(conn),
        })
    }

    /// Persist a graph store to SQLite.
    pub fn save_graph(&self, store: &GraphStore) -> KnowledgeResult<()> {
        let conn = self.conn.lock();

        let tx = conn
            .unchecked_transaction()
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;

        // Clear existing data
        tx.execute("DELETE FROM entities", [])
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        tx.execute("DELETE FROM relations", [])
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;

        // Insert entities
        let entities = store.all_entities();
        for entity in &entities {
            let props = serde_json::to_string(&entity.properties)
                .map_err(|e| KnowledgeError::SerializationError(e.to_string()))?;
            let aliases = serde_json::to_string(&entity.aliases)
                .map_err(|e| KnowledgeError::SerializationError(e.to_string()))?;
            let sources = serde_json::to_string(&entity.sources)
                .map_err(|e| KnowledgeError::SerializationError(e.to_string()))?;

            tx.execute(
                "INSERT INTO entities (id, entity_type, label, description, properties, aliases, namespace, confidence, importance, sources, created_at, updated_at, version, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    entity.id.to_string(),
                    entity.entity_type.to_string(),
                    entity.label,
                    entity.description,
                    props,
                    aliases,
                    entity.namespace,
                    entity.confidence,
                    entity.importance,
                    sources,
                    entity.created_at.to_rfc3339(),
                    entity.updated_at.to_rfc3339(),
                    entity.version,
                    entity.active as i32,
                ],
            )
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        }

        // Insert relations
        let relations = store.all_relations();
        for relation in &relations {
            let props = serde_json::to_string(&relation.properties)
                .map_err(|e| KnowledgeError::SerializationError(e.to_string()))?;
            let sources = serde_json::to_string(&relation.sources)
                .map_err(|e| KnowledgeError::SerializationError(e.to_string()))?;

            tx.execute(
                "INSERT INTO relations (id, relation_type, source, target, directedness, weight, confidence, properties, label, sources, namespace, created_at, updated_at, version, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    relation.id.to_string(),
                    relation.relation_type.to_string(),
                    relation.source.to_string(),
                    relation.target.to_string(),
                    format!("{:?}", relation.directedness),
                    relation.weight,
                    relation.confidence,
                    props,
                    relation.label,
                    sources,
                    relation.namespace,
                    relation.created_at.to_rfc3339(),
                    relation.updated_at.to_rfc3339(),
                    relation.version,
                    relation.active as i32,
                ],
            )
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;
        }

        tx.commit()
            .map_err(|e| KnowledgeError::StorageError(e.to_string()))?;

        info!(
            "Saved {} entities and {} relations to SQLite",
            entities.len(),
            relations.len()
        );
        Ok(())
    }
}
