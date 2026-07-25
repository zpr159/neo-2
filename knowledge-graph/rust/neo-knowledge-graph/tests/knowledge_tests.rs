use neo_knowledge_graph::*;
use std::collections::HashMap;

fn make_graph() -> NeoKnowledgeGraph {
    NeoKnowledgeGraph::new()
}

// ═══════════════════════════════════════
//  CORE TYPES TESTS
// ═══════════════════════════════════════

#[test]
fn entity_creation_and_properties() {
    let mut entity = Entity::new(EntityType::Person, "Alice".to_string());
    assert_eq!(entity.label, "Alice");
    assert!(entity.active);
    assert_eq!(entity.version, 0);

    entity.set_property("age".to_string(), serde_json::json!(30));
    assert_eq!(entity.get_property("age"), Some(&serde_json::json!(30)));
    assert_eq!(entity.version, 1);

    entity.add_alias("Ali".to_string());
    assert_eq!(entity.aliases.len(), 1);

    entity.add_source("user_input".to_string());
    assert_eq!(entity.sources.len(), 1);
}

#[test]
fn entity_builder() {
    let entity = Entity::builder(EntityType::Organization, "Acme Corp")
        .description("A fictional company")
        .confidence(0.95)
        .importance(0.8)
        .alias("Acme")
        .namespace("business")
        .property("founded", serde_json::json!(1921))
        .build();

    assert_eq!(entity.label, "Acme Corp");
    assert_eq!(entity.description, "A fictional company");
    assert!((entity.confidence - 0.95).abs() < 0.01);
    assert!((entity.importance - 0.8).abs() < 0.01);
    assert_eq!(entity.aliases.len(), 1);
    assert_eq!(entity.namespace, "business");
}

#[test]
fn entity_query_matching() {
    let entity = Entity::builder(EntityType::Concept, "Machine Learning")
        .description("A subset of artificial intelligence")
        .build();
    assert!(entity.matches_query("machine"));
    assert!(entity.matches_query("learning"));
    assert!(entity.matches_query("artificial"));
    assert!(!entity.matches_query("biology"));
}

#[test]
fn relation_creation() {
    let source = EntityId::new();
    let target = EntityId::new();
    let relation = Relation::new(RelationType::IsA, source, target, "is_type_of".to_string());
    assert_eq!(relation.source, source);
    assert_eq!(relation.target, target);
    assert_eq!(relation.weight, 1.0);
    assert!(relation.active);
}

#[test]
fn relation_builder() {
    let source = EntityId::new();
    let target = EntityId::new();
    let relation = Relation::builder(RelationType::Causes, source, target, "leads_to")
        .weight(0.8)
        .confidence(0.9)
        .undirected()
        .build();

    assert_eq!(relation.weight, 0.8);
    assert_eq!(relation.confidence, 0.9);
    assert_eq!(relation.directedness, Directedness::Undirected);
}

#[test]
fn relation_connects() {
    let a = EntityId::new();
    let b = EntityId::new();
    let rel = Relation::new(RelationType::RelatedTo, a, b, "related".to_string());
    assert!(rel.connects(a, b));
    assert!(!rel.connects(b, a)); // directed
    assert!(rel.other_end(a) == Some(b));
    assert!(rel.other_end(b) == Some(a));
}

#[test]
fn attribute_types() {
    let attr_str = Attribute::new_string("name", "test");
    assert!(matches!(attr_str.value, AttributeValue::String(_)));

    let attr_int = Attribute::new_integer("count", 42);
    assert!(matches!(attr_int.value, AttributeValue::Integer(42)));

    let attr_float = Attribute::new_float("score", 0.95);
    assert!(matches!(attr_float.value, AttributeValue::Float(0.95)));

    let attr_bool = Attribute::new_boolean("active", true);
    assert!(matches!(attr_bool.value, AttributeValue::Boolean(true)));
}

#[test]
fn namespace_registry() {
    let mut registry = NamespaceRegistry::new();
    assert!(registry.exists("default"));

    let ns = KnowledgeNamespace::with_description("test", "Test namespace");
    registry.register(ns).unwrap();
    assert!(registry.exists("test"));

    let list = registry.list();
    assert_eq!(list.len(), 2);
}

