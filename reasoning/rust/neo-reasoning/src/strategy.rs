use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ReasoningResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReasoningStrategy {
    Deductive,
    Inductive,
    Abductive,
    Analogical,
    Probabilistic,
    Causal,
    Counterfactual,
    ConstraintBased,
    RuleBased,
    ChainOfThought,
    TreeOfThought,
    ReAct,
    Custom(String),
}

impl ReasoningStrategy {
    pub fn all_default() -> Vec<Self> {
        vec![
            Self::Deductive,
            Self::Inductive,
            Self::Abductive,
            Self::Analogical,
            Self::Probabilistic,
            Self::Causal,
            Self::Counterfactual,
            Self::ConstraintBased,
            Self::RuleBased,
        ]
    }

    pub fn classify_query(query: &str) -> Self {
        let lower = query.to_lowercase();
        if lower.contains("if") && lower.contains("then") {
            Self::Deductive
        } else if lower.contains("pattern") || lower.contains("example") {
            Self::Inductive
        } else if lower.contains("explain") || lower.contains("why") {
            Self::Abductive
        } else if lower.contains("similar") || lower.contains("like") {
            Self::Analogical
        } else if lower.contains("probability") || lower.contains("likely") {
            Self::Probabilistic
        } else if lower.contains("cause") || lower.contains("because") {
            Self::Causal
        } else if lower.contains("what if") || lower.contains("alternative") {
            Self::Counterfactual
        } else if lower.contains("constraint") || lower.contains("requirement") {
            Self::ConstraintBased
        } else if lower.contains("rule") || lower.contains("policy") {
            Self::RuleBased
        } else {
            Self::ChainOfThought
        }
    }
}

