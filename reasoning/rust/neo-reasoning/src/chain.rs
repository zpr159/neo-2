use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::strategy::ReasoningStrategy;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepType {
    Premise,
    Observation,
    Inference,
    Hypothesis,
    Evaluation,
    Conclusion,
    Checkpoint,
    Branch,
    Merge,
}

impl std::fmt::Display for StepType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Premise => write!(f, "premise"),
            Self::Observation => write!(f, "observation"),
            Self::Inference => write!(f, "inference"),
            Self::Hypothesis => write!(f, "hypothesis"),
            Self::Evaluation => write!(f, "evaluation"),
            Self::Conclusion => write!(f, "conclusion"),
            Self::Checkpoint => write!(f, "checkpoint"),
            Self::Branch => write!(f, "branch"),
            Self::Merge => write!(f, "merge"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalStep {
    pub id: Uuid,
    pub step_type: StepType,
    pub content: String,
    pub confidence: f32,
    pub source: Option<String>,
    pub reasoning: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub parent_id: Option<Uuid>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InternalStep {
    pub fn new(step_type: StepType, content: String, confidence: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            step_type,
            content,
            confidence,
            source: None,
            reasoning: None,
            timestamp: Utc::now(),
            parent_id: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_reasoning(mut self, reasoning: String) -> Self {
        self.reasoning = Some(reasoning);
        self
    }

    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalChain {
    pub id: Uuid,
    pub strategy: ReasoningStrategy,
    pub steps: Vec<InternalStep>,
    pub checkpoints: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub finalized: bool,
}

impl InternalChain {
    pub fn new(strategy: ReasoningStrategy) -> Self {
        Self {
            id: Uuid::new_v4(),
            strategy,
            steps: Vec::new(),
            checkpoints: Vec::new(),
            created_at: Utc::now(),
            finalized: false,
        }
    }

    pub fn add_step(&mut self, step: InternalStep) {
        self.steps.push(step);
    }

    pub fn add_checkpoint(&mut self) -> Uuid {
        let checkpoint = InternalStep::new(
            StepType::Checkpoint,
            format!("Checkpoint at step {}", self.steps.len()),
            1.0,
        );
        let id = checkpoint.id;
        self.checkpoints.push(id);
        self.steps.push(checkpoint);
        id
    }

    pub fn last_step(&self) -> Option<&InternalStep> {
        self.steps.iter().rev().find(|s| s.step_type != StepType::Checkpoint)
    }

    pub fn step_count(&self) -> usize {
        self.steps.iter().filter(|s| s.step_type != StepType::Checkpoint).count()
    }

    pub fn average_confidence(&self) -> f32 {
        let relevant: Vec<&InternalStep> = self
            .steps
            .iter()
            .filter(|s| s.step_type != StepType::Checkpoint)
            .collect();
        if relevant.is_empty() {
            return 0.0;
        }
        let sum: f32 = relevant.iter().map(|s| s.confidence).sum();
        sum / relevant.len() as f32
    }

    pub fn min_confidence(&self) -> f32 {
        self.steps
            .iter()
            .filter(|s| s.step_type != StepType::Checkpoint)
            .map(|s| s.confidence)
            .fold(1.0f32, f32::min)
    }

    pub fn finalize(&mut self) {
        self.finalized = true;
    }

    pub fn is_finalized(&self) -> bool {
        self.finalized
    }

    pub fn prune_below_confidence(&mut self, threshold: f32) {
        self.steps.retain(|s| {
            s.step_type == StepType::Checkpoint || s.confidence >= threshold
        });
    }

    pub fn get_conclusion(&self) -> Option<&InternalStep> {
        self.steps
            .iter()
            .rev()
            .find(|s| s.step_type == StepType::Conclusion)
    }

    pub fn get_premises(&self) -> Vec<&InternalStep> {
        self.steps
            .iter()
            .filter(|s| s.step_type == StepType::Premise)
            .collect()
    }

    pub fn get_inferences(&self) -> Vec<&InternalStep> {
        self.steps
            .iter()
            .filter(|s| s.step_type == StepType::Inference)
            .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InternalReasoningState {
    pub chains: Vec<InternalChain>,
    pub active_chain_id: Option<Uuid>,
    pub accumulated_evidence: Vec<String>,
    pub rejected_paths: Vec<String>,
    pub working_memory: HashMap<String, serde_json::Value>,
}

impl InternalReasoningState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_chain(&mut self, strategy: ReasoningStrategy) -> Uuid {
        let chain = InternalChain::new(strategy);
        let id = chain.id;
        self.chains.push(chain);
        self.active_chain_id = Some(id);
        id
    }

    pub fn active_chain(&self) -> Option<&InternalChain> {
        self.active_chain_id
            .and_then(|id| self.chains.iter().find(|c| c.id == id))
    }

    pub fn active_chain_mut(&mut self) -> Option<&mut InternalChain> {
        self.active_chain_id
            .and_then(|id| self.chains.iter_mut().find(|c| c.id == id))
    }

    pub fn finalize_active_chain(&mut self) {
        if let Some(id) = self.active_chain_id {
            if let Some(chain) = self.chains.iter_mut().find(|c| c.id == id) {
                chain.finalize();
            }
        }
        self.active_chain_id = None;
    }

    pub fn best_chain(&self) -> Option<&InternalChain> {
        self.chains
            .iter()
            .filter(|c| c.is_finalized() && c.get_conclusion().is_some())
            .max_by(|a, b| {
                a.average_confidence()
                    .partial_cmp(&b.average_confidence())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn all_finalized_chains(&self) -> Vec<&InternalChain> {
        self.chains
            .iter()
            .filter(|c| c.is_finalized())
            .collect()
    }

    pub fn add_evidence(&mut self, evidence: String) {
        self.accumulated_evidence.push(evidence);
    }

    pub fn reject_path(&mut self, reason: String) {
        self.rejected_paths.push(reason);
    }

    pub fn store_working(&mut self, key: String, value: serde_json::Value) {
        self.working_memory.insert(key, value);
    }

    pub fn get_working(&self, key: &str) -> Option<&serde_json::Value> {
        self.working_memory.get(key)
    }
}
