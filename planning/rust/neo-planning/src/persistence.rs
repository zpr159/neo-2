//! Persistence layer for planning data.

use std::collections::HashMap;
use std::path::Path;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::{PlanningError, PlanningErrorCode, PlanningResult};
use crate::goal::Goal;
use crate::id::{PlanId, PlanningGoalId, StrategyId};
use crate::plan::{Plan, PlanCheckpoint};
use crate::strategy::Strategy;

/// Serialization format for persistence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SerializeFormat {
    Json,
    Bincode,
    Toml,
}

impl Default for SerializeFormat {
    fn default() -> Self {
        Self::Json
    }
}

/// Configuration for the persistence layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Root directory for storing planning data.
    pub storage_path: String,
    /// Whether to automatically save after mutations.
    pub auto_save: bool,
    /// Maximum number of historical records to retain.
    pub max_history: usize,
    /// Serialization format to use.
    pub serialize_format: SerializeFormat,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            storage_path: "/tmp/neo-planning".to_string(),
            auto_save: false,
            max_history: 1000,
            serialize_format: SerializeFormat::Json,
        }
    }
}

/// In-memory store for plans with optional history tracking.
pub struct PlanStore {
    plans: RwLock<HashMap<PlanId, Plan>>,
    history: RwLock<Vec<Plan>>,
    config: PersistenceConfig,
}

impl PlanStore {
    /// Create a new plan store.
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            plans: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            config,
        }
    }

    /// Save a plan, replacing any existing plan with the same id.
    pub fn save(&self, plan: Plan) -> PlanningResult<()> {
        let id = plan.id;
        self.plans.write().insert(id, plan.clone());
        let mut history = self.history.write();
        history.push(plan);
        if history.len() > self.config.max_history {
            let excess = history.len() - self.config.max_history;
            history.drain(0..excess);
        }
        Ok(())
    }

    /// Load a plan by id.
    pub fn load(&self, id: PlanId) -> PlanningResult<Plan> {
        self.plans
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| PlanningError::plan_not_found(&id.as_str()))
    }

    /// List all stored plans.
    pub fn list(&self) -> Vec<Plan> {
        self.plans.read().values().cloned().collect()
    }

    /// Delete a plan by id.
    pub fn delete(&self, id: PlanId) -> PlanningResult<()> {
        self.plans
            .write()
            .remove(&id)
            .ok_or_else(|| PlanningError::plan_not_found(&id.as_str()))?;
        Ok(())
    }

    /// Return the history of saved plans.
    pub fn history(&self) -> Vec<Plan> {
        self.history.read().clone()
    }

    /// Return the number of stored plans.
    pub fn len(&self) -> usize {
        self.plans.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.plans.read().is_empty()
    }
}

/// In-memory store for goals.
pub struct GoalStore {
    goals: RwLock<HashMap<PlanningGoalId, Goal>>,
    #[allow(dead_code)]
    config: PersistenceConfig,
}

impl GoalStore {
    /// Create a new goal store.
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            goals: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Save a goal.
    pub fn save(&self, goal: Goal) {
        let id = goal.id;
        self.goals.write().insert(id, goal);
    }

    /// Load a goal by id.
    pub fn load(&self, id: PlanningGoalId) -> PlanningResult<Goal> {
        self.goals
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| PlanningError::goal_not_found(&id.as_str()))
    }

    /// List all stored goals.
    pub fn list(&self) -> Vec<Goal> {
        self.goals.read().values().cloned().collect()
    }

    /// Delete a goal by id.
    pub fn delete(&self, id: PlanningGoalId) {
        self.goals.write().remove(&id);
    }

    /// Return the number of stored goals.
    pub fn len(&self) -> usize {
        self.goals.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.goals.read().is_empty()
    }
}

/// In-memory store for strategies.
pub struct StrategyStore {
    strategies: RwLock<HashMap<StrategyId, Strategy>>,
    #[allow(dead_code)]
    config: PersistenceConfig,
}

impl StrategyStore {
    /// Create a new strategy store.
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            strategies: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Save a strategy.
    pub fn save(&self, strategy: Strategy) {
        let id = strategy.id;
        self.strategies.write().insert(id, strategy);
    }

    /// Load a strategy by id.
    pub fn load(&self, id: StrategyId) -> PlanningResult<Strategy> {
        self.strategies
            .read()
            .get(&id)
            .cloned()
            .ok_or_else(|| PlanningError::strategy_not_found(&id.as_str()))
    }

    /// List all stored strategies.
    pub fn list(&self) -> Vec<Strategy> {
        self.strategies.read().values().cloned().collect()
    }

    /// Delete a strategy by id.
    pub fn delete(&self, id: StrategyId) {
        self.strategies.write().remove(&id);
    }

