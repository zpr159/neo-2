//! Identity types for the planning system.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

macro_rules! define_planning_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub uuid::Uuid);

        impl $name {
            /// Create a new identifier.
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }

            /// Get the inner UUID as a string.
            pub fn as_str(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(uuid::Uuid::parse_str(s)?))
            }
        }

        impl From<uuid::Uuid> for $name {
            fn from(id: uuid::Uuid) -> Self {
                Self(id)
            }
        }

        impl From<$name> for uuid::Uuid {
            fn from(id: $name) -> uuid::Uuid {
                id.0
            }
        }
    };
}

define_planning_id!(
    /// Unique identifier for a planning goal.
    PlanningGoalId
);

define_planning_id!(
    /// Unique identifier for a plan.
    PlanId
);

define_planning_id!(
    /// Unique identifier for a planning strategy.
    StrategyId
);

define_planning_id!(
    /// Unique identifier for a planning algorithm.
    AlgorithmId
);

define_planning_id!(
    /// Unique identifier for a planning node in the plan graph.
    PlanningNodeId
);

define_planning_id!(
    /// Unique identifier for a planning edge in the plan graph.
    PlanningEdgeId
);

define_planning_id!(
    /// Unique identifier for a planning session.
    PlanningSessionId
);

define_planning_id!(
    /// Unique identifier for a planning checkpoint.
    PlanCheckpointId
);

define_planning_id!(
    /// Unique identifier for a resource allocation.
    ResourceAllocationId
);

define_planning_id!(
    /// Unique identifier for a risk assessment.
    RiskAssessmentId
);

define_planning_id!(
    /// Unique identifier for a cost estimate.
    CostEstimateId
);

define_planning_id!(
    /// Unique identifier for an optimization pass.
    OptimizationPassId
);

define_planning_id!(
    /// Unique identifier for a replanning event.
    ReplanEventId
);

define_planning_id!(
    /// Unique identifier for an agent allocation in multi-agent planning.
    AgentAllocationId
);

define_planning_id!(
    /// Unique identifier for a workflow generated from a plan.
    GeneratedWorkflowId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_id_new_is_unique() {
        let a = PlanId::new();
        let b = PlanId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn plan_id_display() {
        let id = PlanId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36);
    }

    #[test]
    fn plan_id_roundtrip() {
        let id = PlanId::new();
        let s = id.to_string();
        let parsed: PlanId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn plan_id_from_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let id: PlanId = uuid.into();
        let back: uuid::Uuid = id.into();
        assert_eq!(uuid, back);
    }

    #[test]
    fn goal_id_default() {
        let id = PlanningGoalId::default();
        assert!(!id.0.is_nil());
    }

    #[test]
    fn strategy_id_new() {
        let id = StrategyId::new();
        assert!(!id.0.is_nil());
    }
}
