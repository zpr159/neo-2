use crate::config::EvolutionConfiguration;
use crate::error::EvolutionResult;
use crate::evolution_engine::EvolutionEngine;

pub struct EvolutionEngineBuilder {
    config: EvolutionConfiguration,
    enable_benchmarks: bool,
    enable_sandbox: bool,
    enable_policy_evolution: bool,
    enable_heuristic_evolution: bool,
    enable_distributed_evolution: bool,
}

impl EvolutionEngineBuilder {
    pub fn new() -> Self {
        Self {
            config: EvolutionConfiguration::default(),
            enable_benchmarks: true,
            enable_sandbox: true,
            enable_policy_evolution: true,
            enable_heuristic_evolution: true,
            enable_distributed_evolution: false,
        }
    }

    pub fn enable_benchmarks(mut self, enable: bool) -> Self {
        self.enable_benchmarks = enable;
        self
    }

    pub fn enable_sandbox(mut self, enable: bool) -> Self {
        self.enable_sandbox = enable;
        self
    }

    pub fn enable_policy_evolution(mut self, enable: bool) -> Self {
        self.enable_policy_evolution = enable;
        self
    }

    pub fn enable_heuristic_evolution(mut self, enable: bool) -> Self {
        self.enable_heuristic_evolution = enable;
        self
    }

    pub fn enable_distributed_evolution(mut self, enable: bool) -> Self {
        self.enable_distributed_evolution = enable;
        self
    }

    pub fn with_config(mut self, config: EvolutionConfiguration) -> Self {
        self.config = config;
        self
    }

    pub fn with_max_concurrent_experiments(mut self, max: usize) -> Self {
        self.config.max_concurrent_cycles = max;
        self
    }

    pub fn with_sandbox_timeout(mut self, secs: u64) -> Self {
        self.config.analysis_timeout_secs = secs;
        self
    }

    pub fn build(self) -> EvolutionResult<EvolutionEngine> {
        EvolutionEngine::new(self.config)
    }
}

impl Default for EvolutionEngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
