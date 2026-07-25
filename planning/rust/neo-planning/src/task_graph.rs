use super::types::*;

pub struct TaskGraphBuilder {
    goals: Vec<Goal>,
    strategy: Option<Strategy>,
    dependencies: Vec<(GoalId, GoalId, DependencyType)>,
}

impl TaskGraphBuilder {
    pub fn new(goal: &Goal) -> Self {
        Self {
            goals: vec![goal.clone()],
            strategy: None,
            dependencies: Vec::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn add_dependency(&mut self, from: GoalId, to: GoalId, dep_type: DependencyType) {
        self.dependencies.push((from, to, dep_type));
    }

    pub async fn build(self) -> Result<TaskGraph, PlanningError> {
        let mut nodes = HashMap::new();
        for goal in &self.goals {
            nodes.insert(goal.id, PlanningNode {
                id: PlanningNodeId(goal.id.0),
                label: goal.metadata.name.clone(),
                node_type: PlanningNodeType::Task,
                goal_id: Some(goal.id),
                cost: goal.budget.max_cost,
                metadata: HashMap::new(),
            });
        }

        let edges = self.dependencies.iter().map(|(from, to, _)| {
            PlanningEdge {
                from: PlanningNodeId(from.0),
                to: PlanningNodeId(to.0),
                weight: 1.0,
                label: String::new(),
            }
        }).collect();

        let mut graph = TaskGraph {
            nodes,
            edges,
            strategy: self.strategy.unwrap_or_else(|| Strategy {
                id: StrategyId::new(),
                name: "Default".to_string(),
                evaluation: StrategyComparison {
                    cost: 0.0,
                    duration_ms: 0,
                    probability_of_success: 1.0,
                    resource_consumption: HashMap::new(),
                    risk_score: 0.0,
                    complexity_score: 0.0,
                },
            }),
        };

        graph.validate()?;
        Ok(graph)
    }
}

pub struct TaskGraph {
    pub nodes: HashMap<PlanningNodeId, PlanningNode>,
    pub edges: Vec<PlanningEdge>,
    pub strategy: Strategy,
}

impl TaskGraph {
    pub fn new(goal: &Goal) -> Self {
        TaskGraphBuilder::new(goal)
            .build()
            .unwrap_or_else(|_| panic!("Failed to build task graph"))
    }

    pub fn with_strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn validate(&self) -> Result<(), PlanningError> {
        let mut in_degree = HashMap::new();
        for node in self.nodes.values() {
            in_degree.entry(node.id).or_insert(0);
        }
        for edge in &self.edges {
            *in_degree.entry(edge.to).or_insert(0) += 1;
        }

        for (node_id, degree) in &in_degree {
            if *degree > 1 {
                return Err(PlanningError::new(
                    PlanningErrorCode::PlanGraphCycleDetected,
                    format!("Node {} has {} incoming edges", node_id, degree),
                ));
            }
        }
        Ok(())
    }

    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        for node_id in self.nodes.keys() {
            if Self::dfs_cycle_check(self, *node_id, &mut visited, &mut recursion_stack) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle_check(graph: &TaskGraph, node_id: PlanningNodeId, visited: &mut HashSet<PlanningNodeId>, recursion_stack: &mut HashSet<PlanningNodeId>) -> bool {
        if recursion_stack.contains(&node_id) {
            return true;
        }
        if visited.contains(&node_id) {
            return false;
        }

        visited.insert(node_id);
        recursion_stack.insert(node_id);

        for edge in &graph.edges {
            if edge.from == node_id {
                if Self::dfs_cycle_check(graph, edge.to, visited, recursion_stack) {
                    return true;
                }
            }
        }

        recursion_stack.remove(&node_id);
        false
    }

    pub fn detect_cycles(&self) -> Result<Vec<Vec<PlanningNodeId>>, PlanningError> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();

        for node_id in self.nodes.keys() {
            if !visited.contains(node_id) {
                let mut cycle = Vec::new();
                Self::dfs_cycle_detection(self, *node_id, &mut visited, &mut parent, &mut cycle);
                if !cycle.is_empty() {
                    cycles.push(cycle);
                }
            }
        }

        if cycles.is_empty() {
            Ok(cycles)
        } else {
            Err(PlanningError::new(
                PlanningErrorCode::PlanGraphCycleDetected,
                "Graph contains cycles",
            ))
        }
    }

    fn dfs_cycle_detection(graph: &TaskGraph, current: PlanningNodeId, visited: &mut HashSet<PlanningNodeId>, parent: &mut HashMap<PlanningNodeId, PlanningNodeId>, cycle: &mut Vec<PlanningNodeId>) {
        visited.insert(current);
        cycle.push(current);

        for edge in &graph.edges {
            if edge.from == current {
                if !visited.contains(&edge.to) {
                    parent.insert(edge.to, current);
                    Self::dfs_cycle_detection(graph, edge.to, visited, parent, cycle);
                } else if cycle.contains(&edge.to) && edge.to != *cycle.first().unwrap_or(&PlanningNodeId(Uuid::nil())) {
                    let start_idx = cycle.iter().position(|&id| id == edge.to).unwrap();
                    *cycle = cycle[start_idx..].to_vec();
                }
            }
        }

        if !cycle.is_empty() && cycle.first() == Some(&current) {
            return;
        }

        cycle.pop();
    }
}

#[derive(Debug, Clone)]
pub struct PlanningNode {
    pub id: PlanningNodeId,
    pub label: String,
    pub node_type: PlanningNodeType,
    pub goal_id: Option<PlanningGoalId>,
    pub cost: f64,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub enum PlanningNodeType {
    Start,
    End,
    Task,
    Decision,
    Parallel,
    Milestone,
    Composite,
}

#[derive(Debug, Clone)]
pub struct PlanningEdge {
    pub from: PlanningNodeId,
    pub to: PlanningNodeId,
    pub weight: f64,
    pub label: String,
}

impl PlanningEdge {
    pub fn new(from: PlanningNodeId, to: PlanningNodeId) -> Self {
        Self { from, to, weight: 1.0, label: String::new() }
    }
}

impl TaskGraph {
    pub fn with_resource_allocation(mut self, allocation: ResourceAllocation) -> Self {
        self.strategy.evaluation.resource_consumption = allocation.allocated_resources.clone();
        self
    }
}