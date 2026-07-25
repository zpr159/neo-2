use crate::inference_integration::fact_retrieval::RetrievedFact;

/// Ranks retrieved facts by relevance and confidence.
pub struct FactRanker;

impl FactRanker {
    /// Create a new ranker.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Rank facts by a combination of confidence and text relevance.
    #[must_use]
    pub fn rank(&self, facts: &mut Vec<RetrievedFact>, query: &str) {
        let query_lower = query.to_lowercase();
        for fact in facts.iter_mut() {
            let text_lower = fact.text.to_lowercase();
            let keyword_score = if text_lower.contains(&query_lower) {
                0.3
            } else {
                query_lower
                    .split_whitespace()
                    .filter(|w| text_lower.contains(w))
                    .count() as f32
                    * 0.1
            };
            fact.confidence = (fact.confidence + keyword_score).min(1.0);
        }
        facts.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    }
}

impl Default for FactRanker {
    fn default() -> Self {
        Self::new()
    }
}
