use async_trait::async_trait;

use crate::error::EvolutionResult;

/// Lifecycle hooks invoked by the evolution engine at key points.
#[async_trait]
pub trait EvolutionLifecycle: Send + Sync {
    async fn on_analysis_complete(&self) -> EvolutionResult<()> {
        Ok(())
    }
    async fn on_experiment_complete(&self) -> EvolutionResult<()> {
        Ok(())
    }
    async fn on_evaluation_complete(&self) -> EvolutionResult<()> {
        Ok(())
    }
    async fn on_deployment_complete(&self) -> EvolutionResult<()> {
        Ok(())
    }
    async fn on_rollback(&self) -> EvolutionResult<()> {
        Ok(())
    }
}
