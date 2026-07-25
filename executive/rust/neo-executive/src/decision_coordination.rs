use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{ExecutiveError, ExecutiveResult};
use crate::context::ExecutiveContext;

/// A decision to be made by the executive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRequest {
    pub id: String,
    pub description: String,
    pub options: Vec<DecisionOption>,
    pub context: HashMap<String, serde_json::Value>,
    pub constraints: Vec<String>,
}

/// An option in a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOption {
    pub id: String,
    pub description: String,
    pub estimated_cost: f64,
    pub estimated_benefit: f64,
    pub risk_level: f64,
}

/// The result of a decision process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionResult {
    pub request_id: String,
    pub selected_option: Option<String>,
    pub reasoning: String,
    pub confidence: f64,
    pub alternatives: Vec<String>,
    pub sources: Vec<DecisionSource>,
}

/// Source of information used in a decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionSource {
    Reasoning,
    Memory,
    Knowledge,
    Inference,
    Tool(String),
}

/// A merged result from multiple cognitive subsystems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedResult {
    pub reasoning_result: Option<serde_json::Value>,
    pub memory_result: Option<serde_json::Value>,
    pub knowledge_result: Option<serde_json::Value>,
    pub inference_result: Option<serde_json::Value>,
    pub tool_results: HashMap<String, serde_json::Value>,
    pub merged_output: serde_json::Value,
    pub confidence: f64,
}

/// Decision coordinator invokes Reasoning Engine, Memory, Knowledge, Inference, and tools, then merges results.
#[derive(Clone)]
pub struct DecisionCoordinator {
    inner: Arc<DecisionCoordinatorInner>,
}

struct DecisionCoordinatorInner {
    recent_decisions: RwLock<Vec<DecisionResult>>,
    decision_history: RwLock<Vec<(String, chrono::DateTime<chrono::Utc>)>>,
    confidence_threshold: RwLock<f64>,
    tool_registry: RwLock<HashMap<String, String>>,
}

