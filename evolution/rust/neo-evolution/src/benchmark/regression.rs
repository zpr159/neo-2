use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::types::RiskLevel;

/// Outcome of a single regression check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionDetectionResult {
    /// Name of the metric that was checked.
    pub metric: String,
    /// Mean value of the historical baseline.
    pub baseline_mean: f64,
    /// Mean value of the current sample.
    pub current_mean: f64,
    /// Signed percentage change (negative = regression for most metrics).
    pub change_percent: f64,
    /// `true` when the change exceeds the configured threshold.
    pub is_regression: bool,
    /// Risk classification of the detected regression.
    pub severity: RiskLevel,
}

/// Compares current metric values against stored baselines and flags
/// regressions.
#[derive(Debug)]
pub struct RegressionDetector {
    /// Historical baseline values keyed by metric name.
    baselines: DashMap<String, Vec<f64>>,
    /// Maximum allowed percentage deviation before a regression is flagged.
    threshold: f64,
}

impl RegressionDetector {
    /// Create a new detector with the given regression threshold (percentage).
    pub fn new(threshold: f64) -> Self {
        Self {
            baselines: DashMap::new(),
            threshold,
        }
    }

    /// Set or replace the baseline for a named metric.
    pub fn set_baseline(&self, metric: impl Into<String>, values: Vec<f64>) {
        self.baselines.insert(metric.into(), values);
    }

    /// Compare each baseline against `current_values` (keyed by the same
    /// metric name) and return detection results for every metric that has
    /// a baseline.
    pub fn detect_regressions(
        &self,
        current_values: &[(String, Vec<f64>)],
    ) -> Vec<RegressionDetectionResult> {
        let mut results: Vec<RegressionDetectionResult> = Vec::new();

        for (metric_name, current) in current_values {
            if let Some(baseline) = self.baselines.get(metric_name) {
                let baseline_mean = mean(&baseline);
                let current_mean = mean(current);

                let change_percent = if baseline_mean.abs() < f64::EPSILON {
                    0.0
                } else {
                    ((current_mean - baseline_mean) / baseline_mean) * 100.0
                };

                let is_regression = change_percent < -self.threshold;
                let severity = classify_severity(change_percent, self.threshold);

                results.push(RegressionDetectionResult {
                    metric: metric_name.clone(),
                    baseline_mean,
                    current_mean,
                    change_percent,
                    is_regression,
                    severity,
                });
            }
        }

        results
    }

    /// Return the current regression threshold.
    pub fn get_threshold(&self) -> f64 {
        self.threshold
    }

    /// Update the regression threshold.
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold;
    }
}

/// Compute the arithmetic mean of a slice, returning 0.0 for empty slices.
fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: f64 = values.iter().sum();
    sum / values.len() as f64
}

/// Classify the severity of a regression based on percentage change and
/// threshold.
fn classify_severity(change_percent: f64, threshold: f64) -> RiskLevel {
    let magnitude = change_percent.abs();
    if magnitude <= threshold {
        RiskLevel::None
    } else if magnitude <= threshold * 2.0 {
        RiskLevel::Low
    } else if magnitude <= threshold * 4.0 {
        RiskLevel::Medium
    } else if magnitude <= threshold * 8.0 {
        RiskLevel::High
    } else {
        RiskLevel::Critical
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_regression_within_threshold() {
        let mut det = RegressionDetector::new(10.0);
        det.set_baseline("latency".to_string(), vec![100.0, 110.0, 105.0]);
        let results = det.detect_regressions(&[("latency".to_string(), vec![102.0, 108.0, 104.0])]);
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_regression);
        assert_eq!(results[0].severity, RiskLevel::None);
    }

    #[test]
    fn regression_detected() {
        let mut det = RegressionDetector::new(5.0);
        det.set_baseline("throughput".to_string(), vec![1000.0, 1050.0, 1020.0]);
        let results =
            det.detect_regressions(&[("throughput".to_string(), vec![800.0, 820.0, 810.0])]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_regression);
        assert!(results[0].severity >= RiskLevel::Medium);
    }

    #[test]
    fn mean_calculation() {
        assert!((mean(&[10.0, 20.0, 30.0]) - 20.0).abs() < f64::EPSILON);
        assert!((mean(&[]) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn threshold_get_set() {
        let mut det = RegressionDetector::new(10.0);
        assert!((det.get_threshold() - 10.0).abs() < f64::EPSILON);
        det.set_threshold(20.0);
        assert!((det.get_threshold() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unknown_metric_ignored() {
        let det = RegressionDetector::new(5.0);
        let results = det.detect_regressions(&[("unknown".to_string(), vec![1.0, 2.0])]);
        assert!(results.is_empty());
    }
}
