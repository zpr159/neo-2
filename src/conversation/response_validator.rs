use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::types::*;
use crate::language::types::GenerationResponse;

/// A validated response ready for delivery.
#[derive(Debug, Clone)]
pub struct ValidatedResponse {
    pub text: String,
    pub warnings: Vec<String>,
    pub citations: Vec<Citation>,
    pub format_applied: ResponseFormat,
    pub safety_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub reference: String,
    pub source: String,
    pub confidence: f32,
}

/// Validates and refines responses before delivery.
pub struct ResponseValidator;

impl ResponseValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate a generated response and apply refinements.
    pub fn validate(
        &self,
        response: &GenerationResponse,
        context: &ConversationContext,
        format: Option<ResponseFormat>,
    ) -> ConversationResult<ValidatedResponse> {
        let mut warnings = Vec::new();

        // Consistency validation
        let consistency_warnings = self.validate_consistency(&response.text, context);
        warnings.extend(consistency_warnings);

        // Citation extraction
        let citations = self.extract_citations(&response.text);

        // Safety checks
        let safety_passed = self.safety_check(&response.text);

        // Format normalization
        let format_applied = format.unwrap_or(ResponseFormat::Markdown);

        Ok(ValidatedResponse {
            text: response.text.clone(),
            warnings,
            citations,
            format_applied,
            safety_passed,
        })
    }

    fn validate_consistency(&self, text: &str, _context: &ConversationContext) -> Vec<String> {
        let mut warnings = Vec::new();

        if text.contains("I don't know") || text.contains("I cannot") {
            warnings.push("Response contains uncertainty markers".to_string());
        }

        if text.len() < 10 {
            warnings.push("Response is very short".to_string());
        }

        warnings
    }

    pub fn extract_citations(&self, text: &str) -> Vec<Citation> {
        let mut citations = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        for line in &lines {
            if line.starts_with("[") && line.contains("]") {
                let content = line.trim_start_matches('[').trim_end_matches(']').trim();
                citations.push(Citation {
                    reference: content.to_string(),
                    source: "context".to_string(),
                    confidence: 0.8,
                });
            }
        }
        citations
    }

    fn safety_check(&self, text: &str) -> bool {
        let dangerous_patterns = [
            "rm -rf",
            "drop table",
            "delete from",
            "sudo rm",
            "format c:",
            "shutdown",
            "eval(",
            "exec(",
        ];

        let text_lower = text.to_lowercase();
        for pattern in &dangerous_patterns {
            if text_lower.contains(pattern) {
                return false;
            }
        }
        true
    }

    /// Insert citations into the response text.
    pub fn insert_citations(text: &str, citations: &[Citation]) -> String {
        if citations.is_empty() {
            return text.to_string();
        }
        let mut result = text.to_string();
        for citation in citations {
            result.push_str(&format!("\n\n[Source: {}]", citation.reference));
        }
        result
    }

    /// Normalize markdown formatting.
    pub fn normalize_markdown(text: &str) -> String {
        let mut result = text.to_string();

        // Ensure code blocks are closed
        let open_count = result.matches("```").count();
        if open_count % 2 != 0 {
            result.push_str("\n```");
        }

        result
    }
}

impl Default for ResponseValidator {
    fn default() -> Self {
        Self::new()
    }
}