#[test]
fn versioning() {
    let mut vv = VersionVector::new();
    assert_eq!(vv.global_version(), 0);
    vv.increment_global();
    assert_eq!(vv.global_version(), 1);
    vv.increment_namespace("test");
    assert_eq!(vv.namespace_version("test"), 1);

    let mut tracker = VersionTracker::new(10);
    tracker.record(1, ChangeType::Created, "initial", "system", "abc123");
    assert_eq!(tracker.latest_version(), 1);
    assert_eq!(tracker.history().len(), 1);
}

// ═══════════════════════════════════════
//  ONTOLOGY TESTS
// ═══════════════════════════════════════

#[test]
fn ontology_default_types() {
    let ontology = Ontology::default();
    assert!(ontology.has_entity_type("person"));
    assert!(ontology.has_entity_type("concept"));
    assert!(ontology.has_relation_type("is_a"));
    assert!(ontology.has_relation_type("causes"));
}

#[test]
fn ontology_custom_type() {
    let mut ontology = Ontology::new("test");
    let def = EntityTypeDefinition {
        entity_type: EntityType::Custom("Robot".to_string()),
        description: "A robotic entity".to_string(),
        parent_type: Some(EntityType::Object),
        required_properties: vec!["model".to_string()],
        allowed_properties: HashMap::new(),
        instantiable: true,
    };
    ontology.register_entity_type(def);
    assert!(ontology.has_entity_type("robot"));
}

#[test]
fn taxonomy_tree() {
    let mut tree = TaxonomyTree::new();
    tree.add_type("Animal".to_string(), None, "A living creature".to_string());
    tree.add_type("Dog".to_string(), Some("Animal".to_string()), "A canine".to_string());
    tree.add_type("Cat".to_string(), Some("Animal".to_string()), "A feline".to_string());

    assert_eq!(tree.count(), 3);
    assert!(tree.is_ancestor("Animal", "Dog"));
    assert!(!tree.is_ancestor("Dog", "Animal"));

    let path = tree.path_to_root("Dog");
    assert_eq!(path, vec!["Animal", "Dog"]);

    let descendants = tree.descendants("Animal");
    assert_eq!(descendants.len(), 2);
}

#[test]
fn ontology_validation() {
    let ontology = Ontology::default();
    let validator = OntologyValidator::new(&ontology);

    let entity = Entity::new(EntityType::Person, "Test".to_string());
    let result = validator.validate_entity(&entity);
    assert!(result.valid);
}

// ═══════════════════════════════════════
//  GRAPH STORE TESTS
// ═══════════════════════════════════════

#[test]
fn graph_store_insert_and_query() {
    let store = GraphStore::default_ontology();

    let alice = Entity::new(EntityType::Person, "Alice".to_string());
    let bob = Entity::new(EntityType::Person, "Bob".to_string());
    let alice_id = alice.id;
    let bob_id = bob.id;

    store.insert_entity(alice);
    store.insert_entity(bob);

    let rel = Relation::new(RelationType::RelatedTo, alice_id, bob_id, "knows".to_string());
    store.insert_relation(rel);

    assert_eq!(store.entity_count(), 2);
    assert_eq!(store.relation_count(), 1);

    let found = store.find_entities_by_label("Alice");
    assert_eq!(found.len(), 1);

    let neighbors = store.neighbors(alice_id);
    assert_eq!(neighbors.len(), 1);
    assert!(neighbors.contains(&bob_id));
}

#[test]
fn graph_store_remove_entity() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    store.insert_entity(e1);
    store.insert_entity(e2);

    let rel = Relation::new(RelationType::RelatedTo, e1_id, e2_id, "link".to_string());
    store.insert_relation(rel);

    store.remove_entity(e1_id).unwrap();
    assert_eq!(store.entity_count(), 1);
    assert_eq!(store.relation_count(), 0);
}

#[test]
fn graph_store_by_type() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Person, "P1".to_string()));
    store.insert_entity(Entity::new(EntityType::Person, "P2".to_string()));
    store.insert_entity(Entity::new(EntityType::Place, "L1".to_string()));

    let people = store.find_entities_by_type(&EntityType::Person);
    assert_eq!(people.len(), 2);

    let places = store.find_entities_by_type(&EntityType::Place);
    assert_eq!(places.len(), 1);
}

