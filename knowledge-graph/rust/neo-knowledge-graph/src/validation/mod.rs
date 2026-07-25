pub mod source;
pub mod evidence;
pub mod contradiction;
pub mod resolution;

pub use source::SourceTracker;
pub use evidence::EvidenceTracker;
pub use contradiction::ContradictionDetector;
pub use resolution::{ConflictResolver, ResolutionStrategy, ResolutionResult};
