//! Evolution strategies subsystem.
//!
//! Provides a pluggable strategy framework for selecting and applying
//! different improvement approaches (mutation, optimization, refinement)
//! based on the target subsystem and improvement category.

mod hybrid;
mod mutation;
mod optimization;
mod refinement;
mod selector;
mod strategy;

pub use hybrid::HybridStrategy;
pub use mutation::MutationStrategy;
pub use optimization::OptimizationStrategy;
pub use refinement::RefinementStrategy;
pub use selector::StrategySelector;
pub use strategy::{EvolutionStrategy, EvolutionStrategyTrait, StrategyInput, StrategyOutput};
