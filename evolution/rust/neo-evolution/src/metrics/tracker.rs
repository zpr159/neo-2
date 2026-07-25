use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{DateTime, Utc};
use serde::de::Deserializer;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

// ---------------------------------------------------------------------------
// Metric structs
// ---------------------------------------------------------------------------

/// High-level evolution counters.
#[derive(Debug)]
pub struct EvolutionMetrics {
    /// Number of improvements that were successfully applied.
    pub successful_improvements: AtomicU64,
    /// Number of experiments that failed.
    pub failed_experiments: AtomicU64,
    /// Number of rollbacks performed.
    pub rollbacks: AtomicU64,
    /// Total experiments that have been initiated.
    pub total_experiments: AtomicU64,
    /// Total benchmarks that have been run.
    pub total_benchmarks: AtomicU64,
}

impl Serialize for EvolutionMetrics {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("EvolutionMetrics", 5)?;
        state.serialize_field("successful_improvements", &self.successful_improvements)?;
        state.serialize_field("failed_experiments", &self.failed_experiments)?;
        state.serialize_field("rollbacks", &self.rollbacks)?;
        state.serialize_field("total_experiments", &self.total_experiments)?;
        state.serialize_field("total_benchmarks", &self.total_benchmarks)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for EvolutionMetrics {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            successful_improvements: u64,
            failed_experiments: u64,
            rollbacks: u64,
            total_experiments: u64,
            total_benchmarks: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(Self {
            successful_improvements: AtomicU64::new(h.successful_improvements),
            failed_experiments: AtomicU64::new(h.failed_experiments),
            rollbacks: AtomicU64::new(h.rollbacks),
            total_experiments: AtomicU64::new(h.total_experiments),
            total_benchmarks: AtomicU64::new(h.total_benchmarks),
        })
    }
}

/// Optimisation-specific counters.
#[derive(Debug)]
pub struct OptimizationMetrics {
    /// Optimisation attempts that were made.
    pub optimizations_attempted: AtomicU64,
    /// Optimisation attempts that succeeded.
    pub optimizations_succeeded: AtomicU64,
    /// Running average improvement percentage (stored as integer × 100).
    pub avg_improvement_percent: AtomicU64,
    /// Aggregate resource savings (arbitrary unit).
    pub total_resource_savings: AtomicU64,
}

impl Serialize for OptimizationMetrics {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("OptimizationMetrics", 4)?;
        state.serialize_field("optimizations_attempted", &self.optimizations_attempted)?;
        state.serialize_field("optimizations_succeeded", &self.optimizations_succeeded)?;
        state.serialize_field("avg_improvement_percent", &self.avg_improvement_percent)?;
        state.serialize_field("total_resource_savings", &self.total_resource_savings)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for OptimizationMetrics {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            optimizations_attempted: u64,
            optimizations_succeeded: u64,
            avg_improvement_percent: u64,
            total_resource_savings: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(Self {
            optimizations_attempted: AtomicU64::new(h.optimizations_attempted),
            optimizations_succeeded: AtomicU64::new(h.optimizations_succeeded),
            avg_improvement_percent: AtomicU64::new(h.avg_improvement_percent),
            total_resource_savings: AtomicU64::new(h.total_resource_savings),
        })
    }
}

/// Experiment-level counters.
#[derive(Debug)]
pub struct ExperimentMetrics {
    /// Total experiments initiated.
    pub total_experiments: AtomicU64,
    /// Experiments that completed successfully.
    pub successful: AtomicU64,
    /// Experiments that failed.
    pub failed: AtomicU64,
    /// Average duration in milliseconds (stored as integer).
    pub avg_duration_ms: AtomicU64,
}

impl Serialize for ExperimentMetrics {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("ExperimentMetrics", 4)?;
        state.serialize_field("total_experiments", &self.total_experiments)?;
        state.serialize_field("successful", &self.successful)?;
        state.serialize_field("failed", &self.failed)?;
        state.serialize_field("avg_duration_ms", &self.avg_duration_ms)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ExperimentMetrics {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            total_experiments: u64,
            successful: u64,
            failed: u64,
            avg_duration_ms: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(Self {
            total_experiments: AtomicU64::new(h.total_experiments),
            successful: AtomicU64::new(h.successful),
            failed: AtomicU64::new(h.failed),
            avg_duration_ms: AtomicU64::new(h.avg_duration_ms),
        })
    }
}

/// Benchmark-specific counters.
#[derive(Debug)]
pub struct BenchmarkMetrics {
    /// Total benchmarks executed.
    pub total_benchmarks: AtomicU64,
    /// Regressions detected during benchmarking.
    pub regressions_detected: AtomicU64,
    /// Improvements detected during benchmarking.
    pub improvements_detected: AtomicU64,
}

