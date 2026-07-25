use super::api::Citation;
use super::config::CitationConfig;

/// Manages citation lifecycle: generation, formatting, and validation.
pub struct CitationManager {
    config: CitationConfig,
}

impl CitationManager {
    pub fn new(config: CitationConfig) -> Self {
        Self { config }
    }

    /// Validate that a set of citations meets minimum requirements.
    pub fn validate_citations(
        &self,
        citations: &[Citation],
        min_required: Option<usize>,
    ) -> Result<Vec<Citation>, Vec<Citation>> {
        let min = min_required.unwrap_or(self.config.min_citations_per_claim);

        if citations.len() >= min {
            Ok(citations.to_vec())
        } else {
            Err(citations.to_vec())
        }
    }

    /// Format citations for display.
    pub fn format_citations(&self, citations: &[Citation]) -> String {
        match self.config.citation_format {
            super::config::CitationFormat::Inline => self.format_inline(citations),
            super::config::CitationFormat::Footnote => self.format_footnote(citations),
            super::config::CitationFormat::Academic => self.format_academic(citations),
        }
    }

    /// Merge citations from multiple sources, deduplicating by URL.
    pub fn merge_citations(&self, citation_groups: Vec<Vec<Citation>>) -> Vec<Citation> {
        let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut merged = Vec::new();

        for group in citation_groups {
            for citation in group {
                let key = citation
                    .source_url
                    .as_deref()
                    .unwrap_or(&citation.source_name)
                    .to_string();

                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    merged.push(citation);
                }
            }
        }

        merged.sort_by(|a, b| {
            b.reliability_score
                .partial_cmp(&a.reliability_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        merged
    }

    /// Get citations sorted by reliability.
    pub fn sorted_by_reliability<'a>(&self, citations: &'a [Citation]) -> Vec<&'a Citation> {
        let mut sorted: Vec<&Citation> = citations.iter().collect();
        sorted.sort_by(|a, b| {
            b.reliability_score
                .partial_cmp(&a.reliability_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted
    }

    fn format_inline(&self, citations: &[Citation]) -> String {
        citations
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let source = c
                    .source_url
                    .as_deref()
                    .unwrap_or(&c.source_name);
                format!("[{}: {}]", i + 1, source)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn format_footnote(&self, citations: &[Citation]) -> String {
        let references: Vec<String> = citations
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut parts = Vec::new();
                parts.push(format!("[{}]", i + 1));

                if let Some(ref title) = c.title {
                    parts.push(title.clone());
                }

                if let Some(ref url) = c.source_url {
                    parts.push(format!("({})", url));
                }

                if self.config.preserve_access_date {
                    parts.push(format!("accessed {}", c.access_date));
                }

                parts.join(" ")
            })
            .collect();

        references.join("\n")
    }

    fn format_academic(&self, citations: &[Citation]) -> String {
        citations
            .iter()
            .map(|c| {
                let mut parts = Vec::new();

                if let Some(ref title) = c.title {
                    parts.push(format!("\"{}\"", title));
                } else {
                    parts.push(format!("\"{}\"", c.source_name));
                }

                if let Some(ref url) = c.source_url {
                    parts.push(format!("<{}>", url));
                }

                if self.config.preserve_access_date {
                    parts.push(format!("[Accessed {}]", c.access_date));
                }

                parts.join(" ")
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