    /// Return the number of stored strategies.
    pub fn len(&self) -> usize {
        self.strategies.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.strategies.read().is_empty()
    }
}

/// In-memory store for plan checkpoints.
pub struct CheckpointStore {
    checkpoints: RwLock<Vec<PlanCheckpoint>>,
    #[allow(dead_code)]
    config: PersistenceConfig,
}

impl CheckpointStore {
    /// Create a new checkpoint store.
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            checkpoints: RwLock::new(Vec::new()),
            config,
        }
    }

    /// Save a checkpoint.
    pub fn save(&self, checkpoint: PlanCheckpoint) {
        self.checkpoints.write().push(checkpoint);
    }

    /// Load a checkpoint by id.
    pub fn load(&self, id: crate::id::PlanCheckpointId) -> PlanningResult<PlanCheckpoint> {
        self.checkpoints
            .read()
            .iter()
            .find(|cp| cp.id == id)
            .cloned()
            .ok_or_else(|| {
                PlanningError::new(
                    PlanningErrorCode::PlanNotFound,
                    format!("checkpoint '{}' not found", id),
                )
            })
    }

    /// List all checkpoints for a specific plan.
    pub fn list_for_plan(&self, plan_id: PlanId) -> Vec<PlanCheckpoint> {
        self.checkpoints
            .read()
            .iter()
            .filter(|cp| cp.plan_id == plan_id)
            .cloned()
            .collect()
    }

    /// Return the total number of stored checkpoints.
    pub fn len(&self) -> usize {
        self.checkpoints.read().len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.checkpoints.read().is_empty()
    }
}

/// Unified persistence repository wrapping all individual stores.
pub struct PlanningRepository {
    plan_store: PlanStore,
    goal_store: GoalStore,
    strategy_store: StrategyStore,
    checkpoint_store: CheckpointStore,
}

impl PlanningRepository {
    /// Create a new repository with the given configuration.
    pub fn new(config: PersistenceConfig) -> Self {
        Self {
            plan_store: PlanStore::new(config.clone()),
            goal_store: GoalStore::new(config.clone()),
            strategy_store: StrategyStore::new(config.clone()),
            checkpoint_store: CheckpointStore::new(config),
        }
    }

    /// Save a plan.
    pub fn save_plan(&self, plan: Plan) -> PlanningResult<()> {
        self.plan_store.save(plan)
    }

    /// Load a plan by id.
    pub fn load_plan(&self, id: PlanId) -> PlanningResult<Plan> {
        self.plan_store.load(id)
    }

    /// List all plans.
    pub fn list_plans(&self) -> Vec<Plan> {
        self.plan_store.list()
    }

    /// Save a goal.
    pub fn save_goal(&self, goal: Goal) {
        self.goal_store.save(goal);
    }

    /// Load a goal by id.
    pub fn load_goal(&self, id: PlanningGoalId) -> PlanningResult<Goal> {
        self.goal_store.load(id)
    }

    /// List all goals.
    pub fn list_goals(&self) -> Vec<Goal> {
        self.goal_store.list()
    }

    /// Save a strategy.
    pub fn save_strategy(&self, strategy: Strategy) {
        self.strategy_store.save(strategy);
    }

    /// Load a strategy by id.
    pub fn load_strategy(&self, id: StrategyId) -> PlanningResult<Strategy> {
        self.strategy_store.load(id)
    }

    /// Save a checkpoint.
    pub fn save_checkpoint(&self, checkpoint: PlanCheckpoint) {
        self.checkpoint_store.save(checkpoint);
    }

    /// Load a checkpoint by id.
    pub fn load_checkpoint(
        &self,
        id: crate::id::PlanCheckpointId,
    ) -> PlanningResult<PlanCheckpoint> {
        self.checkpoint_store.load(id)
    }

    /// List checkpoints for a plan.
    pub fn list_checkpoints_for_plan(&self, plan_id: PlanId) -> Vec<PlanCheckpoint> {
        self.checkpoint_store.list_for_plan(plan_id)
    }

    /// Return the underlying plan store.
    pub fn plans(&self) -> &PlanStore {
        &self.plan_store
    }

    /// Return the underlying goal store.
    pub fn goals(&self) -> &GoalStore {
        &self.goal_store
    }

