use std::sync::Arc;

use crate::bootstrap::NeoSystem;
use crate::cli::GraphAction;
use crate::error::CliResult;

pub async fn run(system: &Arc<NeoSystem>, action: &GraphAction) -> CliResult<()> {
    match action {
        GraphAction::Stats => {
            match &system.knowledge {
                Some(kg) => {
                    let metrics = kg.metrics();
                    println!("Knowledge Graph Statistics");
                    println!("==========================");
                    println!("  Entities:       {} ({} active)",
                        metrics.entity_count, metrics.active_entity_count);
                    println!("  Relations:      {} ({} active)",
                        metrics.relation_count, metrics.active_relation_count);
                    println!("  Namespaces:     {}", metrics.namespace_count);
                    println!("  Avg confidence: {:.3}", metrics.avg_entity_confidence);
                    println!("  Avg importance: {:.3}", metrics.avg_entity_importance);
                    println!("  Total queries:  {}", metrics.total_queries);
                    println!("  Avg latency:    {:.2}ms", metrics.avg_query_latency_ms);
                    println!("  Extractions:    {}", metrics.total_extractions);
                    println!("  Freshness:      {:.1}%", metrics.knowledge_freshness * 100.0);
                    println!("  Consistency:    {:.1}%", metrics.consistency_score * 100.0);
                }
                None => {
                    println!("Knowledge graph is not available.");
                }
            }
        }
        GraphAction::Entities => {
            match &system.knowledge {
                Some(kg) => {
                    let metrics = kg.metrics();
                    println!("Entities: {} total, {} active",
                        metrics.entity_count, metrics.active_entity_count);
                    println!("Relations: {} total, {} active",
                        metrics.relation_count, metrics.active_relation_count);
                }
                None => {
                    println!("Knowledge graph is not available.");
                }
            }
        }
        GraphAction::Search { query } => {
            match &system.knowledge {
                Some(kg) => {
                    let results = kg.search(query, 20);
                    if results.is_empty() {
                        println!("No results found for '{query}'.");
                    } else {
                        println!("Found {} results for '{}':", results.len(), query);
                        println!();
                        for (i, result) in results.iter().enumerate() {
                            println!("  {}. {} (score: {:.3})",
                                i + 1, result.label, result.score);
                            if !result.explanation.is_empty() {
                                println!("     {}", result.explanation);
                            }
                        }
                    }
                }
                None => {
                    println!("Knowledge graph is not available.");
                }
            }
        }
        GraphAction::Create { entity_type, label } => {
            match &system.knowledge {
                Some(kg) => {
                    let et = match entity_type.as_str() {
                        "person" => neo_knowledge_graph::EntityType::Person,
                        "place" => neo_knowledge_graph::EntityType::Place,
                        "organization" => neo_knowledge_graph::EntityType::Organization,
                        "object" => neo_knowledge_graph::EntityType::Object,
                        "event" => neo_knowledge_graph::EntityType::Event,
                        "concept" => neo_knowledge_graph::EntityType::Concept,
                        "task" => neo_knowledge_graph::EntityType::Task,
                        "goal" => neo_knowledge_graph::EntityType::Goal,
                        "skill" => neo_knowledge_graph::EntityType::Skill,
                        "project" => neo_knowledge_graph::EntityType::Project,
                        "document" => neo_knowledge_graph::EntityType::Document,
                        "idea" => neo_knowledge_graph::EntityType::Idea,
                        "rule" => neo_knowledge_graph::EntityType::Rule,
                        other => neo_knowledge_graph::EntityType::Custom(other.to_string()),
                    };
                    let entity = kg.create_entity(et, label.clone());
                    println!("Created entity: {} (id: {})", entity.label, entity.id);
                }
                None => {
                    println!("Knowledge graph is not available.");
                }
            }
        }
    }
    Ok(())
}