// ═══════════════════════════════════════
//  EXTRACTION TESTS
// ═══════════════════════════════════════

#[test]
fn concept_extraction() {
    let extractor = ConceptExtractor::new();
    let text = "Alice and Bob discussed Machine Learning at Stanford University";
    let concepts = extractor.extract_from_text(text, "test_source");
    assert!(!concepts.is_empty());
}

#[test]
fn entity_extraction() {
    let extractor = EntityExtractor::new();
    let text = "Dr. Smith works at Google. He said \"Artificial Intelligence is the future.\"";
    let entities = extractor.extract(text, "test_source");
    assert!(!entities.is_empty());
    assert!(entities.iter().any(|e| e.entity_type == EntityType::Person));
}

#[test]
fn relation_extraction() {
    let extractor = RelationExtractor::new();
    let text = "Python is a programming language. Java depends on the JVM.";
    let labels = vec!["Python".to_string(), "Java".to_string(), "JVM".to_string()];
    let relations = extractor.extract(text, &labels, "test");
    assert!(!relations.is_empty());
}

#[test]
fn confidence_estimation() {
    let estimator = ConfidenceEstimator::new();
    let entity = Entity::builder(EntityType::Person, "Test")
        .confidence(0.8)
        .source("a")
        .source("b")
        .build();
    let conf = estimator.estimate_entity_confidence(&entity, &entity.sources);
    assert!(conf > 0.0 && conf <= 1.0);
}

#[test]
fn conflict_detection() {
    let estimator = ConfidenceEstimator::new();
    let mut e1 = Entity::new(EntityType::Person, "John".to_string());
    let mut e2 = Entity::new(EntityType::Place, "John".to_string());
    let entities = vec![e1, e2];
    let relations = vec![];
    let detection = estimator.detect_conflicts(&entities, &relations);
    assert!(detection.has_conflicts);
    assert!(!detection.conflicts.is_empty());
}

// ═══════════════════════════════════════
//  REASONING TESTS
// ═══════════════════════════════════════

#[test]
fn neighbor_expansion() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e3 = Entity::new(EntityType::Concept, "C".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "ab".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e2_id, e3_id, "bc".to_string()));

    let expander = NeighborExpander::new();
    let neighbors = expander.n_hop_neighbors(&store, e1_id, 2);
    assert!(neighbors.contains(&e2_id));
    assert!(neighbors.contains(&e3_id));
}

#[test]
fn shortest_path() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "Start".to_string());
    let e2 = Entity::new(EntityType::Concept, "Middle".to_string());
    let e3 = Entity::new(EntityType::Concept, "End".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "a".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e2_id, e3_id, "b".to_string()));

    let searcher = PathSearcher::new();
    let result = searcher.shortest_path(&store, e1_id, e3_id);
    assert!(result.found);
    assert_eq!(result.hops, 2);
    assert_eq!(result.path.len(), 3);
}

#[test]
fn graph_traversal_bfs() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "Root".to_string());
    let e2 = Entity::new(EntityType::Concept, "Child1".to_string());
    let e3 = Entity::new(EntityType::Concept, "Child2".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "a".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e3_id, "b".to_string()));

    let traversal = GraphTraversal::new();
    let config = TraversalConfig {
        max_depth: 2,
        max_results: 100,
        edge_filter: None,
        node_type_filter: None,
    };
    let result = traversal.bfs(&store, e1_id, &config);
    assert!(result.entities.len() >= 3);
}

// ═══════════════════════════════════════
//  SEARCH TESTS
// ═══════════════════════════════════════

#[test]
fn keyword_search() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::builder(EntityType::Concept, "Machine Learning").build());
    store.insert_entity(Entity::builder(EntityType::Concept, "Deep Learning").build());
    store.insert_entity(Entity::builder(EntityType::Concept, "Cooking").build());

    let search = KeywordSearch::new();
    let entities = store.all_entities();
    let results = search.search_all(&entities, "learning");
    assert_eq!(results.len(), 2);
}

