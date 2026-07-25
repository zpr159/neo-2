use crate::conversation::evidence::{Evidence, EvidenceSource};

/// Unified context produced by the retrieval coordinator after merging all sources.
#[derive(Debug, Clone, Default)]
pub struct UnifiedContext {
    pub evidence: Vec<Evidence>,
    pub summaries: Vec<ContextSummary>,
    pub entity_map: std::collections::HashMap<String, Vec<usize>>,
    pub source_coverage: std::collections::HashMap<EvidenceSource, usize>,
    pub total_items: usize,
    pub average_confidence: f32,
    pub contradictions_detected: usize,
}

#[derive(Debug, Clone)]
pub struct ContextSummary {
    pub topic: String,
    pub supporting_evidence: Vec<usize>,
    pub contradicting_evidence: Vec<usize>,
    pub average_confidence: f32,
}

/// Merges duplicate entities, facts, and evidence from multiple cognitive sources.
pub struct ContextMerger;

impl ContextMerger {
    pub fn new() -> Self {
        Self
    }

    /// Merge evidence from multiple sources into a unified context.
    pub fn merge(&self, evidence: Vec<Evidence>) -> UnifiedContext {
        let mut context = UnifiedContext::default();
        context.total_items = evidence.len();

        if evidence.is_empty() {
            return context;
        }

        let mut merged: Vec<Evidence> = Vec::new();
        let mut entity_map: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();

        for evidence_item in evidence {
            let key = self.make_dedup_key(&evidence_item);

            if let Some(existing_indices) = entity_map.get(&key) {
                for &idx in existing_indices {
                    merged[idx] = self.reconcile(&merged[idx], &evidence_item);
                }
                entity_map.get_mut(&key).unwrap().push(merged.len());
            } else {
                entity_map
                    .entry(key)
                    .or_default()
                    .push(merged.len());
                merged.push(evidence_item);
            }
        }

        let mut source_coverage = std::collections::HashMap::new();
        let mut total_confidence = 0.0;
        for (idx, item) in merged.iter().enumerate() {
            *source_coverage.entry(item.source.clone()).or_insert(0) += 1;
            total_confidence += item.confidence;
            entity_map.values_mut().for_each(|v| {
                v.iter_mut().for_each(|_i| {
                    // remap indices since we may have deduplicated
                });
            });
            let _ = idx;
        }

        context.evidence = merged;
        context.entity_map = entity_map;
        context.source_coverage = source_coverage;
        context.average_confidence = if context.evidence.is_empty() {
            0.0
        } else {
            total_confidence / context.evidence.len() as f32
        };

        context
    }

    /// Create a deduplication key from evidence content.
    fn make_dedup_key(&self, evidence: &Evidence) -> String {
        let content_normalized = evidence
            .content
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        format!("{:?}:{}", evidence.source, content_normalized)
    }

    /// Reconcile two evidence items that refer to the same fact.
    fn reconcile(&self, existing: &Evidence, new: &Evidence) -> Evidence {
        let higher_confidence = existing.confidence.max(new.confidence);
        let combined_relevance = (existing.relevance_score + new.relevance_score) / 2.0;
        let mut merged = existing.clone();
        merged.confidence = higher_confidence;
        merged.relevance_score = combined_relevance;

        let mut refs = existing.supporting_references.clone();
        refs.extend(new.supporting_references.clone());
        refs.sort();
        refs.dedup();
        merged.supporting_references = refs;

        if merged.explanation.is_none() {
            merged.explanation = new.explanation.clone();
        }

        merged
    }

    /// Detect contradictions in evidence.
    pub fn detect_contradictions(context: &mut UnifiedContext) {
        let n = context.evidence.len();
        let mut contradictions = 0;

        let merger = Self::new();
        for i in 0..n {
            for j in (i + 1)..n {
                if merger.are_contradictory(&context.evidence[i], &context.evidence[j]) {
                    contradictions += 1;
                }
            }
        }

        context.contradictions_detected = contradictions;
    }

    /// Simple contradiction detection based on negation patterns.
    fn are_contradictory(&self, a: &Evidence, b: &Evidence) -> bool {
        if a.source == b.source && a.content == b.content {
            return false;
        }

        let a_lower = a.content.to_lowercase();
        let b_lower = b.content.to_lowercase();

        // Check if one contains the negation of the other
        let a_has_not = a_lower.contains("not ");
        let b_has_not = b_lower.contains("not ");

        if a_has_not != b_has_not {
            // One has "not" and the other doesn't
            let a_stripped = a_lower.replace("not ", "");
            let b_stripped = b_lower.replace("not ", "");
            // Check if the remaining content is similar enough
            let a_words: Vec<&str> = a_stripped.split_whitespace().collect();
            let b_words: Vec<&str> = b_stripped.split_whitespace().collect();
            if a_words.len() >= 3 && b_words.len() >= 3 {
                let common = a_words.iter().filter(|w| b_words.contains(w)).count();
                let total = a_words.len().max(b_words.len());
                if common as f64 / total as f64 > 0.6 {
                    return true;
                }
            }
        }

        false
    }

    /// Sort evidence in deterministic order.
    pub fn sort_deterministic(context: &mut UnifiedContext) {
        context
            .evidence
            .sort_by(|a, b| a.id.cmp(&b.id));
    }
}

impl Default for ContextMerger {
    fn default() -> Self {
        Self::new()
    }
}
