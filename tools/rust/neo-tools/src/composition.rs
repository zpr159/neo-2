//! Tool composition: pipelines, sequential, parallel, and conditional execution.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::ToolResult;
use crate::executor::ToolExecutor;
use crate::types::{ToolContext, ToolId, ToolRequest};

// ---------------------------------------------------------------------------
// CompositionStep
// ---------------------------------------------------------------------------

/// A single step in a composition pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionStep {
    pub name: String,
    pub tool_name: String,
    pub operation: String,
    pub input_transform: Option<String>,
    pub condition: Option<String>,
    pub retry_count: u32,
}

impl CompositionStep {
    pub fn new(
        name: impl Into<String>,
        tool_name: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            tool_name: tool_name.into(),
            operation: operation.into(),
            input_transform: None,
            condition: None,
            retry_count: 0,
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    pub fn with_retry(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

// ---------------------------------------------------------------------------
// CompositionStrategy
// ---------------------------------------------------------------------------

/// Execution strategy for a composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositionStrategy {
    /// Execute steps one after another, threading output to next input.
    Sequential,
    /// Execute all steps in parallel.
    Parallel,
    /// Execute steps in a pipeline (parallel where possible, sequential where needed).
    Pipeline,
    /// Execute steps based on conditions.
    Conditional,
}

// ---------------------------------------------------------------------------
// Composition
// ---------------------------------------------------------------------------

/// A composed sequence of tool invocations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Composition {
    pub name: String,
    pub description: String,
    pub strategy: CompositionStrategy,
    pub steps: Vec<CompositionStep>,
    pub rollback_steps: Vec<CompositionStep>,
}

impl Composition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        strategy: CompositionStrategy,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            strategy,
            steps: Vec::new(),
            rollback_steps: Vec::new(),
        }
    }

    pub fn add_step(mut self, step: CompositionStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_rollback(mut self, step: CompositionStep) -> Self {
        self.rollback_steps.push(step);
        self
    }
}

// ---------------------------------------------------------------------------
// CompositionResult
// ---------------------------------------------------------------------------

/// Result of executing a composition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionResult {
    pub composition_name: String,
    pub success: bool,
    pub step_results: Vec<StepResult>,
    pub total_duration_ms: u64,
    pub error: Option<String>,
}

/// Result of a single composition step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_name: String,
    pub tool_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// ToolComposer — executes compositions
// ---------------------------------------------------------------------------

/// Executes tool compositions using the executor.
pub struct ToolComposer {
    executor: Arc<ToolExecutor>,
}

impl std::fmt::Debug for ToolComposer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolComposer").finish()
    }
}

impl ToolComposer {
    pub fn new(executor: Arc<ToolExecutor>) -> Self {
        Self { executor }
    }

    /// Execute a composition.
    pub async fn execute(
        &self,
        composition: &Composition,
        initial_input: serde_json::Value,
        context: &ToolContext,
    ) -> ToolResult<CompositionResult> {
        let start = std::time::Instant::now();
        let mut step_results = Vec::new();
        let mut current_input = initial_input;
        let mut all_success = true;

        for step in &composition.steps {
            if let Some(ref _condition) = step.condition {
                // Condition evaluation placeholder — always passes for now
            }

            let tool_id = ToolId::new();
            let request = ToolRequest::named(
                tool_id,
                &step.tool_name,
                &step.operation,
                current_input.clone(),
                context.clone(),
            );

            let step_start = std::time::Instant::now();
            let response = if step.retry_count > 0 {
                self.executor
                    .execute_with_retries(request, step.retry_count)
                    .await
            } else {
                self.executor.execute(request).await
            };
            let step_duration = step_start.elapsed().as_millis() as u64;

            match response {
                Ok(resp) => {
                    let success = resp.success;
                    if success {
                        current_input = resp.output.clone();
                    } else {
                        all_success = false;
                    }
                    step_results.push(StepResult {
                        step_name: step.name.clone(),
                        tool_name: step.tool_name.clone(),
                        success,
                        output: resp.output,
                        duration_ms: step_duration,
                        error: resp.error,
                    });

                    if !success && composition.strategy == CompositionStrategy::Sequential {
                        break;
                    }
                }
                Err(err) => {
                    all_success = false;
                    step_results.push(StepResult {
                        step_name: step.name.clone(),
                        tool_name: step.tool_name.clone(),
                        success: false,
                        output: serde_json::json!(null),
                        duration_ms: step_duration,
                        error: Some(err.to_string()),
                    });

                    if composition.strategy == CompositionStrategy::Sequential {
                        break;
                    }
                }
            }
        }

        let total_duration = start.elapsed().as_millis() as u64;

        if !all_success && !composition.rollback_steps.is_empty() {
            self.execute_rollback(composition, context).await;
        }

        Ok(CompositionResult {
            composition_name: composition.name.clone(),
            success: all_success,
            step_results,
            total_duration_ms: total_duration,
            error: if all_success {
                None
            } else {
                Some("one or more steps failed".into())
            },
        })
    }

    async fn execute_rollback(&self, composition: &Composition, context: &ToolContext) {
        for step in &composition.rollback_steps {
            let tool_id = ToolId::new();
            let request = ToolRequest::named(
                tool_id,
                &step.tool_name,
                &step.operation,
                serde_json::json!({}),
                context.clone(),
            );
            if let Err(err) = self.executor.execute(request).await {
                tracing::warn!(
                    composition = %composition.name,
                    step = %step.name,
                    error = %err,
                    "rollback step failed"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;
    use crate::tool::ToolBuilder;
    use crate::types::{ToolCategory, ToolType, ToolVersion};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_sequential_composition() {
        let registry = Arc::new(ToolRegistry::new());

        let tool = ToolBuilder::new(
            "echo",
            ToolVersion::new(1, 0, 0),
            "Echo tool",
            ToolType::Custom("test".into()),
            ToolCategory::Execute,
        )
        .on_execute(|params, _ctx| Box::pin(async move { Ok(params) }))
        .build()
        .unwrap();
        registry.register(tool).await.unwrap();

        let executor = Arc::new(ToolExecutor::new(Arc::clone(&registry), 5));
        let composer = ToolComposer::new(executor);

        let composition = Composition::new("test_pipe", "Test", CompositionStrategy::Sequential)
            .add_step(CompositionStep::new("step1", "echo", "echo"))
            .add_step(CompositionStep::new("step2", "echo", "echo"));

        let ctx = ToolContext::new("test", crate::types::CallerType::Internal);
        let result = composer
            .execute(&composition, serde_json::json!({"key": "value"}), &ctx)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.step_results.len(), 2);
    }
}
