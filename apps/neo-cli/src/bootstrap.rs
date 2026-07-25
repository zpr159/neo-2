use std::sync::Arc;
use std::time::Instant;

use neo_executive::{ExecutionMode, ExecutiveApi};
use neo_knowledge_graph::NeoKnowledgeGraph;
use neo_memory::{CognitiveMemoryManager, UnifiedMemoryConfig};
use neo_reasoning::{ReasoningConfig, ReasoningOrchestrator};
use neo_runtime::{RuntimeConfiguration, RuntimeManager, ServiceRegistration};

use crate::config::AppConfig;
use crate::error::{CliError, CliResult};

/// The complete Neo system holding all initialized subsystems.
pub(crate) struct NeoSystem {
    /// Runtime manager for service lifecycle.
    pub runtime: Arc<RuntimeManager>,
    /// Executive API for goal/task/session management.
    pub executive: ExecutiveApi,
    /// Cognitive memory subsystem (optional — init failure is non-fatal).
    pub memory: Option<CognitiveMemoryManager>,
    /// Knowledge graph subsystem (optional).
    pub knowledge: Option<NeoKnowledgeGraph>,
    /// Reasoning orchestrator (optional).
    pub reasoning: Option<ReasoningOrchestrator>,
    /// Inference engine (optional, wrapped in ManuallyDrop for manual shutdown).
    pub inference: Option<std::mem::ManuallyDrop<neo_inference::InferenceEngine>>,
    /// System bootstrap timestamp.
    pub start_time: Instant,
    /// Application configuration.
    pub config: AppConfig,
}

impl NeoSystem {
    /// Bootstrap the entire Neo system from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns [`CliError::Bootstrap`] if any critical subsystem fails to initialize.
    pub async fn bootstrap(config: AppConfig) -> CliResult<Self> {
        let start_time = Instant::now();

        let runtime_config = if config.core.debug {
            RuntimeConfiguration::development()
        } else if config.is_production() {
            RuntimeConfiguration::production()
        } else {
            RuntimeConfiguration::default()
        };

        let runtime = RuntimeManager::new(runtime_config);
        runtime
            .initialize()
            .map_err(|e| CliError::bootstrap(format!("failed to initialize runtime: {e}")))?;

        runtime.register_service(ServiceRegistration {
            name: "neo-runtime".to_string(),
            version: (1, 0, 0),
            dependencies: Vec::new(),
            optional_dependencies: Vec::new(),
            priority: 0,
        });

        runtime.register_service(ServiceRegistration {
            name: "neo-executive".to_string(),
            version: (1, 0, 0),
            dependencies: vec![],
            optional_dependencies: Vec::new(),
            priority: 1,
        });

        runtime
            .validate_dependencies()
            .map_err(|e| CliError::bootstrap(format!("dependency validation failed: {e}")))?;

        runtime
            .start()
            .map_err(|e| CliError::bootstrap(format!("failed to start runtime: {e}")))?;

        tracing::info!("runtime initialized and started");

        let executive = ExecutiveApi::new(ExecutionMode::Interactive);

        let memory = match CognitiveMemoryManager::new(UnifiedMemoryConfig::default()) {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!("cognitive memory manager failed to initialize: {e}");
                None
            }
        };

        let knowledge = Some(NeoKnowledgeGraph::new());

        let reasoning = Some(ReasoningOrchestrator::new(ReasoningConfig::default()));

        let inference = Some(std::mem::ManuallyDrop::new(
            neo_inference::InferenceEngine::new(neo_inference::EngineConfig::default()),
        ));

        let runtime = Arc::new(runtime);

        let elapsed = start_time.elapsed();
        tracing::info!(
            bootstrap_ms = elapsed.as_millis() as u64,
            "neo system bootstrap complete"
        );

        Ok(Self {
            runtime,
            executive,
            memory,
            knowledge,
            reasoning,
            inference,
            start_time,
            config,
        })
    }

    /// Gracefully shut down all subsystems.
    pub fn shutdown(&self) {
        tracing::info!("neo system shutting down");

        if let Some(ref engine) = self.inference {
            if tokio::runtime::Handle::try_current().is_err() {
                if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    let _ = rt.block_on(engine.shutdown());
                }
            }
        }

        if let Err(e) = self.runtime.shutdown() {
            tracing::error!("runtime shutdown error: {e}");
        }

        tracing::info!("neo system shutdown complete");
    }

    /// Return a list of module names and whether they are active.
    pub(crate) fn module_status(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("Runtime", self.runtime.is_running()),
            ("Executive", true),
            ("Memory", self.memory.is_some()),
            ("Knowledge Graph", self.knowledge.is_some()),
            ("Reasoning", self.reasoning.is_some()),
            ("Inference", self.inference.is_some()),
        ]
    }
}

impl Drop for NeoSystem {
    fn drop(&mut self) {
        self.shutdown();
    }
}
