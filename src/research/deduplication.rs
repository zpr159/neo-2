use super::api::Finding;
use super::config::DeduplicationConfig;

/// Removes duplicate findings based on content similarity.
pub struct Deduplicator {
    config: DeduplicationConfig,
}

impl Deduplicator {
    pub fn new(config: DeduplicationConfig) -> Self {
        Self { config }
    }

    /// Deduplicate a list of findings, keeping the highest confidence version.
    pub fn deduplicate(&self, findings: Vec<Finding>) -> Vec<Finding> {
        if !self.config.enabled {
            return findings;
        }

        match self.config.strategy {
            super::config::DeduplicationStrategy::Exact => self.dedup_exact(findings),
            super::config::DeduplicationStrategy::Fuzzy => self.dedup_fuzzy(findings),
            super::config::DeduplicationStrategy::Semantic => self.dedup_fuzzy(findings),
        }
    }

    fn dedup_exact(&self, findings: Vec<Finding>) -> Vec<Finding> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut result = Vec::new();

        for finding in findings {
            let key = finding.statement.to_lowercase().trim().to_string();
            if !seen.contains(&key) {
                seen.insert(key);
                result.push(finding);
            }
        }

        result
    }

    fn dedup_fuzzy(&self, findings: Vec<Finding>) -> Vec<Finding> {
        let mut result = Vec::new();
        let mut merged_flags = vec![false; findings.len()];

        for i in 0..findings.len() {
            if merged_flags[i] {
                continue;
            }

            let mut best = findings[i].clone();
            merged_flags[i] = true;

            for j in (i + 1)..findings.len() {
                if merged_flags[j] {
                    continue;
                }

                let similarity = compute_similarity(
                    &best.statement,
                    &findings[j].statement,
                );

                if similarity >= self.config.similarity_threshold {
                    if findings[j].confidence > best.confidence {
                        best = findings[j].clone();
                    }
                    merged_flags[j] = true;
                }
            }

            result.push(best);
        }

        result
    }
}

/// Compute Jaccard similarity between two strings based on word tokens.
pub fn compute_similarity(a: &str, b: &str) -> f32 {
    let tokens_a: std::collections::HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let tokens_b: std::collections::HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

/// Compute containment similarity: how much of A is contained in B.
pub fn compute_containment(a: &str, b: &str) -> f32 {
    let tokens_a: std::collections::HashSet<String> = a
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let tokens_b: std::collections::HashSet<String> = b
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if tokens_a.is_empty() {
        return 0.0;
    }

    let contained = tokens_a.intersection(&tokens_b).count();
    contained as f32 / tokens_a.len() as f32
}
