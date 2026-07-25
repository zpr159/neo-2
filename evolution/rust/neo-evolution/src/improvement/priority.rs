use serde::{Deserialize, Serialize};
use std::fmt;

/// Priority level for an improvement candidate, ordered numerically
/// from most urgent (1) to least urgent (5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementPriority {
    /// Immediate action required — number 1.
    Critical,
    /// Should be addressed soon — number 2.
    High,
    /// Normal priority — number 3.
    Medium,
    /// Nice to have — number 4.
    Low,
    /// FYI only — number 5.
    Informational,
}

impl ImprovementPriority {
    /// Returns the numeric rank (1–5). Lower is higher priority.
    pub fn rank(self) -> u8 {
        match self {
            Self::Critical => 1,
            Self::High => 2,
            Self::Medium => 3,
            Self::Low => 4,
            Self::Informational => 5,
        }
    }

    /// Create an `ImprovementPriority` from a numeric rank.
    pub fn from_rank(rank: u8) -> Option<Self> {
        match rank {
            1 => Some(Self::Critical),
            2 => Some(Self::High),
            3 => Some(Self::Medium),
            4 => Some(Self::Low),
            5 => Some(Self::Informational),
            _ => None,
        }
    }
}

impl PartialOrd for ImprovementPriority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ImprovementPriority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank().cmp(&other.rank())
    }
}

impl fmt::Display for ImprovementPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Informational => "informational",
        };
        write!(f, "{label}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_ordering() {
        assert!(ImprovementPriority::Critical < ImprovementPriority::High);
        assert!(ImprovementPriority::High < ImprovementPriority::Medium);
        assert!(ImprovementPriority::Medium < ImprovementPriority::Low);
        assert!(ImprovementPriority::Low < ImprovementPriority::Informational);
    }

    #[test]
    fn roundtrip_from_rank() {
        for rank in 1..=5 {
            let p = ImprovementPriority::from_rank(rank).unwrap();
            assert_eq!(p.rank(), rank);
        }
        assert!(ImprovementPriority::from_rank(0).is_none());
        assert!(ImprovementPriority::from_rank(6).is_none());
    }

    #[test]
    fn display() {
        assert_eq!(ImprovementPriority::Critical.to_string(), "critical");
        assert_eq!(
            ImprovementPriority::Informational.to_string(),
            "informational"
        );
    }
}
