
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ReasoningResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    Search,
    Calculator,
    CodeExecution,
    WebFetch,
    DatabaseQuery,
    FileReader,
    FileWriter,
    ApiCall,
    Transformation,
    Aggregation,
    Custom(String),
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Search => write!(f, "search"),
            Self::Calculator => write!(f, "calculator"),
            Self::CodeExecution => write!(f, "code_execution"),
            Self::WebFetch => write!(f, "web_fetch"),
            Self::DatabaseQuery => write!(f, "database_query"),
            Self::FileReader => write!(f, "file_reader"),
            Self::FileWriter => write!(f, "file_writer"),
            Self::ApiCall => write!(f, "api_call"),
            Self::Transformation => write!(f, "transformation"),
            Self::Aggregation => write!(f, "aggregation"),
            Self::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub id: Uuid,
    pub name: String,
    pub tool_type: ToolType,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub required_capabilities: Vec<String>,
    pub cost_estimate: f64,
    pub reliability: f32,
}

impl ToolDescriptor {
    pub fn new(
        name: String,
        tool_type: ToolType,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            tool_type,
            description,
            input_schema: serde_json::json!({}),
            output_schema: serde_json::json!({}),
            required_capabilities: Vec::new(),
            cost_estimate: 1.0,
            reliability: 0.9,
        }
    }

    pub fn with_reliability(mut self, reliability: f32) -> Self {
        self.reliability = reliability.clamp(0.0, 1.0);
        self
    }

    pub fn with_cost(mut self, cost: f64) -> Self {
        self.cost_estimate = cost;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPlanStep {
    pub tool_id: Uuid,
    pub tool_name: String,
    pub tool_type: ToolType,
    pub input: serde_json::Value,
    pub depends_on: Vec<usize>,
    pub fallback_tool_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPlan {
    pub id: Uuid,
    pub steps: Vec<ToolPlanStep>,
    pub total_cost: f64,
    pub estimated_reliability: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub step_index: usize,
    pub tool_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPlanResult {
    pub plan_id: Uuid,
    pub outputs: Vec<ToolOutput>,
    pub all_succeeded: bool,
    pub merged_output: serde_json::Value,
    pub total_latency_ms: u64,
}

#[derive(Debug)]
pub struct ToolReasoner {
    available_tools: Vec<ToolDescriptor>,
}

impl ToolReasoner {
    pub fn new() -> Self {
        Self {
            available_tools: Vec::new(),
        }
    }

    pub fn register_tool(&mut self, tool: ToolDescriptor) {
        self.available_tools.push(tool);
    }

    pub fn tools(&self) -> &[ToolDescriptor] {
        &self.available_tools
    }

    pub fn needs_tools(&self, reasoning_state: &str, query: &str) -> bool {
        let combined = format!("{reasoning_state} {query}").to_lowercase();
        self.available_tools.iter().any(|tool| {
            combined.contains(&tool.name.to_lowercase())
                || combined.contains(&tool.tool_type.to_string())
        })
    }

    pub fn generate_plan(
        &self,
        query: &str,
        required_capabilities: &[String],
    ) -> ReasoningResult<ToolPlan> {
        let matching: Vec<&ToolDescriptor> = self
            .available_tools
            .iter()
            .filter(|tool| {
                required_capabilities.is_empty()
                    || required_capabilities
                        .iter()
                        .all(|cap| tool.required_capabilities.contains(cap))
            })
            .collect();

        if matching.is_empty() && !required_capabilities.is_empty() {
            return Err(crate::error::ReasoningError::ToolPlanFailed(format!(
                "no tools match required capabilities: {:?}",
                required_capabilities
            )));
        }

        let mut steps = Vec::new();
        let mut total_cost = 0.0;
        let mut reliability_product = 1.0f32;

        for (i, tool) in matching.iter().enumerate() {
            let input = serde_json::json!({
                "query": query,
                "step": i,
            });

            let depends_on = if i > 0 { vec![i - 1] } else { vec![] };

            let fallback = self.find_fallback(tool, &matching);

            total_cost += tool.cost_estimate;
            reliability_product *= tool.reliability;

            steps.push(ToolPlanStep {
                tool_id: tool.id,
                tool_name: tool.name.clone(),
                tool_type: tool.tool_type.clone(),
                input,
                depends_on,
                fallback_tool_id: fallback,
            });
        }

        Ok(ToolPlan {
            id: Uuid::new_v4(),
            steps,
            total_cost,
            estimated_reliability: reliability_product,
            description: format!(
                "Tool plan for '{}' using {} tools",
                query,
                matching.len()
            ),
        })
    }

    fn find_fallback<'a>(
        &self,
        tool: &ToolDescriptor,
        candidates: &[&'a ToolDescriptor],
    ) -> Option<Uuid> {
        candidates
            .iter()
            .find(|c| c.tool_type == tool.tool_type && c.id != tool.id)
            .map(|c| c.id)
    }

    pub fn merge_outputs(&self, outputs: &[ToolOutput]) -> serde_json::Value {
        let successful: Vec<&ToolOutput> = outputs.iter().filter(|o| o.success).collect();

        if successful.is_empty() {
            return serde_json::json!({
                "status": "no_successful_outputs",
                "total_attempts": outputs.len(),
            });
        }

        let results: Vec<&serde_json::Value> = successful.iter().map(|o| &o.output).collect();

        serde_json::json!({
            "status": "success",
            "merged_results": results,
            "tools_used": successful.len(),
            "total_attempts": outputs.len(),
        })
    }

    pub fn recover_from_failure(
        &self,
        failed_output: &ToolOutput,
        plan: &ToolPlan,
    ) -> Option<ToolPlanStep> {
        if let Some(step) = plan.steps.get(failed_output.step_index) {
            if let Some(fallback_id) = step.fallback_tool_id {
                if let Some(fallback_tool) = self
                    .available_tools
                    .iter()
                    .find(|t| t.id == fallback_id)
                {
                    return Some(ToolPlanStep {
                        tool_id: fallback_tool.id,
                        tool_name: fallback_tool.name.clone(),
                        tool_type: fallback_tool.tool_type.clone(),
                        input: step.input.clone(),
                        depends_on: step.depends_on.clone(),
                        fallback_tool_id: None,
                    });
                }
            }
        }
        None
    }

    pub fn estimate_plan_reliability(&self, plan: &ToolPlan) -> f32 {
        plan.steps
            .iter()
            .filter_map(|step| {
                self.available_tools
                    .iter()
                    .find(|t| t.id == step.tool_id)
                    .map(|t| t.reliability)
            })
            .product()
    }
}

impl Default for ToolReasoner {
    fn default() -> Self {
        Self::new()
    }
}
