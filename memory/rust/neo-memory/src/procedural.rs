use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{MemoryError, MemoryResult};
use crate::types::{MemoryEntry, MemoryId};

/// A single step within a procedure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    /// Ordinal position of this step.
    pub step_number: u32,
    /// The action to perform.
    pub action: String,
    /// Named parameters for this step.
    pub parameters: HashMap<String, serde_json::Value>,
    /// What outcome is expected after this step.
    pub expected_outcome: Option<String>,
    /// Optional preconditions that must be met before this step.
    pub preconditions: Vec<String>,
    /// Optional postconditions guaranteed after this step.
    pub postconditions: Vec<String>,
    /// Estimated duration in milliseconds.
    pub estimated_duration_ms: Option<u64>,
}

impl ProcedureStep {
    /// Create a new procedure step.
    pub fn new(step_number: u32, action: impl Into<String>) -> Self {
        Self {
            step_number,
            action: action.into(),
            parameters: HashMap::new(),
            expected_outcome: None,
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            estimated_duration_ms: None,
        }
    }

    /// Set parameters.
    #[must_use]
    pub fn with_parameter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.parameters.insert(key.into(), value);
        self
    }

    /// Set expected outcome.
    #[must_use]
    pub fn with_outcome(mut self, outcome: impl Into<String>) -> Self {
        self.expected_outcome = Some(outcome.into());
        self
    }

    /// Set estimated duration.
    #[must_use]
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.estimated_duration_ms = Some(ms);
        self
    }
}

/// A reusable procedure consisting of ordered steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// Description of what this procedure accomplishes.
    pub description: String,
    /// Ordered steps to execute.
    pub steps: Vec<ProcedureStep>,
    /// Historical success rate (0.0 - 1.0).
    pub success_rate: f32,
    /// Total number of times this procedure has been executed.
    pub execution_count: u64,
    /// Average execution duration in milliseconds.
    pub avg_duration_ms: f64,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Prerequisites (what must be true before running this procedure).
    pub prerequisites: Vec<String>,
    /// When this procedure was created.
    pub created_at: DateTime<Utc>,
    /// When this procedure was last modified.
    pub last_modified: DateTime<Utc>,
    /// Version number.
    pub version: u64,
    /// Associated memory entry id.
    pub memory_id: Option<MemoryId>,
}

impl Procedure {
    /// Create a new procedure.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            steps: Vec::new(),
            success_rate: 1.0,
            execution_count: 0,
            avg_duration_ms: 0.0,
            tags: Vec::new(),
            prerequisites: Vec::new(),
            created_at: now,
            last_modified: now,
            version: 1,
            memory_id: None,
        }
    }

    /// Add a step.
    pub fn add_step(&mut self, step: ProcedureStep) {
        self.steps.push(step);
        self.steps
            .sort_by_key(|s| s.step_number);
        self.last_modified = Utc::now();
        self.version += 1;
    }

    /// Total estimated duration in milliseconds.
    #[must_use]
    pub fn estimated_duration_ms(&self) -> u64 {
        self.steps
            .iter()
            .filter_map(|s| s.estimated_duration_ms)
            .sum()
    }
}

/// Record of a single procedure execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Unique identifier.
    pub id: Uuid,
    /// The procedure that was executed.
    pub procedure_id: Uuid,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Whether execution was successful.
    pub success: bool,
    /// Actual duration in milliseconds.
    pub duration_ms: f64,
    /// Output or result of the execution.
    pub output: Option<serde_json::Value>,
    /// Error message if execution failed.
    pub error: Option<String>,
    /// Parameters used for this execution.
    pub parameters: HashMap<String, serde_json::Value>,
    /// Step-by-step results.
    pub step_results: Vec<StepResult>,
}

/// Result of executing a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step number.
    pub step_number: u32,
    /// Whether the step succeeded.
    pub success: bool,
    /// Step output.
    pub output: Option<serde_json::Value>,
    /// Duration in milliseconds.
    pub duration_ms: f64,
    /// Error if failed.
    pub error: Option<String>,
}