impl Serialize for BenchmarkMetrics {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("BenchmarkMetrics", 3)?;
        state.serialize_field("total_benchmarks", &self.total_benchmarks)?;
        state.serialize_field("regressions_detected", &self.regressions_detected)?;
        state.serialize_field("improvements_detected", &self.improvements_detected)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for BenchmarkMetrics {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Helper {
            total_benchmarks: u64,
            regressions_detected: u64,
            improvements_detected: u64,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(Self {
            total_benchmarks: AtomicU64::new(h.total_benchmarks),
            regressions_detected: AtomicU64::new(h.regressions_detected),
            improvements_detected: AtomicU64::new(h.improvements_detected),
        })
    }
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// Human-readable snapshot of all key metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionSummary {
    /// Successful improvements applied.
    pub successful_improvements: u64,
    /// Failed experiments.
    pub failed_experiments: u64,
    /// Rollbacks performed.
    pub rollbacks: u64,
    /// Total experiments initiated.
    pub total_experiments: u64,
    /// Total benchmarks run.
    pub total_benchmarks: u64,
    /// Optimisation attempts.
    pub optimizations_attempted: u64,
    /// Successful optimisations.
    pub optimizations_succeeded: u64,
    /// Average improvement percentage (× 100).
    pub avg_improvement_percent: u64,
    /// Total resource savings.
    pub total_resource_savings: u64,
    /// Experiment success count.
    pub experiment_successful: u64,
    /// Experiment failure count.
    pub experiment_failed: u64,
    /// Average experiment duration in ms.
    pub experiment_avg_duration_ms: u64,
    /// Benchmarks total count.
    pub benchmark_total: u64,
    /// Regressions detected.
    pub benchmark_regressions: u64,
    /// Improvements detected.
    pub benchmark_improvements: u64,
    /// When the tracker was created.
    pub started_at: DateTime<Utc>,
    /// When this summary was generated.
    pub generated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// MetricsTracker
// ---------------------------------------------------------------------------

/// Central metrics tracker that aggregates counters across all subsystems.
#[derive(Debug)]
pub struct MetricsTracker {
    /// Evolution-level counters.
    pub evolution: EvolutionMetrics,
    /// Optimisation-level counters.
    pub optimization: OptimizationMetrics,
    /// Experiment-level counters.
    pub experiment: ExperimentMetrics,
    /// Benchmark-level counters.
    pub benchmark: BenchmarkMetrics,
    /// When tracking began.
    started_at: DateTime<Utc>,
}

impl MetricsTracker {
    /// Create a new tracker with all counters initialised to zero.
    pub fn new() -> Self {
        Self {
            evolution: EvolutionMetrics {
                successful_improvements: AtomicU64::new(0),
                failed_experiments: AtomicU64::new(0),
                rollbacks: AtomicU64::new(0),
                total_experiments: AtomicU64::new(0),
                total_benchmarks: AtomicU64::new(0),
            },
            optimization: OptimizationMetrics {
                optimizations_attempted: AtomicU64::new(0),
                optimizations_succeeded: AtomicU64::new(0),
                avg_improvement_percent: AtomicU64::new(0),
                total_resource_savings: AtomicU64::new(0),
            },
            experiment: ExperimentMetrics {
                total_experiments: AtomicU64::new(0),
                successful: AtomicU64::new(0),
                failed: AtomicU64::new(0),
                avg_duration_ms: AtomicU64::new(0),
            },
            benchmark: BenchmarkMetrics {
                total_benchmarks: AtomicU64::new(0),
                regressions_detected: AtomicU64::new(0),
                improvements_detected: AtomicU64::new(0),
            },
            started_at: Utc::now(),
        }
    }

    /// Record a successful improvement.
    pub fn record_improvement_success(&self) {
        self.evolution
            .successful_improvements
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a failed experiment.
    pub fn record_improvement_failure(&self) {
        self.evolution
            .failed_experiments
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rollback.
    pub fn record_rollback(&self) {
        self.evolution.rollbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// Record the completion of an experiment.
    pub fn record_experiment(&self, success: bool, duration_ms: f64) {
        self.evolution
            .total_experiments
            .fetch_add(1, Ordering::Relaxed);
        self.experiment
            .total_experiments
            .fetch_add(1, Ordering::Relaxed);

        if success {
            self.experiment.successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.experiment.failed.fetch_add(1, Ordering::Relaxed);
        }

        // Update rolling average duration.
        let prev = self.experiment.avg_duration_ms.load(Ordering::Relaxed);
        let count = self.experiment.total_experiments.load(Ordering::Relaxed);
        let new_avg = if count > 0 {
            let prev_total = prev * (count - 1);
            (prev_total + duration_ms as u64) / count
        } else {
            duration_ms as u64
        };
        self.experiment
            .avg_duration_ms
            .store(new_avg, Ordering::Relaxed);
    }

    /// Record a benchmark completion.
    pub fn record_benchmark(&self) {
        self.evolution
            .total_benchmarks
            .fetch_add(1, Ordering::Relaxed);
        self.benchmark
            .total_benchmarks
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record an optimisation attempt.
    pub fn record_optimization(&self, success: bool, improvement_percent: f64) {
        self.optimization
            .optimizations_attempted
            .fetch_add(1, Ordering::Relaxed);

        if success {
            self.optimization
                .optimizations_succeeded
                .fetch_add(1, Ordering::Relaxed);
        }

        // Update rolling average improvement (stored as integer × 100).
        let prev = self
            .optimization
            .avg_improvement_percent
            .load(Ordering::Relaxed);
        let attempts = self
            .optimization
            .optimizations_attempted
            .load(Ordering::Relaxed);
        let new_avg = if attempts > 0 {
            let prev_total = prev * (attempts - 1);
            let new_val = (improvement_percent * 100.0) as u64;
            (prev_total + new_val) / attempts
        } else {
            (improvement_percent * 100.0) as u64
        };
        self.optimization
            .avg_improvement_percent
            .store(new_avg, Ordering::Relaxed);
    }

    /// Generate a point-in-time summary of all tracked metrics.
    pub fn get_evolution_summary(&self) -> EvolutionSummary {
        EvolutionSummary {
            successful_improvements: self
                .evolution
                .successful_improvements
                .load(Ordering::Relaxed),
            failed_experiments: self.evolution.failed_experiments.load(Ordering::Relaxed),
            rollbacks: self.evolution.rollbacks.load(Ordering::Relaxed),
            total_experiments: self.evolution.total_experiments.load(Ordering::Relaxed),
            total_benchmarks: self.evolution.total_benchmarks.load(Ordering::Relaxed),
            optimizations_attempted: self
                .optimization
                .optimizations_attempted
                .load(Ordering::Relaxed),
            optimizations_succeeded: self
                .optimization
                .optimizations_succeeded
                .load(Ordering::Relaxed),
            avg_improvement_percent: self
                .optimization
                .avg_improvement_percent
                .load(Ordering::Relaxed),
            total_resource_savings: self
                .optimization
                .total_resource_savings
                .load(Ordering::Relaxed),
            experiment_successful: self.experiment.successful.load(Ordering::Relaxed),
            experiment_failed: self.experiment.failed.load(Ordering::Relaxed),
            experiment_avg_duration_ms: self.experiment.avg_duration_ms.load(Ordering::Relaxed),
            benchmark_total: self.benchmark.total_benchmarks.load(Ordering::Relaxed),
            benchmark_regressions: self.benchmark.regressions_detected.load(Ordering::Relaxed),
            benchmark_improvements: self.benchmark.improvements_detected.load(Ordering::Relaxed),
            started_at: self.started_at,
            generated_at: Utc::now(),
        }
    }

    /// Record that a benchmark regression was detected.
    pub fn record_regression(&self) {
        self.benchmark
            .regressions_detected
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record that a benchmark improvement was detected.
    pub fn record_benchmark_improvement(&self) {
        self.benchmark
            .improvements_detected
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Add to the total resource savings counter.
    pub fn add_resource_savings(&self, amount: u64) {
        self.optimization
            .total_resource_savings
            .fetch_add(amount, Ordering::Relaxed);
    }

    /// Return all raw atomic values as a serialisable snapshot.
    pub fn get_all_metrics(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            evolution: self.evolution(),
            optimization: self.optimization(),
            experiment: self.experiment(),
            benchmark: self.benchmark(),
            started_at: self.started_at,
        }
    }

    fn evolution(&self) -> EvolutionSnapshotInner {
        EvolutionSnapshotInner {
            successful_improvements: self
                .evolution
                .successful_improvements
                .load(Ordering::Relaxed),
            failed_experiments: self.evolution.failed_experiments.load(Ordering::Relaxed),
            rollbacks: self.evolution.rollbacks.load(Ordering::Relaxed),
            total_experiments: self.evolution.total_experiments.load(Ordering::Relaxed),
            total_benchmarks: self.evolution.total_benchmarks.load(Ordering::Relaxed),
        }
    }

    fn optimization(&self) -> OptimizationSnapshotInner {
        OptimizationSnapshotInner {
            optimizations_attempted: self
                .optimization
                .optimizations_attempted
                .load(Ordering::Relaxed),
            optimizations_succeeded: self
                .optimization
                .optimizations_succeeded
                .load(Ordering::Relaxed),
            avg_improvement_percent: self
                .optimization
                .avg_improvement_percent
                .load(Ordering::Relaxed),
            total_resource_savings: self
                .optimization
                .total_resource_savings
                .load(Ordering::Relaxed),
        }
    }

    fn experiment(&self) -> ExperimentSnapshotInner {
        ExperimentSnapshotInner {
            total_experiments: self.experiment.total_experiments.load(Ordering::Relaxed),
            successful: self.experiment.successful.load(Ordering::Relaxed),
            failed: self.experiment.failed.load(Ordering::Relaxed),
            avg_duration_ms: self.experiment.avg_duration_ms.load(Ordering::Relaxed),
        }
    }

    fn benchmark(&self) -> BenchmarkSnapshotInner {
        BenchmarkSnapshotInner {
            total_benchmarks: self.benchmark.total_benchmarks.load(Ordering::Relaxed),
            regressions_detected: self.benchmark.regressions_detected.load(Ordering::Relaxed),
            improvements_detected: self.benchmark.improvements_detected.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Snapshot types (plain data, easy to serialise)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvolutionSnapshotInner {
    successful_improvements: u64,
    failed_experiments: u64,
    rollbacks: u64,
    total_experiments: u64,
    total_benchmarks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizationSnapshotInner {
    optimizations_attempted: u64,
    optimizations_succeeded: u64,
    avg_improvement_percent: u64,
    total_resource_savings: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExperimentSnapshotInner {
    total_experiments: u64,
    successful: u64,
    failed: u64,
    avg_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BenchmarkSnapshotInner {
    total_benchmarks: u64,
    regressions_detected: u64,
    improvements_detected: u64,
}

/// Serialisable snapshot of every metric category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    evolution: EvolutionSnapshotInner,
    optimization: OptimizationSnapshotInner,
    experiment: ExperimentSnapshotInner,
    benchmark: BenchmarkSnapshotInner,
    started_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_creation() {
        let t = MetricsTracker::new();
        let s = t.get_evolution_summary();
        assert_eq!(s.successful_improvements, 0);
        assert_eq!(s.total_experiments, 0);
    }

    #[test]
    fn record_improvement() {
        let t = MetricsTracker::new();
        t.record_improvement_success();
        t.record_improvement_success();
        t.record_improvement_failure();
        let s = t.get_evolution_summary();
        assert_eq!(s.successful_improvements, 2);
        assert_eq!(s.failed_experiments, 1);
    }

    #[test]
    fn record_experiment_updates_avg() {
        let t = MetricsTracker::new();
        t.record_experiment(true, 100.0);
        t.record_experiment(true, 200.0);
        let s = t.get_evolution_summary();
        assert_eq!(s.experiment_successful, 2);
        assert_eq!(s.experiment_avg_duration_ms, 150);
    }

    #[test]
    fn record_optimization_updates_avg() {
        let t = MetricsTracker::new();
        t.record_optimization(true, 10.0);
        t.record_optimization(true, 20.0);
        let s = t.get_evolution_summary();
        assert_eq!(s.optimizations_attempted, 2);
        assert_eq!(s.optimizations_succeeded, 2);
        // avg = ((10*100 + 20*100) / 2) = 1500
        assert_eq!(s.avg_improvement_percent, 1500);
    }

    #[test]
    fn record_rollback() {
        let t = MetricsTracker::new();
        t.record_rollback();
        t.record_rollback();
        assert_eq!(t.get_evolution_summary().rollbacks, 2);
    }

    #[test]
    fn summary_serialises() {
        let t = MetricsTracker::new();
        t.record_improvement_success();
        t.record_benchmark();
        let json = serde_json::to_string(&t.get_evolution_summary()).unwrap();
        assert!(json.contains("successful_improvements"));
    }

    #[test]
    fn get_all_metrics_works() {
        let t = MetricsTracker::new();
        t.record_improvement_success();
        let snap = t.get_all_metrics();
        assert_eq!(snap.evolution.successful_improvements, 1);
    }

    #[test]
    fn record_regression_and_improvement() {
        let t = MetricsTracker::new();
        t.record_regression();
        t.record_regression();
        t.record_benchmark_improvement();
        let s = t.get_evolution_summary();
        assert_eq!(s.benchmark_regressions, 2);
        assert_eq!(s.benchmark_improvements, 1);
    }

    #[test]
    fn add_resource_savings() {
        let t = MetricsTracker::new();
        t.add_resource_savings(500);
        t.add_resource_savings(300);
        assert_eq!(
            t.optimization
                .total_resource_savings
                .load(Ordering::Relaxed),
            800
        );
    }
}
