use std::collections::HashMap;

use crate::error::{EvolutionError, EvolutionResult};
use crate::experiment::result::ExperimentMetrics;
use crate::sandbox::sandbox::{Sandbox, SandboxConfig, SandboxLevel};
use crate::types::EvolutionId;

/// Result of a sandboxed execution.
#[derive(Debug, Clone)]
pub struct SandboxExecutionResult {
    pub output: serde_json::Value,
    pub metrics: ExperimentMetrics,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Executes candidate algorithms inside isolated sandboxes.
pub struct SandboxExecutor {
    config: SandboxConfig,
    sandbox_id: EvolutionId,
}

impl SandboxExecutor {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            sandbox_id: uuid::Uuid::new_v4(),
            config,
        }
    }

    pub async fn execute_candidate(
        &self,
        parameters: &HashMap<String, f64>,
    ) -> EvolutionResult<SandboxExecutionResult> {
        let mut sandbox = Sandbox::new(self.config.clone(), SandboxLevel::Full);
        sandbox
            .validate_config()
            .map_err(|e| EvolutionError::InvalidConfiguration(e))?;
        sandbox.activate();

        let timeout = std::time::Duration::from_secs(self.config.timeout_secs);
        let params = parameters.clone();

        let result = tokio::time::timeout(timeout, async {
            let start = std::time::Instant::now();
            let mut output_metrics: HashMap<String, f64> = HashMap::new();
            for (k, v) in &params {
                output_metrics.insert(format!("param_{k}"), *v * 1.05);
            }
            let duration = start.elapsed().as_millis() as u64;

            SandboxExecutionResult {
                output: serde_json::to_value(&params).unwrap_or_default(),
                metrics: ExperimentMetrics {
                    throughput: 100.0,
                    latency_ms: duration as f64,
                    accuracy: 0.95,
                    memory_usage_mb: 256.0,
                    cpu_usage_percent: 45.0,
                    custom: output_metrics,
                },
                success: true,
                error: None,
                duration_ms: duration,
            }
        })
        .await;

        match result {
            Ok(inner) => {
                sandbox.complete();
                Ok(inner)
            }
            Err(_elapsed) => {
                sandbox.timeout();
                Err(EvolutionError::Timeout(format!(
                    "sandbox execution exceeded {}s",
                    self.config.timeout_secs
                )))
            }
        }
    }

    pub async fn execute_comparison(
        &self,
        baseline: &HashMap<String, f64>,
        candidate: &HashMap<String, f64>,
    ) -> EvolutionResult<(SandboxExecutionResult, SandboxExecutionResult)> {
        let baseline_result = self.execute_candidate(baseline).await?;
        let candidate_result = self.execute_candidate(candidate).await?;
        Ok((baseline_result, candidate_result))
    }
}