/// Optimization record for a procedure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecord {
    /// Unique identifier.
    pub id: Uuid,
    /// The procedure that was optimized.
    pub procedure_id: Uuid,
    /// When the optimization was applied.
    pub applied_at: DateTime<Utc>,
    /// Description of the optimization.
    pub description: String,
    /// Performance improvement ratio (> 1.0 = improvement).
    pub improvement_ratio: f64,
    /// The optimization type.
    pub optimization_type: OptimizationType,
}

/// Type of optimization applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OptimizationType {
    /// Removed redundant steps.
    StepReduction,
    /// Parallelized independent steps.
    Parallelization,
    /// Reduced parameter complexity.
    ParameterSimplification,
    /// Improved step ordering.
    Reordering,
    /// Merged similar steps.
    StepMerging,
    /// Cached intermediate results.
    Caching,
}

impl std::fmt::Display for OptimizationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StepReduction => write!(f, "step_reduction"),
            Self::Parallelization => write!(f, "parallelization"),
            Self::ParameterSimplification => write!(f, "parameter_simplification"),
            Self::Reordering => write!(f, "reordering"),
            Self::StepMerging => write!(f, "step_merging"),
            Self::Caching => write!(f, "caching"),
        }
    }
}

/// Configuration for procedural memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralMemoryConfig {
    /// Maximum number of procedures.
    pub max_procedures: usize,
    /// Maximum execution history per procedure.
    pub max_history_per_procedure: usize,
    /// Whether to persist to sled DB.
    pub persistence_enabled: bool,
    /// Path for sled DB persistence.
    pub persistence_path: Option<String>,
    /// Minimum executions before optimization analysis.
    pub min_executions_for_optimization: u64,
}

impl Default for ProceduralMemoryConfig {
    fn default() -> Self {
        Self {
            max_procedures: 10_000,
            max_history_per_procedure: 100,
            persistence_enabled: false,
            persistence_path: None,
            min_executions_for_optimization: 5,
        }
    }
}

/// Procedural memory store for skills, workflows, and procedures.
#[derive(Debug)]
pub struct ProceduralMemory {
    procedures: DashMap<Uuid, Procedure>,
    execution_history: DashMap<Uuid, Vec<ExecutionRecord>>,
    optimization_history: DashMap<Uuid, Vec<OptimizationRecord>>,
    name_index: DashMap<String, Uuid>,
    entries: DashMap<MemoryId, MemoryEntry>,
    db: Option<sled::Db>,
    config: ProceduralMemoryConfig,
}

impl ProceduralMemory {
    /// Create a new procedural memory store.
    pub fn new(config: ProceduralMemoryConfig) -> MemoryResult<Self> {
        let db = if config.persistence_enabled {
            let path = config
                .persistence_path
                .as_deref()
                .unwrap_or("/tmp/neo-procedural");
            Some(
                sled::open(path)
                    .map_err(|e| MemoryError::PersistenceError(e.to_string()))?,
            )
        } else {
            None
        };
        Ok(Self {
            procedures: DashMap::new(),
            execution_history: DashMap::new(),
            optimization_history: DashMap::new(),
            name_index: DashMap::new(),
            entries: DashMap::new(),
            db,
            config,
        })
    }