#[test]
fn metadata_search() {
    let store = GraphStore::default_ontology();
    let mut e1 = Entity::new(EntityType::Person, "Alice".to_string());
    e1.namespace = "team_a".to_string();
    store.insert_entity(e1);
    store.insert_entity(Entity::new(EntityType::Person, "Bob".to_string()));

    let search = MetadataSearch::new();
    let entities = store.all_entities();
    let results = search.search_by_namespace(&entities, "team_a");
    assert_eq!(results.len(), 1);
}

#[test]
fn temporal_search() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Event, "Recent".to_string()));
    store.insert_entity(Entity::new(EntityType::Event, "Another".to_string()));

    let search = TemporalSearch::new();
    let entities = store.all_entities();
    let results = search.recently_updated(&entities);
    assert_eq!(results.len(), 2);
}

// ═══════════════════════════════════════
//  VALIDATION TESTS
// ═══════════════════════════════════════

#[test]
fn source_tracking() {
    let tracker = SourceTracker::new();
    tracker.record_source("entity_1", "memory_system", 0.9);
    tracker.record_source("entity_1", "user_input", 0.8);
    tracker.record_source("entity_2", "inference", 0.7);

    assert_eq!(tracker.source_count("entity_1"), 2);
    assert_eq!(tracker.source_count("entity_2"), 1);
    assert_eq!(tracker.total_records(), 3);
}

#[test]
fn evidence_tracking() {
    let tracker = EvidenceTracker::new();
    tracker.add_supporting("claim_1", "Evidence A", "source_1", 0.9);
    tracker.add_supporting("claim_1", "Evidence B", "source_2", 0.8);
    tracker.add_contradicting("claim_1", "Counter-evidence", "source_3", 0.6);

    assert_eq!(tracker.support_count("claim_1"), 2);
    assert_eq!(tracker.contradiction_count("claim_1"), 1);
    assert!(tracker.net_score("claim_1") > 0.0);
}

#[test]
fn contradiction_detection() {
    let detector = ContradictionDetector::new();
    let e1 = Entity::builder(EntityType::Person, "John").build();
    let mut e2 = Entity::new(EntityType::Place, "John".to_string());
    let entities = vec![e1, e2];
    let contradictions = detector.detect_entity_contradictions(&entities);
    assert!(!contradictions.is_empty());
}

#[test]
fn conflict_resolution() {
    let resolver = ConflictResolver::new(ResolutionStrategy::HighestConfidence);
    let mut a = Entity::new(EntityType::Person, "Test".to_string());
    a.confidence = 0.9;
    let mut b = Entity::new(EntityType::Person, "Test".to_string());
    b.confidence = 0.5;

    let result = resolver.resolve_contradiction(&a, &b, None);
    assert_eq!(result.kept.len(), 1);
    assert_eq!(result.removed.len(), 1);
}

// ═══════════════════════════════════════
//  EVOLUTION TESTS
// ═══════════════════════════════════════

#[test]
fn concept_merging() {
    let merger = ConceptMerger::new();
    let mut a = Entity::new(EntityType::Concept, "Machine Learning".to_string());
    a.confidence = 0.9;
    let mut b = Entity::new(EntityType::Concept, "ML".to_string());
    b.confidence = 0.7;
    let outcome = merger.merge_pair(&a, &b);
    assert_eq!(outcome.surviving, a.id);
    assert_eq!(outcome.merged.len(), 1);
}

#[test]
fn concept_splitting() {
    let splitter = ConceptSplitter::new();
    let original = Entity::new(EntityType::Concept, "AI/ML".to_string());
    let outcome = splitter.split_by_labels(&original, "AI", "ML");
    assert_eq!(outcome.created.len(), 2);
}

#[test]
fn relationship_discovery() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e3 = Entity::new(EntityType::Concept, "Shared".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e3_id, "a".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e2_id, e3_id, "b".to_string()));

    let discovery = RelationshipDiscovery::new();
    let discoveries = discovery.discover_by_shared_neighbors(&store, 1, 0.1);
    assert!(!discoveries.is_empty());
}

#[test]
fn knowledge_pruning() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::builder(EntityType::Concept, "Unimportant")
        .confidence(0.05)
        .importance(0.05)
        .build());

    let pruner = KnowledgePruner::default();
    let entities = store.all_entities();
    let candidates = pruner.candidates(&entities, &store);
    assert!(!candidates.is_empty());
}

