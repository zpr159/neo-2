use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, HashSet};

use crate::types::{Confidence, EventId, CausalLinkId, AttributeValue};

/// Strength of a causal link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CausalStrength {
    Definite,
    Strong,
    Moderate,
    Weak,
    Speculative,
}

impl CausalStrength {
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Definite => 1.0,
            Self::Strong => 0.8,
            Self::Moderate => 0.5,
            Self::Weak => 0.3,
            Self::Speculative => 0.1,
        }
    }
}

impl std::fmt::Display for CausalStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Definite => write!(f, "definite"),
            Self::Strong => write!(f, "strong"),
            Self::Moderate => write!(f, "moderate"),
            Self::Weak => write!(f, "weak"),
            Self::Speculative => write!(f, "speculative"),
        }
    }
}

/// A directed causal link between two events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalLink {
    pub id: CausalLinkId,
    pub cause: EventId,
    pub effect: EventId,
    pub strength: CausalStrength,
    pub explanation: String,
    pub confidence: Confidence,
    pub probability: f32,
    pub properties: HashMap<String, AttributeValue>,
}

/// A chain of causally connected events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    pub events: Vec<EventId>,
    pub strength: CausalStrength,
    pub description: String,
}

/// Result of a counterfactual reasoning query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualResult {
    pub original_outcome: String,
    pub counterfactual_outcome: String,
    pub changed_event: EventId,
    pub impact_description: String,
    pub confidence: Confidence,
}

/// Manages causal relationships between events.
pub struct CausalModel {
    links: Vec<CausalLink>,
    cause_to_effect: HashMap<EventId, Vec<CausalLinkId>>,
    effect_to_cause: HashMap<EventId, Vec<CausalLinkId>>,
    link_index: HashMap<CausalLinkId, usize>,
}

impl CausalModel {
    #[must_use]
    pub fn new() -> Self {
        Self {
            links: Vec::new(),
            cause_to_effect: HashMap::new(),
            effect_to_cause: HashMap::new(),
            link_index: HashMap::new(),
        }
    }

    pub fn add_link(&mut self, link: CausalLink) {
        let idx = self.links.len();
        self.link_index.insert(link.id.clone(), idx);
        self.cause_to_effect
            .entry(link.cause.clone())
            .or_default()
            .push(link.id.clone());
        self.effect_to_cause
            .entry(link.effect.clone())
            .or_default()
            .push(link.id.clone());
        self.links.push(link);
    }

    pub fn effects_of(&self, cause: &EventId) -> Vec<&CausalLink> {
        self.links
            .iter()
            .filter(|l| &l.cause == cause)
            .collect()
    }

    pub fn causes_of(&self, effect: &EventId) -> Vec<&CausalLink> {
        self.links
            .iter()
            .filter(|l| &l.effect == effect)
            .collect()
    }

    pub fn root_causes(&self, event_id: &EventId) -> Vec<EventId> {
        let mut roots = Vec::new();
        let mut visited = HashSet::new();
        self.trace_root_causes(event_id, &mut roots, &mut visited);
        roots
    }

    fn trace_root_causes(&self, event_id: &EventId, roots: &mut Vec<EventId>, visited: &mut HashSet<EventId>) {
        if visited.contains(event_id) {
            return;
        }
        visited.insert(event_id.clone());
        if let Some(causes) = self.effect_to_cause.get(event_id) {
            if causes.is_empty() {
                roots.push(event_id.clone());
            } else {
                for cause_id in causes {
                    if let Some(link) = self.get_link(cause_id) {
                        self.trace_root_causes(&link.cause, roots, visited);
                    }
                }
            }
        } else {
            roots.push(event_id.clone());
        }
    }

    pub fn all_effects(&self, event_id: &EventId) -> Vec<EventId> {
        let mut effects = Vec::new();
        let mut visited = HashSet::new();
        self.trace_all_effects(event_id, &mut effects, &mut visited);
        effects
    }

    fn trace_all_effects(&self, event_id: &EventId, effects: &mut Vec<EventId>, visited: &mut HashSet<EventId>) {
        if visited.contains(event_id) {
            return;
        }
        visited.insert(event_id.clone());
        if let Some(link_ids) = self.cause_to_effect.get(event_id) {
            for link_id in link_ids {
                if let Some(link) = self.get_link(link_id) {
                    effects.push(link.effect.clone());
                    self.trace_all_effects(&link.effect, effects, visited);
                }
            }
        }
    }

    pub fn find_path(&self, from: &EventId, to: &EventId) -> Option<Vec<EventId>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<EventId, EventId> = HashMap::new();

        queue.push_back(from.clone());
        visited.insert(from.clone());

        while let Some(current) = queue.pop_front() {
            if current == *to {
                let mut path = vec![to.clone()];
                let mut node = to.clone();
                while let Some(p) = parent.get(&node) {
                    path.push(p.clone());
                    node = p.clone();
                }
                path.reverse();
                return Some(path);
            }
            if let Some(link_ids) = self.cause_to_effect.get(&current) {
                for link_id in link_ids {
                    if let Some(link) = self.get_link(link_id) {
                        if !visited.contains(&link.effect) {
                            visited.insert(link.effect.clone());
                            parent.insert(link.effect.clone(), current.clone());
                            queue.push_back(link.effect.clone());
                        }
                    }
                }
            }
        }
        None
    }

    pub fn counterfactual(
        &self,
        blocked_event: &EventId,
        original_outcome: &str,
    ) -> CounterfactualResult {
        let downstream = self.all_effects(blocked_event);
        CounterfactualResult {
            original_outcome: original_outcome.to_string(),
            counterfactual_outcome: format!(
                "If event {blocked_event} had not occurred, {} downstream events would not have happened",
                downstream.len()
            ),
            changed_event: blocked_event.clone(),
            impact_description: format!("{} events affected", downstream.len()),
            confidence: Confidence::LOW,
        }
    }

    pub fn get_link(&self, id: &CausalLinkId) -> Option<&CausalLink> {
        self.link_index.get(id).and_then(|&idx| self.links.get(idx))
    }

    pub fn count(&self) -> usize {
        self.links.len()
    }

    pub fn all_links(&self) -> &[CausalLink] {
        &self.links
    }
}

impl Default for CausalModel {
    fn default() -> Self {
        Self::new()
    }
}
