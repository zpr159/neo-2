use std::io::{Read, Write};

use crate::core::entity::Entity;
use crate::core::relation::Relation;
use crate::error::{KnowledgeError, KnowledgeResult};
use crate::storage::graph_store::GraphStore;

/// Export formats for the knowledge graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
}

/// Exports the knowledge graph to various formats.
pub struct GraphExporter;

impl GraphExporter {
    /// Create a new exporter.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Export the graph to JSON.
    #[must_use]
    pub fn to_json(store: &GraphStore) -> KnowledgeResult<String> {
        let data = JsonExport {
            entities: store.all_entities(),
            relations: store.all_relations(),
            entity_count: store.active_entity_count(),
            relation_count: store.active_relation_count(),
        };
        serde_json::to_string_pretty(&data).map_err(|e| KnowledgeError::SerializationError(e.to_string()))
    }

    /// Export entities to CSV format.
    #[must_use]
    pub fn entities_to_csv(entities: &[Entity]) -> String {
        let mut output = String::from("id,type,label,description,confidence,importance,namespace\n");
        for e in entities {
            output.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                e.id,
                e.entity_type,
                escape_csv(&e.label),
                escape_csv(&e.description),
                e.confidence,
                e.importance,
                e.namespace
            ));
        }
        output
    }
}

impl Default for GraphExporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Imports a knowledge graph from various formats.
pub struct GraphImporter;

impl GraphImporter {
    /// Create a new importer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Import from JSON.
    pub fn from_json(json: &str, store: &GraphStore) -> KnowledgeResult<ImportResult> {
        let data: JsonExport = serde_json::from_str(json)
            .map_err(|e| KnowledgeError::DeserializationError(e.to_string()))?;

        let mut entities_imported = 0;
        let mut relations_imported = 0;

        for entity in data.entities {
            store.insert_entity(entity);
            entities_imported += 1;
        }

        for relation in data.relations {
            store.insert_relation(relation);
            relations_imported += 1;
        }

        Ok(ImportResult {
            entities_imported,
            relations_imported,
        })
    }
}

impl Default for GraphImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of an import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub entities_imported: usize,
    pub relations_imported: usize,
}

/// JSON export format.
#[derive(serde::Serialize, serde::Deserialize)]
struct JsonExport {
    entities: Vec<Entity>,
    relations: Vec<Relation>,
    entity_count: usize,
    relation_count: usize,
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
