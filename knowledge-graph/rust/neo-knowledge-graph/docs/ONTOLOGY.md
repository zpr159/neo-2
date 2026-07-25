# Ontology System

## Overview

The ontology system defines the schema for the knowledge graph: what types of entities and relations are valid, how they relate to each other hierarchically, and what properties they can have.

## Ontology Structure

```
Ontology
+-- name: String
+-- version: String
+-- entity_types:    HashMap<String, EntityTypeDefinition>
+-- relation_types:  HashMap<String, RelationTypeDefinition>
+-- properties:      HashMap<String, PropertyDefinition>
+-- description: String
```

Keys are normalized to lowercase for consistent lookup.

## Default Types

### Entity Types (13)

| Type | Description | Parent |
|------|-------------|--------|
| `person` | A human person | - |
| `place` | A physical or logical location | - |
| `organization` | An organization or group | - |
| `object` | A physical or digital object | - |
| `event` | An occurrence or happening | - |
| `concept` | An abstract concept or idea | - |
| `task` | A unit of work to be done | - |
| `goal` | An objective to be achieved | - |
| `skill` | A capability or expertise | - |
| `project` | A planned undertaking | - |
| `document` | A written or digital document | - |
| `idea` | A thought or suggestion | - |
| `rule` | A governing principle | - |

Custom types can be registered via `register_entity_type()`.

### Relation Types (19)

| Type | Description | Symmetric | Transitive |
|------|-------------|-----------|------------|
| `is_a` | Entity is a type of another | yes | no |
| `has_a` | Entity has a component | no | no |
| `part_of` | Entity is part of another | no | yes |
| `related_to` | General relation | no | no |
| `causes` | First causes second | no | no |
| `enables` | First enables second | no | no |
| `prevents` | First prevents second | no | no |
| `depends_on` | First depends on second | no | no |
| `located_at` | Entity is located at | no | no |
| `member_of` | Entity is member of | no | no |
| `author_of` | Entity authored target | no | no |
| `created_by` | Entity was created by | no | no |
| `uses` | Entity uses target | no | no |
| `inherits_from` | Entity inherits from | no | no |
| `implements` | Entity implements target | no | no |
| `contradicts` | Entities contradict | yes | no |
| `supports` | Entities support each other | yes | no |
| `temporally_follows` | First follows second in time | no | no |
| `spatially_near` | Entities are spatially near | yes | no |

Custom relation types can be registered via `register_relation_type()`.

## EntityTypeDefinition

```rust
pub struct EntityTypeDefinition {
    pub entity_type: EntityType,
    pub description: String,
    pub parent_type: Option<EntityType>,   // inheritance
    pub required_properties: Vec<String>,
    pub allowed_properties: HashMap<String, String>,
    pub instantiable: bool,
}
```

## RelationTypeDefinition

```rust
pub struct RelationTypeDefinition {
    pub relation_type: RelationType,
    pub description: String,
    pub parent_type: Option<RelationType>,
    pub valid_source_types: Option<Vec<EntityType>>,
    pub valid_target_types: Option<Vec<EntityType>>,
    pub required_properties: Vec<String>,
    pub symmetric: bool,
    pub transitive: bool,
    pub max_weight: f32,
}
```

## API

### Registration

```rust
let mut ontology = Ontology::new("my_ontology");

// Register a custom entity type
let def = EntityTypeDefinition {
    entity_type: EntityType::Custom("Robot".to_string()),
    description: "A robotic entity".to_string(),
    parent_type: Some(EntityType::Object),
    required_properties: vec!["model".to_string()],
    allowed_properties: HashMap::new(),
    instantiable: true,
};
ontology.register_entity_type(def);

// Register a custom relation type
let rel_def = RelationTypeDefinition { ... };
ontology.register_relation_type(rel_def);
```

### Lookup

```rust
ontology.has_entity_type("robot");           // true
ontology.get_entity_type("robot");           // Some(&EntityTypeDefinition)
ontology.entity_type_names();                // Vec<&str>
ontology.has_relation_type("custom_rel");    // bool
ontology.relation_type_names();              // Vec<&str>
```

### Inheritance

```rust
// Get parent chain: ["object", "concept"]
ontology.parent_chain("robot");

// Check if child is subtype of parent
ontology.is_subtype("robot", "object");      // true
ontology.is_subtype("robot", "concept");     // false
```

## TaxonomyTree

A tree-based taxonomy for hierarchical type organization:

```
TaxonomyTree
+-- nodes: HashMap<String, TaxonomyNode>

TaxonomyNode
+-- name: String
+-- parent: Option<String>
+-- children: Vec<String>
+-- depth: usize
+-- description: String
```

### API

```rust
let mut tree = TaxonomyTree::new();
tree.add_type("Animal".to_string(), None, "A living creature".to_string());
tree.add_type("Dog".to_string(), Some("Animal".to_string()), "A canine".to_string());
tree.add_type("Cat".to_string(), Some("Animal".to_string()), "A feline".to_string());

tree.count();                              // 3
tree.is_ancestor("Animal", "Dog");         // true
tree.path_to_root("Dog");                  // ["Animal", "Dog"]
tree.descendants("Animal");                // ["Dog", "Cat"]
tree.ancestors("Dog");                     // ["Animal"]
tree.roots();                              // ["Animal"]
tree.depth_of("Dog");                      // 1
tree.max_depth();                          // 1
```

Removing a type re-parents its children to the grandparent.

## OntologyValidator

Validates entities and relations against the ontology schema:

```rust
let ontology = Ontology::default();
let validator = OntologyValidator::new(&ontology);

// Validate an entity
let result = validator.validate_entity(&entity);
if !result.valid {
    for violation in &result.violations {
        println!("{}: {}", violation.kind, violation.message);
    }
}

// Validate a relation (checks source/target entity types)
let result = validator.validate_relation(&relation, &source_entity, &target_entity);
```

### Violation Types

| Kind | Description |
|------|-------------|
| `UnknownEntityType` | Entity type not in ontology |
| `UnknownRelationType` | Relation type not in ontology |
| `MissingRequiredProperty` | Required property not present |
| `InvalidPropertyType` | Property value type mismatch |
| `InvalidSourceEntityType` | Source entity type not allowed for this relation |
| `InvalidTargetEntityType` | Target entity type not allowed for this relation |
| `CyclicInheritance` | Parent chain forms a cycle |