// ═══════════════════════════════════════
//  INFERENCE INTEGRATION TESTS
// ═══════════════════════════════════════

#[test]
fn knowledge_aware_prompting() {
    let prompter = KnowledgeAwarePrompter::new();
    let entities = vec![
        Entity::builder(EntityType::Concept, "Rust")
            .description("A systems programming language")
            .build(),
    ];
    let prompt = prompter.build_prompt("What is Rust?", &entities, 1000);
    assert!(prompt.contains("Rust"));
    assert!(prompt.contains("Query:"));
}

#[test]
fn context_enrichment() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Person, "Alice".to_string());
    let e2 = Entity::new(EntityType::Person, "Bob".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "knows".to_string()));

    let enricher = ContextEnricher::new();
    let alice = store.get_entity(e1_id).unwrap();
    let ctx = enricher.enrich_entity(&store, &alice);
    assert!(!ctx.related_facts.is_empty());
}

#[test]
fn fact_retrieval() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::builder(EntityType::Concept, "Python")
        .description("A programming language")
        .build());

    let retriever = FactRetriever::new();
    let facts = retriever.retrieve(&store, "Python", 5);
    assert!(!facts.is_empty());
}

#[test]
fn prompt_assembly() {
    let assembler = PromptAssembler::new();
    let facts = vec![];
    let result = assembler.assemble("You are helpful.", "What is 2+2?", &facts, 4096);
    assert_eq!(result.messages.len(), 2);
}

// ═══════════════════════════════════════
//  WORLD MODEL TESTS
// ═══════════════════════════════════════

#[test]
fn world_model_manager() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Person, "Alice".to_string()));
    store.insert_entity(Entity::new(EntityType::Person, "Bob".to_string()));
    store.insert_entity(Entity::new(EntityType::Place, "Office".to_string()));
    store.insert_entity(Entity::new(EntityType::Task, "Write code".to_string()));

    let wm = WorldModelManager::new(&store);
    assert_eq!(wm.people().len(), 2);
    assert_eq!(wm.places().len(), 1);
    assert_eq!(wm.tasks().len(), 1);

    let counts = wm.count_by_type();
    assert_eq!(counts.get("person").copied().unwrap_or(0), 2);
}

// ═══════════════════════════════════════
//  ANALYTICS TESTS
// ═══════════════════════════════════════

#[test]
fn density_analysis() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "link".to_string()));

    let analyzer = DensityAnalyzer::new();
    let stats = analyzer.analyze(&store);
    assert_eq!(stats.entity_count, 2);
    assert_eq!(stats.relation_count, 1);
}

#[test]
fn centrality_analysis() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "Hub".to_string());
    let e2 = Entity::new(EntityType::Concept, "Leaf1".to_string());
    let e3 = Entity::new(EntityType::Concept, "Leaf2".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "a".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e3_id, "b".to_string()));

    let analyzer = CentralityAnalyzer::new();
    let centrality = analyzer.degree_centrality(&store);
    let hub_score = centrality.get(&e1_id).copied().unwrap_or(0.0);
    let leaf_score = centrality.get(&e2_id).copied().unwrap_or(0.0);
    assert!(hub_score > leaf_score);
}

#[test]
fn connected_components() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Concept, "A".to_string()));
    store.insert_entity(Entity::new(EntityType::Concept, "B".to_string()));
    store.insert_entity(Entity::new(EntityType::Concept, "C".to_string()));
    // A-B connected, C isolated

    let e_a = store.find_entities_by_label("A")[0].id;
    let e_b = store.find_entities_by_label("B")[0].id;
    store.insert_relation(Relation::new(RelationType::RelatedTo, e_a, e_b, "ab".to_string()));

    let analyzer = ConnectedComponentAnalyzer::new();
    let components = analyzer.find_components(&store);
    assert_eq!(components.len(), 2); // {A,B} and {C}
}

#[test]
fn community_detection() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "link".to_string()));

    let detector = CommunityDetector::new();
    let communities = detector.detect(&store);
    assert!(!communities.is_empty());
}

