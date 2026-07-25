use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{EvolutionId, EvolutionStatus, ImprovementCategory, RiskLevel, SubsystemTarget};

use super::priority::ImprovementPriority;

/// A candidate improvement identified through self-analysis.
///
/// Contains all the metadata needed to describe, evaluate, and track
/// a potential improvement to the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementCandidate {
    /// Unique identifier for this candidate.
    pub id: EvolutionId,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the improvement.
    pub description: String,
    /// Category of the improvement.
    pub category: ImprovementCategory,
    /// Which subsystem this improvement targets.
    pub target: SubsystemTarget,
    /// Priority level.
    pub priority: ImprovementPriority,
    /// Estimated impact score (0.0 – 1.0).
    pub estimated_impact: f64,
    /// Risk level of implementing this improvement.
    pub risk_level: RiskLevel,
    /// High-level implementation plan.
    pub implementation_plan: String,
    /// IDs of candidates that must be completed first.
    pub prerequisites: Vec<EvolutionId>,
    /// When this candidate was created.
    pub created_at: DateTime<Utc>,
    /// Current lifecycle status.
    pub status: EvolutionStatus,
}

impl ImprovementCandidate {
    /// Create a new candidate with sensible defaults.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        category: ImprovementCategory,
        target: SubsystemTarget,
        priority: ImprovementPriority,
        estimated_impact: f64,
        risk_level: RiskLevel,
        implementation_plan: impl Into<String>,
    ) -> Self {
        Self {
            id: EvolutionId::new_v4(),
            title: title.into(),
            description: description.into(),
            category,
            target,
            priority,
            estimated_impact: estimated_impact.clamp(0.0, 1.0),
            risk_level,
            implementation_plan: implementation_plan.into(),
            prerequisites: Vec::new(),
            created_at: Utc::now(),
            status: EvolutionStatus::Pending,
        }
    }

    /// Add a prerequisite by its ID.
    pub fn with_prerequisite(mut self, prerequisite_id: EvolutionId) -> Self {
        self.prerequisites.push(prerequisite_id);
        self
    }

    /// Returns `true` if all prerequisites are satisfied given the completed set.
    pub fn prerequisites_met(&self, completed: &std::collections::HashSet<EvolutionId>) -> bool {
        self.prerequisites.iter().all(|id| completed.contains(id))
    }

    /// Transition the status.
    pub fn set_status(&mut self, status: EvolutionStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn create_candidate() {
        let c = ImprovementCandidate::new(
            "Optimize cache",
            "Improve LRU cache hit rate",
            ImprovementCategory::Performance,
            SubsystemTarget::Memory,
            ImprovementPriority::High,
            0.8,
            RiskLevel::Low,
            "Refill cache from hot keys",
        );
        assert_eq!(c.title, "Optimize cache");
        assert_eq!(c.estimated_impact, 0.8);
        assert_eq!(c.status, EvolutionStatus::Pending);
        assert!(c.prerequisites.is_empty());
    }

    #[test]
    fn impact_clamped() {
        let c = ImprovementCandidate::new(
            "X",
            "Y",
            ImprovementCategory::Reliability,
            SubsystemTarget::Core,
            ImprovementPriority::Medium,
            1.5,
            RiskLevel::None,
            "plan",
        );
        assert_eq!(c.estimated_impact, 1.0);

        let c2 = ImprovementCandidate::new(
            "X",
            "Y",
            ImprovementCategory::Reliability,
            SubsystemTarget::Core,
            ImprovementPriority::Medium,
            -0.5,
            RiskLevel::None,
            "plan",
        );
        assert_eq!(c2.estimated_impact, 0.0);
    }

    #[test]
    fn prerequisites_check() {
        let prereq = EvolutionId::new_v4();
        let c = ImprovementCandidate::new(
            "X",
            "Y",
            ImprovementCategory::Security,
            SubsystemTarget::Core,
            ImprovementPriority::Critical,
            0.9,
            RiskLevel::High,
            "plan",
        )
        .with_prerequisite(prereq);

        let mut completed = HashSet::new();
        assert!(!c.prerequisites_met(&completed));

        completed.insert(prereq);
        assert!(c.prerequisites_met(&completed));
    }
}
