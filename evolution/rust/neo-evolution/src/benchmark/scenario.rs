use std::collections::HashMap;

use crate::types::{EvolutionId, SubsystemTarget};

use super::suite::BenchmarkScenario;

/// Builder for constructing [`BenchmarkScenario`] instances with a fluent
/// API.
#[derive(Debug)]
pub struct ScenarioBuilder {
    name: String,
    description: String,
    target: SubsystemTarget,
    parameters: HashMap<String, f64>,
    iterations: usize,
}

impl ScenarioBuilder {
    /// Start building a scenario with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            target: SubsystemTarget::Core,
            parameters: HashMap::new(),
            iterations: 10,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the target subsystem.
    pub fn with_target(mut self, target: SubsystemTarget) -> Self {
        self.target = target;
        self
    }

    /// Add a named parameter.
    pub fn with_parameter(mut self, key: impl Into<String>, value: f64) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    /// Set the number of iterations.
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    /// Consume the builder and return a [`BenchmarkScenario`].
    pub fn build(self) -> BenchmarkScenario {
        BenchmarkScenario {
            id: EvolutionId::new_v4(),
            name: self.name,
            description: self.description,
            target: self.target,
            parameters: self.parameters,
            iterations: self.iterations,
        }
    }
}

// ---------------------------------------------------------------------------
// Pre-defined scenarios
// ---------------------------------------------------------------------------

impl ScenarioBuilder {
    /// CPU-bound stress scenario.
    pub fn cpu_stress() -> BenchmarkScenario {
        Self::new("cpu_stress")
            .with_description("Stresses CPU with computation-heavy workload")
            .with_target(SubsystemTarget::Core)
            .with_parameter("base_latency_ms", 50.0)
            .with_parameter("jitter_ms", 10.0)
            .with_iterations(50)
            .build()
    }

    /// Memory-bound stress scenario.
    pub fn memory_stress() -> BenchmarkScenario {
        Self::new("memory_stress")
            .with_description("Stresses memory allocation and deallocation")
            .with_target(SubsystemTarget::Memory)
            .with_parameter("base_latency_ms", 30.0)
            .with_parameter("jitter_ms", 8.0)
            .with_iterations(40)
            .build()
    }

    /// Disk I/O stress scenario.
    pub fn io_stress() -> BenchmarkScenario {
        Self::new("io_stress")
            .with_description("Stresses disk I/O with sequential and random writes")
            .with_target(SubsystemTarget::Runtime)
            .with_parameter("base_latency_ms", 80.0)
            .with_parameter("jitter_ms", 20.0)
            .with_iterations(30)
            .build()
    }

    /// Network latency test scenario.
    pub fn latency_test() -> BenchmarkScenario {
        Self::new("latency_test")
            .with_description("Measures end-to-end network latency under load")
            .with_target(SubsystemTarget::Distributed)
            .with_parameter("base_latency_ms", 5.0)
            .with_parameter("jitter_ms", 1.0)
            .with_iterations(100)
            .build()
    }

    /// Throughput measurement scenario.
    pub fn throughput_test() -> BenchmarkScenario {
        Self::new("throughput_test")
            .with_description("Measures maximum sustainable throughput")
            .with_target(SubsystemTarget::Workflows)
            .with_parameter("base_latency_ms", 2.0)
            .with_parameter("jitter_ms", 0.5)
            .with_parameter("target_throughput", 10_000.0)
            .with_iterations(200)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_produces_scenario() {
        let scenario = ScenarioBuilder::new("custom")
            .with_description("A custom test")
            .with_target(SubsystemTarget::Reasoning)
            .with_parameter("alpha", 1.0)
            .with_iterations(25)
            .build();

        assert_eq!(scenario.name, "custom");
        assert_eq!(scenario.description, "A custom test");
        assert_eq!(scenario.target, SubsystemTarget::Reasoning);
        assert_eq!(scenario.parameters.len(), 1);
        assert_eq!(scenario.iterations, 25);
    }

    #[test]
    fn predefined_scenarios_compile_and_build() {
        let s1 = ScenarioBuilder::cpu_stress();
        assert_eq!(s1.name, "cpu_stress");
        let s2 = ScenarioBuilder::memory_stress();
        assert_eq!(s2.name, "memory_stress");
        let s3 = ScenarioBuilder::io_stress();
        assert_eq!(s3.name, "io_stress");
        let s4 = ScenarioBuilder::latency_test();
        assert_eq!(s4.name, "latency_test");
        let s5 = ScenarioBuilder::throughput_test();
        assert_eq!(s5.name, "throughput_test");
    }
}