// ═══════════════════════════════════════
//  PERSISTENCE TESTS
// ═══════════════════════════════════════

#[test]
fn sqlite_persistence() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Person, "Alice".to_string()));

    let sqlite = SqliteStore::open(&db_path).unwrap();
    sqlite.save_graph(&store).unwrap();

    // Verify file exists
    assert!(db_path.exists());
}

// ═══════════════════════════════════════
//  SECURITY TESTS
// ═══════════════════════════════════════

#[test]
fn namespace_permissions() {
    let perms = NamespacePermissions::new();
    assert!(perms.check("default", PermissionLevel::Admin));
    assert!(perms.check("default", PermissionLevel::Read));

    perms.set("secret", PermissionLevel::Read);
    assert!(perms.check("secret", PermissionLevel::Read));
    assert!(!perms.check("secret", PermissionLevel::Write));
}

#[test]
fn access_controller() {
    let controller = AccessController::new();
    assert!(controller.can_read("default"));
    assert!(controller.can_write("default"));
    assert!(controller.can_admin("default"));
}

#[test]
fn audit_trail() {
    let trail = AuditTrail::new(100);
    trail.log(
        AuditAction::EntityCreate,
        "user",
        "entity_1",
        "default",
        true,
        None,
    );
    assert_eq!(trail.count(), 1);

    let entries = trail.entries_for_target("entity_1");
    assert_eq!(entries.len(), 1);
}

#[test]
fn graph_encryption() {
    let hash1 = GraphEncryption::checksum("hello");
    let hash2 = GraphEncryption::checksum("hello");
    assert_eq!(hash1, hash2);

    let hash3 = GraphEncryption::checksum("world");
    assert_ne!(hash1, hash3);
}

// ═══════════════════════════════════════
//  MONITORING TESTS
// ═══════════════════════════════════════

#[test]
fn monitoring() {
    let monitor = KnowledgeMonitor::new();
    assert_eq!(monitor.query_count(), 0);

    monitor.record_query(1.5);
    monitor.record_query(2.5);
    assert_eq!(monitor.query_count(), 2);
    assert!((monitor.avg_query_latency_ms() - 2.0).abs() < 0.01);

    monitor.record_extraction();
    assert_eq!(monitor.extraction_count(), 1);
}

// ═══════════════════════════════════════
//  API TESTS
// ═══════════════════════════════════════

#[test]
fn entity_api_crud() {
    let store = GraphStore::default_ontology();
    let api = EntityApi::new(&store);

    let entity = api.create(EntityType::Person, "TestUser");
    assert_eq!(entity.label, "TestUser");

    let found = api.get(entity.id).unwrap();
    assert_eq!(found.label, "TestUser");

    api.update(entity.id, |e| {
        e.description = "Updated".to_string();
    }).unwrap();

    let updated = api.get(entity.id).unwrap();
    assert_eq!(updated.description, "Updated");

    api.delete(entity.id).unwrap();
    let deleted = store.get_entity(entity.id).unwrap();
    assert!(!deleted.active);
}

#[test]
fn relation_api_crud() {
    let store = GraphStore::default_ontology();
    let e1 = store.insert_entity(Entity::new(EntityType::Concept, "A".to_string()));
    let e2 = store.insert_entity(Entity::new(EntityType::Concept, "B".to_string()));

    let api = RelationApi::new(&store);
    let rel = api.create(RelationType::RelatedTo, e1, e2, "link".to_string()).unwrap();
    assert_eq!(api.outgoing(e1).len(), 1);
    assert_eq!(api.incoming(e2).len(), 1);

    api.remove(rel.id).unwrap();
    assert_eq!(api.count(), 0);
}

#[test]
fn traverse_api() {
    let store = GraphStore::default_ontology();
    let e1 = Entity::new(EntityType::Concept, "A".to_string());
    let e2 = Entity::new(EntityType::Concept, "B".to_string());
    let e3 = Entity::new(EntityType::Concept, "C".to_string());
    let e1_id = e1.id;
    let e2_id = e2.id;
    let e3_id = e3.id;
    store.insert_entity(e1);
    store.insert_entity(e2);
    store.insert_entity(e3);
    store.insert_relation(Relation::new(RelationType::RelatedTo, e1_id, e2_id, "a".to_string()));
    store.insert_relation(Relation::new(RelationType::RelatedTo, e2_id, e3_id, "b".to_string()));

    let api = TraverseApi::new(&store);
    let path = api.shortest_path(e1_id, e3_id);
    assert!(path.found);
    assert_eq!(path.hops, 2);
}

