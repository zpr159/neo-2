# Extraction Pipeline

## Overview

The extraction pipeline transforms unstructured text and keywords into structured knowledge graph elements. All extraction is pattern/keyword-based with no external ML dependencies.

## Pipeline Flow

```text
Input Text
    |
    v
+---ConceptExtractor---+---EntityExtractor---+---RelationExtractor---+
|   (keyword/phrase    |   (pattern matching |   (pattern matching   |
|    extraction)       |    for proper nouns,|    for "is a", "has", |
|                      |    titles, quotes,  |    "depends on", etc.)|
|                      |    URLs)            |                       |
+----------+-----------+----------+----------+----------+------------+
           |                      |                      |
           v                      v                      v
     ExtractedConcept       ExtractedEntity         ExtractedRelation
           |                      |                      |
           +----------+-----------+----------+-----------+
                      |
                      v
              DuplicateMerger
              (Jaccard similarity within same EntityType)
                      |
                      v
            ConfidenceEstimator
            (conflict detection, confidence scoring)
                      |
                      v
              Merged Knowledge in GraphStore
```

## ConceptExtractor

Extracts abstract concepts from text.

```rust
let extractor = ConceptExtractor::new();

// From natural language text
let concepts = extractor.extract_from_text(
    "Machine Learning and Deep Learning are AI techniques",
    "conversation"
);

// From explicit keywords
let concepts = extractor.extract_from_keywords(
    &["rust", "systems programming", "memory safety"],
    "documentation"
);
```

### Algorithm

1. Scans for consecutive capitalized words as concept phrases
2. Multi-word concepts receive confidence 0.9 with a length score
3. Single-word concepts receive confidence 0.6
4. Frequency weighting: `freq / (freq + 5)`
5. Output: `ExtractedConcept { label, concept_type, confidence, context, source, properties }`

## EntityExtractor

Extracts typed entities from text using pattern matching.

```rust
let extractor = EntityExtractor::new();
let entities = extractor.extract(
    "Alice is a Machine Learning engineer at Google. She depends on TensorFlow.",
    "conversation"
);

// Convert to graph entities
let graph_entities = EntityExtractor::to_entities(&entities);
```

### Extraction Patterns

| Pattern | Entity Type | Confidence | Example |
|---------|-------------|------------|---------|
| Title prefix | Person | 0.85 | "Dr. Smith", "Mr. Jones" |
| Capitalized proper noun | Concept | 0.5 | "Alice", "Google", "TensorFlow" |
| Quoted text | Concept | 0.6 | `"Machine Learning"` |
| URL | Document | 0.95 | `https://example.com` |

### Skip Words

Common English words (articles, pronouns, prepositions, auxiliary verbs, etc.) are excluded from proper noun detection. The skip list includes ~150 words.

### Deduplication

A `seen_labels` set prevents duplicate entities from being returned.

## RelationExtractor

Extracts typed relations between known entities.

```rust
let extractor = RelationExtractor::new();
let entity_labels = vec!["Alice".to_string(), "Google".to_string()];
let relations = extractor.extract(
    "Alice works at Google",
    &entity_labels,
    "conversation"
);
```

### Extraction Patterns

| Pattern | Relation Type | Confidence |
|---------|--------------|------------|
| "is a" / "is an" | `IsA` | 0.80 |
| "has" / "contains" / "includes" | `HasA` | 0.70 |
| "depends on" / "requires" | `DependsOn` | 0.75 |
| "caused" / "leads to" / "results in" | `Causes` | 0.70 |

Only entity labels present in `entity_labels` are used as source/target. Target entity is extracted as the next 1-3 words after the pattern.

## DuplicateMerger

Identifies and merges duplicate entities within the same `EntityType`.

```rust
let merger = DuplicateMerger::new();
let results = merger.merge_duplicates(&store, 0.7)?; // threshold = 0.7

for result in &results {
    println!("Surviving: {}, Merged: {:?}", result.surviving_entity_id, result.merged_entity_ids);
}
```

### Algorithm

1. Groups active entities by `EntityType`
2. O(n^2) pairwise comparison within each group
3. Similarity = Jaccard index on lowercased word sets of labels
4. If similarity >= threshold: merge group keeps the highest-confidence entity
5. All incoming/outgoing relations of merged entities are redirected to the survivor
6. Merged entities are soft-deleted (active = false)

### Output

```rust
pub struct MergeResult {
    pub surviving_entity_id: EntityId,
    pub merged_entity_ids: Vec<EntityId>,
    pub relations_redirected: usize,
    pub relations_removed: usize,
}
```

## ConfidenceEstimator

Estimates confidence scores and detects conflicts.

```rust
let estimator = ConfidenceEstimator::new();

// Estimate entity confidence
let confidence = estimator.estimate_entity_confidence(&entity, &all_sources);

// Generate full confidence report
let report = estimator.generate_report(&entities, &relations);

// Detect conflicts
let conflicts = estimator.detect_conflicts(&entities, &relations);
```

### Confidence Formulas

**Entity confidence:** `0.6 * base_confidence + 0.4 * source_score`
- `source_score = source_count / (source_count + 5)`

**Relation confidence:** `0.5 * confidence + 0.25 * weight + 0.25 * source_score`

### Conflict Detection

1. Groups entities by lowercase label
2. Detects type mismatches (same label, different EntityType) - severity 0.8
3. Detects property value conflicts (same label + key, different values) - severity 0.6
4. Detects explicit `Contradicts` relations - severity = relation weight