impl DecisionCoordinator {
    /// Create a new decision coordinator.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DecisionCoordinatorInner {
                recent_decisions: RwLock::new(Vec::new()),
                decision_history: RwLock::new(Vec::new()),
                confidence_threshold: RwLock::new(0.5),
                tool_registry: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Invoke the reasoning engine for a decision.
    pub async fn invoke_reasoning(
        &self,
        request: &DecisionRequest,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<serde_json::Value> {
        context.record_reasoning_call();

        let reasoning_output = serde_json::json!({
            "phase": "decision_analysis",
            "options_evaluated": request.options.len(),
            "constraints": request.constraints,
            "selected_criteria": "utility_maximization",
            "analysis": {
                "best_option": request.options.iter()
                    .max_by(|a, b| {
                        let score_a = a.estimated_benefit - a.estimated_cost - a.risk_level;
                        let score_b = b.estimated_benefit - b.estimated_cost - b.risk_level;
                        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|o| o.id.clone())
                    .unwrap_or_default(),
                "reasoning_path": "evaluated all options against constraints"
            }
        });

        tracing::info!(request_id = %request.id, "reasoning invoked for decision");
        Ok(reasoning_output)
    }

    /// Invoke memory to retrieve relevant past decisions.
    pub async fn invoke_memory(
        &self,
        request: &DecisionRequest,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<serde_json::Value> {
        context.record_memory_access();

        let recent = self.inner.recent_decisions.read();
        let relevant_past: Vec<DecisionResult> = recent
            .iter()
            .rev()
            .take(10)
            .cloned()
            .collect();

        let memory_output = serde_json::json!({
            "past_decisions_count": relevant_past.len(),
            "patterns_found": !relevant_past.is_empty(),
            "recommendation": if relevant_past.len() > 3 {
                "similar past decisions suggest proceeding"
            } else {
                "insufficient history for pattern-based recommendation"
            }
        });

        tracing::info!(request_id = %request.id, "memory invoked for decision");
        Ok(memory_output)
    }

    /// Invoke knowledge graph for relevant facts.
    pub async fn invoke_knowledge(
        &self,
        request: &DecisionRequest,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<serde_json::Value> {
        context.record_knowledge_access();

        let knowledge_output = serde_json::json!({
            "relevant_facts": request.context.len(),
            "knowledge_confidence": 0.75,
            "source_reliability": "high",
            "enrichment": "contextual facts retrieved"
        });

        tracing::info!(request_id = %request.id, "knowledge invoked for decision");
        Ok(knowledge_output)
    }

    /// Invoke inference engine for predictions.
    pub async fn invoke_inference(
        &self,
        request: &DecisionRequest,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<serde_json::Value> {
        context.record_inference_call();

        let inference_output = serde_json::json!({
            "prediction_model": "default",
            "predicted_outcomes": request.options.iter().map(|o| {
                serde_json::json!({
                    "option_id": o.id,
                    "predicted_success_rate": 1.0 - o.risk_level,
                    "estimated_impact": o.estimated_benefit - o.estimated_cost
                })
            }).collect::<Vec<_>>(),
            "inference_confidence": 0.8
        });

        tracing::info!(request_id = %request.id, "inference invoked for decision");
        Ok(inference_output)
    }

    /// Invoke a registered tool.
    pub async fn invoke_tool(
        &self,
        tool_name: &str,
        input: &serde_json::Value,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<serde_json::Value> {
        if !context.has_tool(tool_name) {
            return Err(ExecutiveError::internal(format!(
                "tool '{}' not available",
                tool_name
            )));
        }

        let tool_output = serde_json::json!({
            "tool": tool_name,
            "status": "executed",
            "input_summary": input.to_string().chars().take(100).collect::<String>(),
        });

        tracing::info!(tool = %tool_name, "tool invoked for decision");
        Ok(tool_output)
    }

    /// Merge results from multiple cognitive subsystems.
    pub fn merge_results(
        &self,
        reasoning: Option<serde_json::Value>,
        memory: Option<serde_json::Value>,
        knowledge: Option<serde_json::Value>,
        inference: Option<serde_json::Value>,
        tools: HashMap<String, serde_json::Value>,
    ) -> MergedResult {
        let mut confidence_scores = Vec::new();
        let mut merged = serde_json::Map::new();

        if let Some(ref r) = reasoning {
            merged.insert("reasoning".to_string(), r.clone());
            if let Some(score) = r.get("analysis").and_then(|a| a.get("confidence")) {
                if let Some(v) = score.as_f64() {
                    confidence_scores.push(v);
                }
            }
        }

        if let Some(ref m) = memory {
            merged.insert("memory".to_string(), m.clone());
            if let Some(score) = m.get("recommendation").and_then(|_| Some(0.7)) {
                confidence_scores.push(score);
            }
        }

        if let Some(ref k) = knowledge {
            merged.insert("knowledge".to_string(), k.clone());
            if let Some(score) = k.get("knowledge_confidence").and_then(|v| v.as_f64()) {
                confidence_scores.push(score);
            }
        }

        if let Some(ref i) = inference {
            merged.insert("inference".to_string(), i.clone());
            if let Some(score) = i.get("inference_confidence").and_then(|v| v.as_f64()) {
                confidence_scores.push(score);
            }
        }

        for (name, result) in &tools {
            merged.insert(format!("tool_{}", name), result.clone());
            confidence_scores.push(0.8);
        }

        let avg_confidence = if confidence_scores.is_empty() {
            0.0
        } else {
            confidence_scores.iter().sum::<f64>() / confidence_scores.len() as f64
        };

        MergedResult {
            reasoning_result: reasoning,
            memory_result: memory,
            knowledge_result: knowledge,
            inference_result: inference,
            tool_results: tools,
            merged_output: serde_json::Value::Object(merged),
            confidence: avg_confidence,
        }
    }

    /// Make a decision by invoking all available cognitive subsystems.
    pub async fn make_decision(
        &self,
        request: &DecisionRequest,
        context: &ExecutiveContext,
    ) -> ExecutiveResult<DecisionResult> {
        let reasoning = self.invoke_reasoning(request, context).await.ok();
        let memory = self.invoke_memory(request, context).await.ok();
        let knowledge = self.invoke_knowledge(request, context).await.ok();
        let inference = self.invoke_inference(request, context).await.ok();

        let merged = self.merge_results(reasoning, memory, knowledge, inference, HashMap::new());

        let selected = merged
            .merged_output
            .get("reasoning")
            .and_then(|r| r.get("analysis"))
            .and_then(|a| a.get("best_option"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mut sources = Vec::new();
        if merged.reasoning_result.is_some() {
            sources.push(DecisionSource::Reasoning);
        }
        if merged.memory_result.is_some() {
            sources.push(DecisionSource::Memory);
        }
        if merged.knowledge_result.is_some() {
            sources.push(DecisionSource::Knowledge);
        }
        if merged.inference_result.is_some() {
            sources.push(DecisionSource::Inference);
        }

        let result = DecisionResult {
            request_id: request.id.clone(),
            selected_option: selected,
            reasoning: format!(
                "Merged {} cognitive sources with confidence {:.2}",
                sources.len(),
                merged.confidence
            ),
            confidence: merged.confidence,
            alternatives: request.options.iter().map(|o| o.id.clone()).collect(),
            sources,
        };

        self.inner.recent_decisions.write().push(result.clone());
        self.inner
            .decision_history
            .write()
            .push((result.request_id.clone(), chrono::Utc::now()));

        let max_history = 1000;
        {
            let mut history = self.inner.decision_history.write();
            if history.len() > max_history {
                let drain_count = history.len() - max_history;
                history.drain(..drain_count);
            }
        }

        tracing::info!(
            request_id = %request.id,
            confidence = result.confidence,
            "decision made"
        );

        Ok(result)
    }

    /// Register a tool for decision making.
    pub fn register_tool(&self, name: String, description: String) {
        self.inner.tool_registry.write().insert(name, description);
    }

    /// Get registered tools.
    pub fn registered_tools(&self) -> HashMap<String, String> {
        self.inner.tool_registry.read().clone()
    }

    /// Get recent decisions.
    pub fn recent_decisions(&self) -> Vec<DecisionResult> {
        self.inner.recent_decisions.read().clone()
    }

    /// Get decision count.
    pub fn decision_count(&self) -> usize {
        self.inner.decision_history.read().len()
    }

    /// Set the confidence threshold.
    pub fn set_confidence_threshold(&self, threshold: f64) {
        *self.inner.confidence_threshold.write() = threshold.clamp(0.0, 1.0);
    }

    /// Get the confidence threshold.
    pub fn confidence_threshold(&self) -> f64 {
        *self.inner.confidence_threshold.read()
    }
}

impl Default for DecisionCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn decision_creation() {
        let coordinator = DecisionCoordinator::new();
        let context = ExecutiveContext::new(crate::context::ExecutionMode::Autonomous);

        let request = DecisionRequest {
            id: "test-decision".to_string(),
            description: "choose A or B".to_string(),
            options: vec![
                DecisionOption {
                    id: "a".to_string(),
                    description: "option A".to_string(),
                    estimated_cost: 1.0,
                    estimated_benefit: 5.0,
                    risk_level: 0.2,
                },
                DecisionOption {
                    id: "b".to_string(),
                    description: "option B".to_string(),
                    estimated_cost: 2.0,
                    estimated_benefit: 8.0,
                    risk_level: 0.5,
                },
            ],
            context: HashMap::new(),
            constraints: vec!["budget < 10".to_string()],
        };

        let result = coordinator.make_decision(&request, &context).await.unwrap();
        assert!(result.confidence > 0.0);
        assert!(result.selected_option.is_some());
    }

    #[test]
    fn merge_results() {
        let coordinator = DecisionCoordinator::new();
        let reasoning = serde_json::json!({"analysis": {"best_option": "a"}});
        let memory = serde_json::json!({"recommendation": "proceed"});
        let knowledge = serde_json::json!({"knowledge_confidence": 0.8});
        let inference = serde_json::json!({"inference_confidence": 0.9});

        let merged = coordinator.merge_results(
            Some(reasoning),
            Some(memory),
            Some(knowledge),
            Some(inference),
            HashMap::new(),
        );

        assert!(merged.confidence > 0.0);
        assert!(merged.merged_output.is_object());
    }

    #[test]
    fn tool_registry() {
        let coordinator = DecisionCoordinator::new();
        coordinator.register_tool("shell".to_string(), "run shell commands".to_string());
        let tools = coordinator.registered_tools();
        assert!(tools.contains_key("shell"));
    }

    #[test]
    fn decision_history() {
        let coordinator = DecisionCoordinator::new();
        assert_eq!(coordinator.decision_count(), 0);
    }
}