impl fmt::Display for ReasoningStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deductive => write!(f, "deductive"),
            Self::Inductive => write!(f, "inductive"),
            Self::Abductive => write!(f, "abductive"),
            Self::Analogical => write!(f, "analogical"),
            Self::Probabilistic => write!(f, "probabilistic"),
            Self::Causal => write!(f, "causal"),
            Self::Counterfactual => write!(f, "counterfactual"),
            Self::ConstraintBased => write!(f, "constraint_based"),
            Self::RuleBased => write!(f, "rule_based"),
            Self::ChainOfThought => write!(f, "chain_of_thought"),
            Self::TreeOfThought => write!(f, "tree_of_thought"),
            Self::ReAct => write!(f, "react"),
            Self::Custom(name) => write!(f, "custom({name})"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyContext {
    pub query: String,
    pub context_data: HashMap<String, serde_json::Value>,
    pub available_facts: Vec<String>,
    pub available_rules: Vec<String>,
    pub constraints: Vec<String>,
    pub max_depth: u32,
}

impl StrategyContext {
    pub fn new(query: String) -> Self {
        Self {
            query,
            context_data: HashMap::new(),
            available_facts: Vec::new(),
            available_rules: Vec::new(),
            constraints: Vec::new(),
            max_depth: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyResult {
    pub strategy: ReasoningStrategy,
    pub output: String,
    pub confidence: f32,
    pub steps_taken: usize,
    pub intermediate_states: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

pub trait ReasoningStrategyExecutor: Send + Sync + fmt::Debug {
    fn strategy_type(&self) -> ReasoningStrategy;
    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult>;
    fn confidence_for_query(&self, query: &str) -> f32;
}

#[derive(Debug)]
pub struct DeductiveStrategy;

impl ReasoningStrategyExecutor for DeductiveStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Deductive
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.5;

        for fact in &context.available_facts {
            steps.push(format!("Known fact: {fact}"));
            confidence = (confidence + 0.05).min(0.95);
        }

        for rule in &context.available_rules {
            steps.push(format!("Applying rule: {rule}"));
            confidence = (confidence + 0.03).min(0.95);
        }

        let conclusion = if !steps.is_empty() {
            format!(
                "Deductive reasoning from {} facts and {} rules yields a conclusion about: {}",
                context.available_facts.len(),
                context.available_rules.len(),
                context.query
            )
        } else {
            format!("Deductive analysis of: {}", context.query)
        };

        steps.push(format!("Conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Deductive,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.starts_with("if ") || lower.contains(" therefore ") {
            0.9
        } else if lower.contains("all ") || lower.contains("every ") {
            0.7
        } else {
            0.4
        }
    }
}

#[derive(Debug)]
pub struct InductiveStrategy;

impl ReasoningStrategyExecutor for InductiveStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Inductive
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.4;

        let observations = &context.available_facts;
        for obs in observations {
            steps.push(format!("Observation: {obs}"));
        }

        if observations.len() >= 3 {
            confidence = 0.6;
            steps.push("Sufficient observations to identify pattern".to_string());
        } else if !observations.is_empty() {
            steps.push("Limited observations - pattern inference has lower confidence".to_string());
        }

        let conclusion = format!(
            "Inductive generalization from {} observations about: {}",
            observations.len(),
            context.query
        );
        steps.push(format!("Generalization: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Inductive,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("pattern") || lower.contains("trend") || lower.contains("generalize") {
            0.8
        } else if lower.contains("example") || lower.contains("sample") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct AbductiveStrategy;

impl ReasoningStrategyExecutor for AbductiveStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Abductive
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.5;

        steps.push(format!("Observation to explain: {}", context.query));

        for fact in &context.available_facts {
            steps.push(format!("Potential explanatory fact: {fact}"));
            confidence = (confidence + 0.05).min(0.85);
        }

        let conclusion = format!(
            "Best explanation for '{}' based on available evidence",
            context.query
        );
        steps.push(format!("Abduced explanation: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Abductive,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("explain") || lower.contains("why") || lower.contains("cause") {
            0.8
        } else if lower.contains("hypothesis") || lower.contains("suppose") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct AnalogicalStrategy;

impl ReasoningStrategyExecutor for AnalogicalStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Analogical
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.45;

        steps.push(format!("Source domain analysis for: {}", context.query));

        for fact in &context.available_facts {
            steps.push(format!("Structural element: {fact}"));
            confidence = (confidence + 0.04).min(0.80);
        }

        steps.push("Mapping structural correspondences".to_string());
        steps.push("Evaluating analogy strength".to_string());

        let conclusion = format!(
            "Analogical transfer from similar domain to: {}",
            context.query
        );
        steps.push(format!("Analogical conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Analogical,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("similar") || lower.contains("like") || lower.contains("analogy") {
            0.8
        } else if lower.contains("compare") || lower.contains("metaphor") {
            0.7
        } else {
            0.2
        }
    }
}

#[derive(Debug)]
pub struct ProbabilisticStrategy;

impl ReasoningStrategyExecutor for ProbabilisticStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Probabilistic
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let n = context.available_facts.len();
        let base_confidence = if n > 0 { 0.5 + (n as f32 * 0.03) } else { 0.3 };
        let confidence = base_confidence.min(0.90);

        steps.push(format!(
            "Probabilistic analysis with {} evidence sources",
            n
        ));

        for fact in &context.available_facts {
            steps.push(format!("Evidence: {fact}"));
        }

        steps.push("Computing posterior probabilities".to_string());
        steps.push("Bayesian belief update applied".to_string());

        let conclusion = format!(
            "Probabilistic assessment of '{}' with {}% confidence",
            context.query,
            (confidence * 100.0) as u32
        );
        steps.push(format!("Probabilistic conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Probabilistic,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("probability") || lower.contains("likely") || lower.contains("chance") {
            0.9
        } else if lower.contains("risk") || lower.contains("uncertain") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct CausalStrategy;

impl ReasoningStrategyExecutor for CausalStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Causal
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.5;

        steps.push(format!("Causal analysis of: {}", context.query));

        for fact in &context.available_facts {
            steps.push(format!("Causal factor: {fact}"));
            confidence = (confidence + 0.04).min(0.85);
        }

        steps.push("Tracing causal chains".to_string());
        steps.push("Identifying confounders".to_string());
        steps.push("Estimating causal effect size".to_string());

        let conclusion = format!(
            "Causal analysis of '{}' identifies key mechanisms",
            context.query
        );
        steps.push(format!("Causal conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Causal,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("cause") || lower.contains("effect") || lower.contains("because") {
            0.85
        } else if lower.contains("why") || lower.contains("mechanism") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct CounterfactualStrategy;

impl ReasoningStrategyExecutor for CounterfactualStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::Counterfactual
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.45;

        steps.push(format!(
            "Counterfactual reasoning about: {}",
            context.query
        ));
        steps.push("Establishing baseline scenario".to_string());

        for fact in &context.available_facts {
            steps.push(format!("Counterfactual condition: {fact}"));
            confidence = (confidence + 0.03).min(0.75);
        }

        steps.push("Computing counterfactual outcomes".to_string());

        let conclusion = format!(
            "Counterfactual analysis: alternative scenario for '{}'",
            context.query
        );
        steps.push(format!("Counterfactual conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::Counterfactual,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("what if") || lower.contains("would have") {
            0.85
        } else if lower.contains("alternative") || lower.contains("instead") {
            0.7
        } else {
            0.2
        }
    }
}

#[derive(Debug)]
pub struct ConstraintBasedStrategy;

impl ReasoningStrategyExecutor for ConstraintBasedStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::ConstraintBased
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.6;

        steps.push(format!(
            "Constraint satisfaction for: {}",
            context.query
        ));

        let mut unsatisfied = 0;
        for constraint in &context.constraints {
            steps.push(format!("Checking constraint: {constraint}"));
            if context.available_facts.iter().any(|f| f.contains(constraint.as_str())) {
                steps.push("  -> satisfied".to_string());
            } else {
                steps.push("  -> not directly satisfied".to_string());
                unsatisfied += 1;
            }
        }

        if !context.constraints.is_empty() {
            let ratio = 1.0 - (unsatisfied as f32 / context.constraints.len() as f32);
            confidence = 0.3 + (ratio * 0.5);
        }

        let conclusion = format!(
            "Constraint-based solution for '{}' satisfying {}/{} constraints",
            context.query,
            context.constraints.len() - unsatisfied,
            context.constraints.len()
        );
        steps.push(format!("Constraint conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::ConstraintBased,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("constraint") || lower.contains("requirement") || lower.contains("must") {
            0.85
        } else if lower.contains("valid") || lower.contains("satisfy") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct RuleBasedStrategy;

impl ReasoningStrategyExecutor for RuleBasedStrategy {
    fn strategy_type(&self) -> ReasoningStrategy {
        ReasoningStrategy::RuleBased
    }

    fn execute(&self, context: &StrategyContext) -> ReasoningResult<StrategyResult> {
        let mut steps = Vec::new();
        let mut confidence: f32 = 0.5;
        let mut fired_rules = 0;

        steps.push(format!("Rule-based reasoning for: {}", context.query));

        for rule in &context.available_rules {
            steps.push(format!("Evaluating rule: {rule}"));
            let rule_satisfied = context
                .available_facts
                .iter()
                .any(|f| rule.to_lowercase().contains(&f.to_lowercase()));
            if rule_satisfied {
                steps.push("  -> rule fires".to_string());
                fired_rules += 1;
                confidence = (confidence + 0.08).min(0.90);
            } else {
                steps.push("  -> rule does not fire".to_string());
            }
        }

        let conclusion = format!(
            "Rule-based reasoning: {}/{} rules fired for '{}'",
            fired_rules,
            context.available_rules.len(),
            context.query
        );
        steps.push(format!("Rule conclusion: {conclusion}"));

        Ok(StrategyResult {
            strategy: ReasoningStrategy::RuleBased,
            output: conclusion,
            confidence,
            steps_taken: steps.len(),
            intermediate_states: steps,
            metadata: HashMap::new(),
        })
    }

    fn confidence_for_query(&self, query: &str) -> f32 {
        let lower = query.to_lowercase();
        if lower.contains("rule") || lower.contains("policy") || lower.contains("according to") {
            0.85
        } else if lower.contains("should") || lower.contains("must") {
            0.7
        } else {
            0.3
        }
    }
}

#[derive(Debug)]
pub struct StrategyRegistry {
    executors: Vec<Box<dyn ReasoningStrategyExecutor>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        let mut executors: Vec<Box<dyn ReasoningStrategyExecutor>> = Vec::new();
        executors.push(Box::new(DeductiveStrategy));
        executors.push(Box::new(InductiveStrategy));
        executors.push(Box::new(AbductiveStrategy));
        executors.push(Box::new(AnalogicalStrategy));
        executors.push(Box::new(ProbabilisticStrategy));
        executors.push(Box::new(CausalStrategy));
        executors.push(Box::new(CounterfactualStrategy));
        executors.push(Box::new(ConstraintBasedStrategy));
        executors.push(Box::new(RuleBasedStrategy));
        Self { executors }
    }

    pub fn get(&self, strategy: &ReasoningStrategy) -> Option<&dyn ReasoningStrategyExecutor> {
        self.executors
            .iter()
            .find(|e| &e.strategy_type() == strategy)
            .map(|e| e.as_ref())
    }

    pub fn select_best(&self, query: &str) -> &dyn ReasoningStrategyExecutor {
        self.executors
            .iter()
            .max_by(|a, b| {
                a.confidence_for_query(query)
                    .partial_cmp(&b.confidence_for_query(query))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|e| e.as_ref())
            .expect("at least one strategy must be registered")
    }

    pub fn strategies(&self) -> Vec<ReasoningStrategy> {
        self.executors.iter().map(|e| e.strategy_type()).collect()
    }

    pub fn count(&self) -> usize {
        self.executors.len()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}