#[test]
fn export_import_json() {
    let store = GraphStore::default_ontology();
    store.insert_entity(Entity::new(EntityType::Person, "Exported".to_string()));

    let json = GraphExporter::to_json(&store).unwrap();
    assert!(json.contains("Exported"));

    let store2 = GraphStore::default_ontology();
    let result = GraphImporter::from_json(&json, &store2).unwrap();
    assert!(result.entities_imported >= 1);
}

// ═══════════════════════════════════════
//  INTEGRATION TESTS
// ═══════════════════════════════════════

#[test]
fn full_knowledge_graph_lifecycle() {
    let kg = make_graph();

    // Create entities
    let alice = kg.create_entity(EntityType::Person, "Alice");
    let bob = kg.create_entity(EntityType::Person, "Bob");
    let ml = kg.create_entity(EntityType::Concept, "Machine Learning");

    // Create relations
    let rel1 = kg.create_relation(RelationType::RelatedTo, alice.id, bob.id, "colleague").unwrap();
    let rel2 = kg.create_relation(RelationType::RelatedTo, alice.id, ml.id, "expertise").unwrap();

    // Search
    let results = kg.search("Alice", 10);
    assert!(!results.is_empty());

    // Traversal
    let neighbors = kg.expand_neighbors(alice.id, 2);
    assert!(neighbors.contains(&bob.id));
    assert!(neighbors.contains(&ml.id));

    // Snapshot
    let snapshot = kg.create_snapshot("test snapshot");
    assert_eq!(snapshot.entity_count, 3);

    // World model
    let wm = kg.world_model();
    assert_eq!(wm.people().len(), 2);

    // Analytics
    let density = kg.density_analysis();
    assert_eq!(density.entity_count, 3);

    // Metrics
    let metrics = kg.metrics();
    assert_eq!(metrics.entity_count, 3);

    // Contradiction detection
    let contradictions = kg.detect_contradictions();
    assert!(contradictions.is_empty());
}

#[test]
fn knowledge_extraction_pipeline() {
    let kg = make_graph();

    let text = "Alice is a Machine Learning engineer at Google. She depends on TensorFlow.";
    let concept = kg.extract_from_text(text, "conversation");
    assert!(!concept.label.is_empty());

    let entities = kg.extract_entities(text, "conversation");
    assert!(!entities.is_empty());
}

#[test]
fn knowledge_persistence_cycle() {
    let kg = make_graph();
    kg.create_entity(EntityType::Person, "PersistentUser");

    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("cycle_test.db");
    kg.save_to_sqlite(&db_path).unwrap();
    assert!(db_path.exists());
}

#[test]
fn concurrent_entity_creation() {
    use std::sync::Arc;
    let store = Arc::new(GraphStore::default_ontology());
    let mut handles = vec![];

    for i in 0..10 {
        let store = store.clone();
        handles.push(std::thread::spawn(move || {
            let entity = Entity::new(EntityType::Concept, format!("Concept_{}", i));
            store.insert_entity(entity);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(store.entity_count(), 10);
}

#[test]
fn stress_test_large_graph() {
    let store = GraphStore::default_ontology();
    let mut entity_ids = vec![];

    for i in 0..500 {
        let entity = Entity::new(EntityType::Concept, format!("Node_{}", i));
        entity_ids.push(entity.id);
        store.insert_entity(entity);
    }

    for i in 0..499 {
        let rel = Relation::new(
            RelationType::RelatedTo,
            entity_ids[i],
            entity_ids[i + 1],
            format!("edge_{}", i),
        );
        store.insert_relation(rel);
    }

    assert_eq!(store.entity_count(), 500);
    assert_eq!(store.relation_count(), 499);

    let expander = NeighborExpander::new();
    let neighbors = expander.n_hop_neighbors(&store, entity_ids[0], 10);
    assert!(neighbors.len() >= 10);
}