    /// Return the underlying strategy store.
    pub fn strategies(&self) -> &StrategyStore {
        &self.strategy_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::GoalPriority;
    use crate::plan::{PlanDefinition, PlanState, PlanTask};
    use crate::strategy::Strategy;
    use crate::types::AlgorithmType;
    use crate::types::{ExecutionBudget, PlanMetadata, PlanTaskType};

    fn make_test_plan() -> Plan {
        let task = PlanTask::new("task1", PlanTaskType::Atomic);
        let goal_id = crate::id::PlanningGoalId::new();
        let def = PlanDefinition::new(goal_id, AlgorithmType::AStar).with_task(task);
        Plan::new(def, PlanMetadata::new("test-plan"))
    }

    fn make_test_goal() -> Goal {
        Goal::new("test-goal", GoalPriority::Normal)
    }

    fn make_test_strategy() -> Strategy {
        Strategy::new("test-strategy", AlgorithmType::AStar)
    }

    fn default_config() -> PersistenceConfig {
        PersistenceConfig::default()
    }

    // ---- PlanStore ----

    #[test]
    fn plan_store_save_and_load() {
        let store = PlanStore::new(default_config());
        let plan = make_test_plan();
        let id = plan.id;
        store.save(plan).unwrap();
        let loaded = store.load(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn plan_store_load_not_found() {
        let store = PlanStore::new(default_config());
        let result = store.load(PlanId::new());
        assert!(result.is_err());
    }

    #[test]
    fn plan_store_list() {
        let store = PlanStore::new(default_config());
        let p1 = make_test_plan();
        let p2 = make_test_plan();
        store.save(p1).unwrap();
        store.save(p2).unwrap();
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn plan_store_delete() {
        let store = PlanStore::new(default_config());
        let plan = make_test_plan();
        let id = plan.id;
        store.save(plan).unwrap();
        store.delete(id).unwrap();
        assert!(store.load(id).is_err());
    }

    #[test]
    fn plan_store_delete_not_found() {
        let store = PlanStore::new(default_config());
        assert!(store.delete(PlanId::new()).is_err());
    }

    #[test]
    fn plan_store_history() {
        let store = PlanStore::new(default_config());
        let plan = make_test_plan();
        store.save(plan).unwrap();
        let history = store.history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn plan_store_len_and_is_empty() {
        let store = PlanStore::new(default_config());
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        store.save(make_test_plan()).unwrap();
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn plan_store_history_max() {
        let config = PersistenceConfig {
            max_history: 2,
            ..default_config()
        };
        let store = PlanStore::new(config);
        store.save(make_test_plan()).unwrap();
        store.save(make_test_plan()).unwrap();
        store.save(make_test_plan()).unwrap();
        assert_eq!(store.history().len(), 2);
    }

    // ---- GoalStore ----

    #[test]
    fn goal_store_save_and_load() {
        let store = GoalStore::new(default_config());
        let goal = make_test_goal();
        let id = goal.id;
        store.save(goal);
        let loaded = store.load(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn goal_store_load_not_found() {
        let store = GoalStore::new(default_config());
        assert!(store.load(PlanningGoalId::new()).is_err());
    }

    #[test]
    fn goal_store_list() {
        let store = GoalStore::new(default_config());
        store.save(make_test_goal());
        store.save(make_test_goal());
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn goal_store_delete() {
        let store = GoalStore::new(default_config());
        let goal = make_test_goal();
        let id = goal.id;
        store.save(goal);
        store.delete(id);
        assert!(store.load(id).is_err());
    }

    #[test]
    fn goal_store_len_and_is_empty() {
        let store = GoalStore::new(default_config());
        assert!(store.is_empty());
        store.save(make_test_goal());
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    // ---- StrategyStore ----

    #[test]
    fn strategy_store_save_and_load() {
        let store = StrategyStore::new(default_config());
        let strategy = make_test_strategy();
        let id = strategy.id;
        store.save(strategy);
        let loaded = store.load(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn strategy_store_load_not_found() {
        let store = StrategyStore::new(default_config());
        assert!(store.load(StrategyId::new()).is_err());
    }

    #[test]
    fn strategy_store_list() {
        let store = StrategyStore::new(default_config());
        store.save(make_test_strategy());
        store.save(make_test_strategy());
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn strategy_store_delete() {
        let store = StrategyStore::new(default_config());
        let strategy = make_test_strategy();
        let id = strategy.id;
        store.save(strategy);
        store.delete(id);
        assert!(store.load(id).is_err());
    }

    #[test]
    fn strategy_store_len_and_is_empty() {
        let store = StrategyStore::new(default_config());
        assert!(store.is_empty());
        store.save(make_test_strategy());
        assert!(!store.is_empty());
    }

    // ---- CheckpointStore ----

    #[test]
    fn checkpoint_store_save_and_load() {
        let store = CheckpointStore::new(default_config());
        let plan_id = PlanId::new();
        let cp = PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        );
        let cp_id = cp.id;
        store.save(cp);
        let loaded = store.load(cp_id).unwrap();
        assert_eq!(loaded.plan_id, plan_id);
    }

    #[test]
    fn checkpoint_store_load_not_found() {
        let store = CheckpointStore::new(default_config());
        assert!(store.load(crate::id::PlanCheckpointId::new()).is_err());
    }

    #[test]
    fn checkpoint_store_list_for_plan() {
        let store = CheckpointStore::new(default_config());
        let plan_id = PlanId::new();
        store.save(PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        ));
        store.save(PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Executing,
        ));
        store.save(PlanCheckpoint::new(
            PlanId::new(),
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        ));
        assert_eq!(store.list_for_plan(plan_id).len(), 2);
    }

    #[test]
    fn checkpoint_store_len_and_is_empty() {
        let store = CheckpointStore::new(default_config());
        assert!(store.is_empty());
        store.save(PlanCheckpoint::new(
            PlanId::new(),
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        ));
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }

    // ---- PlanningRepository ----

    #[test]
    fn repository_save_and_load_plan() {
        let repo = PlanningRepository::new(default_config());
        let plan = make_test_plan();
        let id = plan.id;
        repo.save_plan(plan).unwrap();
        let loaded = repo.load_plan(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn repository_list_plans() {
        let repo = PlanningRepository::new(default_config());
        repo.save_plan(make_test_plan()).unwrap();
        repo.save_plan(make_test_plan()).unwrap();
        assert_eq!(repo.list_plans().len(), 2);
    }

    #[test]
    fn repository_save_and_load_goal() {
        let repo = PlanningRepository::new(default_config());
        let goal = make_test_goal();
        let id = goal.id;
        repo.save_goal(goal);
        let loaded = repo.load_goal(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn repository_list_goals() {
        let repo = PlanningRepository::new(default_config());
        repo.save_goal(make_test_goal());
        repo.save_goal(make_test_goal());
        assert_eq!(repo.list_goals().len(), 2);
    }

    #[test]
    fn repository_save_and_load_strategy() {
        let repo = PlanningRepository::new(default_config());
        let strategy = make_test_strategy();
        let id = strategy.id;
        repo.save_strategy(strategy);
        let loaded = repo.load_strategy(id).unwrap();
        assert_eq!(loaded.id, id);
    }

    #[test]
    fn repository_save_checkpoint() {
        let repo = PlanningRepository::new(default_config());
        let plan_id = PlanId::new();
        let cp = PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        );
        let cp_id = cp.id;
        repo.save_checkpoint(cp);
        let loaded = repo.load_checkpoint(cp_id).unwrap();
        assert_eq!(loaded.plan_id, plan_id);
    }

    #[test]
    fn repository_list_checkpoints_for_plan() {
        let repo = PlanningRepository::new(default_config());
        let plan_id = PlanId::new();
        repo.save_checkpoint(PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Created,
        ));
        repo.save_checkpoint(PlanCheckpoint::new(
            plan_id,
            crate::types::PlanVersion::initial(),
            PlanState::Executing,
        ));
        assert_eq!(repo.list_checkpoints_for_plan(plan_id).len(), 2);
    }

    #[test]
    fn repository_inner_stores() {
        let repo = PlanningRepository::new(default_config());
        repo.save_plan(make_test_plan()).unwrap();
        repo.save_goal(make_test_goal());
        repo.save_strategy(make_test_strategy());
        assert_eq!(repo.plans().len(), 1);
        assert_eq!(repo.goals().len(), 1);
        assert_eq!(repo.strategies().len(), 1);
    }

    // ---- Serialization ----

    #[test]
    fn persistence_config_default() {
        let config = PersistenceConfig::default();
        assert_eq!(config.storage_path, "/tmp/neo-planning");
        assert!(!config.auto_save);
        assert_eq!(config.max_history, 1000);
        assert_eq!(config.serialize_format, SerializeFormat::Json);
    }

    #[test]
    fn persistence_config_serialization() {
        let config = PersistenceConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let back: PersistenceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.storage_path, config.storage_path);
    }

    #[test]
    fn plan_store_overwrite() {
        let store = PlanStore::new(default_config());
        let mut plan = make_test_plan();
        let id = plan.id;
        store.save(plan.clone()).unwrap();
        plan.metadata.name = "updated".to_string();
        store.save(plan).unwrap();
        let loaded = store.load(id).unwrap();
        assert_eq!(loaded.metadata.name, "updated");
    }

    #[test]
    fn storage_path_can_be_custom() {
        let config = PersistenceConfig {
            storage_path: "/custom/path".to_string(),
            ..default_config()
        };
        assert_eq!(config.storage_path, "/custom/path");
        let _exists = Path::new(&config.storage_path);
    }
}