    /// Store a procedure and return its id.
    pub fn store_procedure(&self, proc: Procedure) -> MemoryResult<Uuid> {
        let count = self.procedures.len();
        if count >= self.config.max_procedures {
            return Err(MemoryError::CapacityExceeded(
                "Procedural memory capacity reached".to_string(),
            ));
        }

        let id = proc.id;

        // Persist to sled DB.
        if let Some(ref db) = self.db {
            let key = format!("proc:{id}");
            let value = serde_json::to_vec(&proc)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        self.name_index
            .entry(proc.name.clone())
            .or_insert(id);
        self.procedures.insert(id, proc);

        Ok(id)
    }

    /// Retrieve a procedure by id.
    pub fn get_procedure(&self, id: Uuid) -> Option<Procedure> {
        self.procedures.get(&id).map(|p| p.value().clone())
    }

    /// Retrieve a procedure by name.
    pub fn get_procedure_by_name(&self, name: &str) -> Option<Procedure> {
        let id = *self.name_index.get(name)?;
        self.procedures.get(&id).map(|p| p.value().clone())
    }

    /// Update a procedure.
    pub fn update_procedure(&self, id: Uuid, updater: impl FnOnce(&mut Procedure)) -> MemoryResult<()> {
        let mut proc = self
            .procedures
            .get_mut(&id)
            .ok_or_else(|| MemoryError::NotFound(format!("Procedure {id} not found")))?;

        updater(&mut proc);
        proc.last_modified = Utc::now();
        proc.version += 1;

        // Persist update.
        if let Some(ref db) = self.db {
            let key = format!("proc:{id}");
            let value = serde_json::to_vec(proc.value())
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        Ok(())
    }

    /// Remove a procedure.
    pub fn remove_procedure(&self, id: Uuid) -> MemoryResult<bool> {
        if let Some((_, proc)) = self.procedures.remove(&id) {
            self.name_index.remove(&proc.name);
            self.execution_history.remove(&id);
            self.optimization_history.remove(&id);

            if let Some(ref db) = self.db {
                let key = format!("proc:{id}");
                let _ = db.remove(key.as_bytes());
            }

            return Ok(true);
        }
        Ok(false)
    }

    /// Search procedures by name (case-insensitive substring match).
    #[must_use]
    pub fn search_by_name(&self, name: &str) -> Vec<Procedure> {
        let lower = name.to_lowercase();
        self.procedures
            .iter()
            .filter(|p| p.value().name.to_lowercase().contains(&lower))
            .map(|p| p.value().clone())
            .collect()
    }

    /// Search procedures by tag.
    #[must_use]
    pub fn search_by_tag(&self, tag: &str) -> Vec<Procedure> {
        let lower = tag.to_lowercase();
        self.procedures
            .iter()
            .filter(|p| {
                p.value()
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == lower)
            })
            .map(|p| p.value().clone())
            .collect()
    }

    /// Find the best procedure for a given action keyword (highest success rate).
    #[must_use]
    pub fn best_procedure_for(&self, action: &str) -> Option<Procedure> {
        let lower = action.to_lowercase();
        self.procedures
            .iter()
            .filter(|p| {
                p.value()
                    .steps
                    .iter()
                    .any(|s| s.action.to_lowercase().contains(&lower))
            })
            .max_by(|a, b| {
                a.value()
                    .success_rate
                    .partial_cmp(&b.value().success_rate)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.value().clone())
    }

    /// Record an execution for a procedure.
    pub fn record_execution(&self, record: ExecutionRecord) -> MemoryResult<()> {
        let proc_id = record.procedure_id;

        // Update procedure stats.
        if let Some(mut proc) = self.procedures.get_mut(&proc_id) {
            proc.execution_count += 1;
            let count = proc.execution_count as f64;
            proc.avg_duration_ms =
                (proc.avg_duration_ms * (count - 1.0) + record.duration_ms) / count;
            let successes =
                proc.success_rate as f64 * (count - 1.0) + if record.success { 1.0 } else { 0.0 };
            proc.success_rate = (successes / count) as f32;
            proc.last_modified = Utc::now();
        }

        // Persist record.
        if let Some(ref db) = self.db {
            let key = format!("exec:{}", record.id);
            let value = serde_json::to_vec(&record)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        // Add to history, evicting oldest if needed.
        let mut history = self.execution_history.entry(proc_id).or_default();
        if history.len() >= self.config.max_history_per_procedure {
            history.remove(0);
        }
        history.push(record);

        Ok(())
    }

    /// Get execution history for a procedure.
    #[must_use]
    pub fn execution_history(&self, procedure_id: Uuid) -> Vec<ExecutionRecord> {
        self.execution_history
            .get(&procedure_id)
            .map(|h| h.value().clone())
            .unwrap_or_default()
    }

    /// Record an optimization.
    pub fn record_optimization(&self, record: OptimizationRecord) -> MemoryResult<()> {
        // Persist record.
        if let Some(ref db) = self.db {
            let key = format!("opt:{}", record.id);
            let value = serde_json::to_vec(&record)
                .map_err(|e| MemoryError::SerializationError(e.to_string()))?;
            db.insert(key.as_bytes(), value)
                .map_err(|e| MemoryError::PersistenceError(e.to_string()))?;
        }

        self.optimization_history
            .entry(record.procedure_id)
            .or_default()
            .push(record);

        Ok(())
    }

    /// Get optimization history for a procedure.
    #[must_use]
    pub fn optimization_history(&self, procedure_id: Uuid) -> Vec<OptimizationRecord> {
        self.optimization_history
            .get(&procedure_id)
            .map(|h| h.value().clone())
            .unwrap_or_default()
    }

    /// Analyze a procedure for potential optimizations.
    #[must_use]
    pub fn analyze_optimization_opportunities(
        &self,
        procedure_id: Uuid,
    ) -> Vec<String> {
        let proc = match self.procedures.get(&procedure_id) {
            Some(p) => p.value().clone(),
            None => return Vec::new(),
        };

        let mut opportunities = Vec::new();

        if proc.execution_count < self.config.min_executions_for_optimization {
            return opportunities;
        }

        let history = self.execution_history(procedure_id);

        // Check for frequently failing steps.
        let mut step_failures: HashMap<u32, u64> = HashMap::new();
        let mut step_counts: HashMap<u32, u64> = HashMap::new();

        for record in &history {
            for step_result in &record.step_results {
                *step_counts
                    .entry(step_result.step_number)
                    .or_insert(0) += 1;
                if !step_result.success {
                    *step_failures
                        .entry(step_result.step_number)
                        .or_insert(0) += 1;
                }
            }
        }

        for (step_num, failures) in &step_failures {
            if let Some(count) = step_counts.get(step_num) {
                let fail_rate = *failures as f64 / *count as f64;
                if fail_rate > 0.3 {
                    opportunities.push(format!(
                        "Step {step_num} has high failure rate ({:.0}%)",
                        fail_rate * 100.0
                    ));
                }
            }
        }

        // Check for slow steps.
        for record in &history {
            for step_result in &record.step_results {
                if let Some(step) = proc.steps.iter().find(|s| s.step_number == step_result.step_number) {
                    if let Some(expected) = step.estimated_duration_ms {
                        if step_result.duration_ms > expected as f64 * 2.0 {
                            opportunities.push(format!(
                                "Step {} is running slower than expected ({:.0}ms vs {expected}ms)",
                                step_result.step_number, step_result.duration_ms
                            ));
                        }
                    }
                }
            }
        }

        // Check for redundant steps.
        let unique_actions: Vec<String> = proc
            .steps
            .iter()
            .map(|s| s.action.to_lowercase())
            .collect();
        let mut seen_actions = std::collections::HashSet::new();
        for action in &unique_actions {
            if !seen_actions.insert(action.clone()) {
                opportunities.push(format!(
                    "Duplicate action found: '{action}'"
                ));
            }
        }

        opportunities
    }

    /// Get all procedures sorted by success rate (descending).
    #[must_use]
    pub fn top_procedures(&self, count: usize) -> Vec<Procedure> {
        let mut all: Vec<Procedure> = self.procedures.iter().map(|p| p.value().clone()).collect();
        all.sort_by(|a, b| {
            b.success_rate
                .partial_cmp(&a.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        all.into_iter().take(count).collect()
    }

    /// Get the total number of procedures.
    #[must_use]
    pub fn count(&self) -> usize {
        self.procedures.len()
    }

    /// Store a memory entry alongside a procedure.
    pub fn store_with_entry(
        &self,
        entry: MemoryEntry,
        mut proc: Procedure,
    ) -> MemoryResult<MemoryId> {
        let memory_id = entry.id;
        self.entries.insert(memory_id, entry);
        proc.memory_id = Some(memory_id);
        self.store_procedure(proc)?;
        Ok(memory_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_and_retrieve_procedure() {
        let mem = ProceduralMemory::new(ProceduralMemoryConfig::default()).unwrap();
        let mut proc = Procedure::new("Test Procedure", "A test");
        proc.add_step(ProcedureStep::new(1, "step_one"));
        proc.add_step(ProcedureStep::new(2, "step_two"));

        let id = mem.store_procedure(proc).unwrap();
        let retrieved = mem.get_procedure(id).unwrap();
        assert_eq!(retrieved.name, "Test Procedure");
        assert_eq!(retrieved.steps.len(), 2);
    }

    #[test]
    fn execution_recording() {
        let mem = ProceduralMemory::new(ProceduralMemoryConfig::default()).unwrap();
        let proc = Procedure::new("Exec Test", "Test execution");
        let proc_id = proc.id;
        mem.store_procedure(proc).unwrap();

        let record = ExecutionRecord {
            id: Uuid::new_v4(),
            procedure_id: proc_id,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            success: true,
            duration_ms: 100.0,
            output: Some(serde_json::json!("done")),
            error: None,
            parameters: HashMap::new(),
            step_results: Vec::new(),
        };

        mem.record_execution(record).unwrap();

        let proc = mem.get_procedure(proc_id).unwrap();
        assert_eq!(proc.execution_count, 1);
        assert!((proc.success_rate - 1.0).abs() < 0.01);

        let history = mem.execution_history(proc_id);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn search_by_tag() {
        let mem = ProceduralMemory::new(ProceduralMemoryConfig::default()).unwrap();
        let mut proc = Procedure::new("Tagged Proc", "Has tags");
        proc.tags = vec!["automation".to_string(), "deploy".to_string()];
        mem.store_procedure(proc).unwrap();

        let results = mem.search_by_tag("automation");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn optimization_analysis() {
        let mem = ProceduralMemory::new(ProceduralMemoryConfig::default()).unwrap();
        let mut proc = Procedure::new("Analysis Proc", "Test");
        proc.add_step(ProcedureStep::new(1, "deploy"));
        proc.add_step(ProcedureStep::new(2, "deploy")); // Duplicate
        let proc_id = proc.id;
        mem.store_procedure(proc).unwrap();

        // Record enough executions.
        for _ in 0..10 {
            let record = ExecutionRecord {
                id: Uuid::new_v4(),
                procedure_id: proc_id,
                started_at: Utc::now(),
                completed_at: Some(Utc::now()),
                success: true,
                duration_ms: 100.0,
                output: None,
                error: None,
                parameters: HashMap::new(),
                step_results: Vec::new(),
            };
            mem.record_execution(record).unwrap();
        }

        let opportunities = mem.analyze_optimization_opportunities(proc_id);
        assert!(!opportunities.is_empty());
        assert!(opportunities.iter().any(|o| o.contains("Duplicate")));
    }

    #[test]
    fn capacity_limit() {
        let config = ProceduralMemoryConfig {
            max_procedures: 2,
            ..ProceduralMemoryConfig::default()
        };
        let mem = ProceduralMemory::new(config).unwrap();
        mem.store_procedure(Procedure::new("A", "desc")).unwrap();
        mem.store_procedure(Procedure::new("B", "desc")).unwrap();
        let result = mem.store_procedure(Procedure::new("C", "desc"));
        assert!(result.is_err());
    }
}
