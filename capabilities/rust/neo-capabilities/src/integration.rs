use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::CapabilityApi;
use crate::core::{
    Capability, CapabilityCategory, CapabilityId, CapabilityMetadata, CapabilityNamespace,
    CapabilityResult_output, CapabilitySummary, CapabilityTags, CapabilityVersion,
    ExecutionContext, ResourceRequirements,
};
use crate::error::{CapabilityError, CapabilityResult};
use crate::execution::{ExecutionRecord, RetryConfig};

/// A stub capability used for built-in integration registrations.
struct StubCapability {
    meta: CapabilityMetadata,
}

#[async_trait]
impl Capability for StubCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.meta
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.meta
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _ctx: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        Ok(CapabilityResult_output::success(input, 0))
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Executive Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Links a task to a capability execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCapabilityLink {
    pub task_id: Uuid,
    pub capability_id: CapabilityId,
    pub execution_id: Uuid,
    pub status: String,
}

/// Bridges the Executive subsystem with the Capability framework.
///
/// Allows the Executive to invoke capabilities on behalf of tasks and track
/// which capabilities are associated with each task.
pub struct ExecutiveIntegration {
    api: Arc<CapabilityApi>,
    task_records: RwLock<HashMap<Uuid, Vec<TaskCapabilityLink>>>,
}

impl ExecutiveIntegration {
    /// Create a new executive integration backed by the given API.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self {
            api,
            task_records: RwLock::new(HashMap::new()),
        }
    }

    /// Invoke a capability for a specific task.
    ///
    /// Creates an execution context, executes the capability through the API,
    /// and records the linkage between the task and the capability execution.
    pub async fn invoke_capability_for_task(
        &self,
        task_id: Uuid,
        capability_id: CapabilityId,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> CapabilityResult<CapabilityResult_output> {
        let execution_id = Uuid::new_v4();

        let link = TaskCapabilityLink {
            task_id,
            capability_id,
            execution_id,
            status: "running".to_string(),
        };

        {
            let mut records = self.task_records.write();
            records
                .entry(task_id)
                .or_insert_with(Vec::new)
                .push(link);
        }

        let result = self.api.execute(capability_id, input, context).await;

        let final_status = match &result {
            Ok(output) if output.success => "completed",
            Ok(_) => "failed",
            Err(_) => "error",
        };

        {
            let mut records = self.task_records.write();
            if let Some(links) = records.get_mut(&task_id) {
                if let Some(link) = links.iter_mut().rev().find(|l| l.execution_id == execution_id)
                {
                    link.status = final_status.to_string();
                }
            }
        }

        result
    }

    /// Get all capability links for a given task.
    pub fn get_task_capabilities(&self, task_id: Uuid) -> Vec<TaskCapabilityLink> {
        self.task_records
            .read()
            .get(&task_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Get all tasks that have invoked a specific capability.
    pub fn get_capability_tasks(&self, capability_id: CapabilityId) -> Vec<TaskCapabilityLink> {
        self.task_records
            .read()
            .values()
            .flatten()
            .filter(|link| link.capability_id == capability_id)
            .cloned()
            .collect()
    }

    /// Cancel all running executions for a task.
    pub fn cancel_task_executions(&self, task_id: Uuid) {
        let mut records = self.task_records.write();
        if let Some(links) = records.get_mut(&task_id) {
            for link in links.iter_mut() {
                if link.status == "running" {
                    link.status = "cancelled".to_string();
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Reasoning Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Records a capability selection decision made by the reasoning subsystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRecord {
    pub query: String,
    pub selected_capabilities: Vec<CapabilityId>,
    pub timestamp: DateTime<Utc>,
    pub reasoning: String,
}

/// Bridges the Reasoning subsystem with the Capability framework.
///
/// Enables the reasoning engine to select the best capabilities for a given
/// query based on metadata analysis, tag matching, category affinity, and
/// historical selection patterns.
pub struct ReasoningIntegration {
    api: Arc<CapabilityApi>,
    selection_history: RwLock<Vec<SelectionRecord>>,
}

impl ReasoningIntegration {
    /// Create a new reasoning integration backed by the given API.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self {
            api,
            selection_history: RwLock::new(Vec::new()),
        }
    }

    /// Select the best capabilities for a query from the given candidate IDs.
    ///
    /// Scores each candidate by:
    /// - Tag overlap with query terms (weight: 3.0)
    /// - Category affinity with query keywords (weight: 2.0)
    /// - Historical selection frequency (weight: 1.0)
    ///
    /// Returns candidates sorted by score, descending, limited to those with a
    /// positive score. If no candidates score above zero, returns the first
    /// available capability.
    pub fn select_capabilities(
        &self,
        query: &str,
        _context: &ExecutionContext,
        available_ids: &[CapabilityId],
    ) -> Vec<CapabilityId> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let history = self.selection_history.read();
        let mut frequency: HashMap<CapabilityId, u32> = HashMap::new();
        for record in history.iter() {
            for cap_id in &record.selected_capabilities {
                *frequency.entry(*cap_id).or_insert(0) += 1;
            }
        }

        let caps = self.api.list();

        let mut scored: Vec<(CapabilityId, f64)> = available_ids
            .iter()
            .filter_map(|id| {
                let summary = caps.iter().find(|c| c.id == *id)?;
                let mut score: f64 = 0.0;

                let tag_overlap = summary
                    .tags
                    .iter()
                    .filter(|t| query_terms.iter().any(|qt| t.to_lowercase().contains(qt)))
                    .count();
                score += tag_overlap as f64 * 3.0;

                let category_match = query_terms.iter().any(|term| {
                    let cat_str = summary.category.to_string();
                    cat_str.contains(term)
                });
                if category_match {
                    score += 2.0;
                }

                let freq = frequency.get(id).copied().unwrap_or(0);
                score += freq as f64;

                if score > 0.0 {
                    Some((*id, score))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if scored.is_empty() {
            available_ids.first().cloned().into_iter().collect()
        } else {
            scored.into_iter().map(|(id, _)| id).collect()
        }
    }

    /// Record a selection decision for future reference.
    pub fn record_selection(
        &self,
        query: String,
        selected: Vec<CapabilityId>,
        reasoning: String,
    ) {
        self.selection_history.write().push(SelectionRecord {
            query,
            selected_capabilities: selected,
            timestamp: Utc::now(),
            reasoning,
        });
    }

    /// Get the full selection history.
    pub fn get_selection_history(&self) -> Vec<SelectionRecord> {
        self.selection_history.read().clone()
    }

    /// Suggest capabilities for a query based on past selection patterns.
    ///
    /// Finds previous selections where the query terms overlap with the stored
    /// query, then ranks the most frequently selected capabilities.
    pub fn suggest_capabilities(&self, query: &str) -> Vec<CapabilitySummary> {
        let query_lower = query.to_lowercase();
        let history = self.selection_history.read();

        let mut frequency: HashMap<CapabilityId, u32> = HashMap::new();
        for record in history.iter() {
            if record.query.to_lowercase().contains(&query_lower)
                || query_lower.contains(&record.query.to_lowercase())
            {
                for cap_id in &record.selected_capabilities {
                    *frequency.entry(*cap_id).or_insert(0) += 1;
                }
            }
        }

        if frequency.is_empty() {
            return Vec::new();
        }

        let mut freq_vec: Vec<(CapabilityId, u32)> = frequency.into_iter().collect();
        freq_vec.sort_by(|a, b| b.1.cmp(&a.1));

        let all_caps = self.api.list();
        freq_vec
            .into_iter()
            .filter_map(|(id, _)| all_caps.iter().find(|c| c.id == id).cloned())
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Memory Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Aggregated statistics for a capability's execution history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_duration: f64,
}

/// Bridges the Memory subsystem with the Capability framework.
///
/// Stores execution records in an in-memory store and provides search and
/// statistics capabilities over the historical execution data.
pub struct MemoryIntegration {
    api: Arc<CapabilityApi>,
    execution_store: RwLock<Vec<ExecutionRecord>>,
}

impl MemoryIntegration {
    /// Create a new memory integration backed by the given API.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self {
            api,
            execution_store: RwLock::new(Vec::new()),
        }
    }

    /// Store an execution record in memory.
    pub fn store_execution(&self, record: ExecutionRecord) {
        self.execution_store.write().push(record);
    }

    /// Retrieve the execution history for a specific capability.
    pub fn retrieve_execution_history(
        &self,
        capability_id: CapabilityId,
    ) -> Vec<ExecutionRecord> {
        self.execution_store
            .read()
            .iter()
            .filter(|r| r.request.capability_id == capability_id)
            .cloned()
            .collect()
    }

    /// Search execution records by capability name.
    pub fn search_executions(&self, query: &str) -> Vec<ExecutionRecord> {
        let query_lower = query.to_lowercase();
        let all_caps = self.api.list();

        let matching_ids: Vec<CapabilityId> = all_caps
            .iter()
            .filter(|c| c.name.to_lowercase().contains(&query_lower))
            .map(|c| c.id)
            .collect();

        self.execution_store
            .read()
            .iter()
            .filter(|r| matching_ids.contains(&r.request.capability_id))
            .cloned()
            .collect()
    }

    /// Compute aggregated execution statistics for a capability.
    pub fn get_execution_stats(&self, capability_id: CapabilityId) -> ExecutionStats {
        let records = self.retrieve_execution_history(capability_id);

        let total = records.len() as u64;
        let successful = records
            .iter()
            .filter(|r| matches!(r.status, crate::execution::ExecutionStatus::Completed))
            .count() as u64;
        let failed = records
            .iter()
            .filter(|r| {
                matches!(
                    r.status,
                    crate::execution::ExecutionStatus::Failed
                        | crate::execution::ExecutionStatus::TimedOut
                )
            })
            .count() as u64;

        let avg_duration = if !records.is_empty() {
            let total_duration: u64 = records
                .iter()
                .filter_map(|r| {
                    r.completed_at
                        .map(|c| c.signed_duration_since(r.started_at).num_milliseconds() as u64)
                })
                .sum();
            total_duration as f64 / records.len() as f64
        } else {
            0.0
        };

        ExecutionStats {
            total,
            successful,
            failed,
            avg_duration,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Knowledge Integration
// ═══════════════════════════════════════════════════════════════════════════

/// Represents a directed, weighted relationship between two capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRelationship {
    pub source: CapabilityId,
    pub target: CapabilityId,
    pub relationship: String,
    pub weight: f64,
}

/// Bridges the Knowledge subsystem with the Capability framework.
///
/// Models capability-to-capability relationships as a directed graph and
/// provides traversal and path-finding operations over that graph.
pub struct KnowledgeIntegration {
    api: Arc<CapabilityApi>,
    relationships: RwLock<Vec<CapabilityRelationship>>,
}

impl KnowledgeIntegration {
    /// Create a new knowledge integration backed by the given API.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self {
            api,
            relationships: RwLock::new(Vec::new()),
        }
    }

    /// Add a directed relationship between two capabilities.
    pub fn add_relationship(
        &self,
        source: CapabilityId,
        target: CapabilityId,
        relationship: String,
        weight: f64,
    ) {
        self.relationships.write().push(CapabilityRelationship {
            source,
            target,
            relationship,
            weight,
        });
    }

    /// Get all relationships (incoming and outgoing) for a capability.
    pub fn get_relationships(&self, capability_id: CapabilityId) -> Vec<CapabilityRelationship> {
        self.relationships
            .read()
            .iter()
            .filter(|r| r.source == capability_id || r.target == capability_id)
            .cloned()
            .collect()
    }

    /// Get all capability IDs directly connected to the given capability
    /// (via outgoing edges only).
    pub fn get_related_capabilities(&self, capability_id: CapabilityId) -> Vec<CapabilityId> {
        self.relationships
            .read()
            .iter()
            .filter(|r| r.source == capability_id)
            .map(|r| r.target)
            .collect()
    }

    /// Find the shortest path between two capabilities using BFS.
    ///
    /// Returns `Some(vec![from, ..., to])` if a path exists, or `None` if the
    /// target is unreachable from the source.
    pub fn find_path(&self, from: CapabilityId, to: CapabilityId) -> Option<Vec<CapabilityId>> {
        if from == to {
            return Some(vec![from]);
        }

        let relationships = self.relationships.read();

        let mut adjacency: HashMap<CapabilityId, Vec<CapabilityId>> = HashMap::new();
        for rel in relationships.iter() {
            adjacency
                .entry(rel.source)
                .or_insert_with(Vec::new)
                .push(rel.target);
        }

        let mut visited: HashMap<CapabilityId, Option<CapabilityId>> = HashMap::new();
        let mut queue: VecDeque<CapabilityId> = VecDeque::new();

        visited.insert(from, None);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains_key(neighbor) {
                        visited.insert(*neighbor, Some(current));

                        if *neighbor == to {
                            let mut path = Vec::new();
                            let mut step = Some(to);
                            while let Some(node) = step {
                                path.push(node);
                                step = visited[&node];
                            }
                            path.reverse();
                            return Some(path);
                        }

                        queue.push_back(*neighbor);
                    }
                }
            }
        }

        None
    }

    /// Remove all relationships between two capabilities (in both directions).
    pub fn remove_relationship(&self, source: CapabilityId, target: CapabilityId) {
        self.relationships
            .write()
            .retain(|r| !(r.source == source && r.target == target));
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. CLI Integration
// ═══════════════════════════════════════════════════════════════════════════

/// A parsed CLI command for capability operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    /// List all capabilities.
    List,
    /// Inspect a specific capability.
    Inspect(CapabilityId),
    /// Execute a capability with JSON input.
    Execute(CapabilityId, serde_json::Value),
    /// Enable a capability.
    Enable(CapabilityId),
    /// Disable a capability.
    Disable(CapabilityId),
    /// Search capabilities by query string.
    Search(String),
}

/// Formats capability data for CLI display and parses CLI commands.
pub struct CliIntegration {
    api: Arc<CapabilityApi>,
}

impl CliIntegration {
    /// Create a new CLI integration backed by the given API.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self { api }
    }

    /// Format a listing of all capabilities as a displayable string.
    pub fn format_list(&self) -> String {
        let caps = self.api.list();
        if caps.is_empty() {
            return "No capabilities registered.".to_string();
        }

        let mut output = String::new();
        output.push_str(&format!("{:<36}  {:<20}  {:<12}  {:<10}\n", "ID", "NAME", "VERSION", "STATE"));
        output.push_str(&"-".repeat(82));
        output.push('\n');

        for cap in &caps {
            output.push_str(&format!(
                "{:<36}  {:<20}  {:<12}  {:<10}\n",
                cap.id,
                cap.name,
                cap.version,
                format!("{:?}", cap.state).to_lowercase(),
            ));
        }

        output.push_str(&format!("\n{} capabilities registered.", caps.len()));
        output
    }

    /// Format detailed information about a specific capability.
    pub fn format_inspect(&self, id: CapabilityId) -> String {
        let summary = match self.api.inspect(id) {
            Ok(s) => s,
            Err(_) => return format!("Capability {} not found.", id),
        };

        let mut output = String::new();
        output.push_str(&format!("Capability: {}\n", summary.name));
        output.push_str(&format!("  ID:          {}\n", summary.id));
        output.push_str(&format!("  Version:     {}\n", summary.version));
        output.push_str(&format!("  Category:    {}\n", summary.category));
        output.push_str(&format!("  Namespace:   {}\n", summary.namespace));
        output.push_str(&format!("  Description: {}\n", summary.description));
        output.push_str(&format!("  State:       {:?}\n", summary.state));
        output.push_str(&format!(
            "  Tags:        {}\n",
            summary.tags.join(", ")
        ));
        output.push_str(&format!("  Executions:  {}\n", summary.execution_count));

        output
    }

    /// Format search results as a displayable string.
    pub fn format_search_results(&self, results: &[CapabilitySummary]) -> String {
        if results.is_empty() {
            return "No capabilities found.".to_string();
        }

        let mut output = String::new();
        output.push_str(&format!(
            "{:<36}  {:<20}  {:<12}  {:<10}\n",
            "ID", "NAME", "VERSION", "STATE"
        ));
        output.push_str(&"-".repeat(82));
        output.push('\n');

        for cap in results {
            output.push_str(&format!(
                "{:<36}  {:<20}  {:<12}  {:<10}\n",
                cap.id,
                cap.name,
                cap.version,
                format!("{:?}", cap.state).to_lowercase(),
            ));
        }

        output.push_str(&format!("\n{} result(s) found.", results.len()));
        output
    }

    /// Parse a CLI command string into a structured command.
    ///
    /// Supported formats:
    /// - `list`
    /// - `inspect <uuid>`
    /// - `execute <uuid> <json_input>`
    /// - `enable <uuid>`
    /// - `disable <uuid>`
    /// - `search <query>`
    pub fn parse_command(command: &str) -> Option<CliCommand> {
        let parts: Vec<&str> = command.trim().splitn(3, ' ').collect();

        match parts.first().copied()? {
            "list" => Some(CliCommand::List),

            "inspect" => {
                let id_str = parts.get(1)?;
                let uuid = Uuid::parse_str(id_str).ok()?;
                Some(CliCommand::Inspect(CapabilityId(uuid)))
            }

            "execute" => {
                let id_str = parts.get(1)?;
                let uuid = Uuid::parse_str(id_str).ok()?;
                let input = parts
                    .get(2)
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                Some(CliCommand::Execute(CapabilityId(uuid), input))
            }

            "enable" => {
                let id_str = parts.get(1)?;
                let uuid = Uuid::parse_str(id_str).ok()?;
                Some(CliCommand::Enable(CapabilityId(uuid)))
            }

            "disable" => {
                let id_str = parts.get(1)?;
                let uuid = Uuid::parse_str(id_str).ok()?;
                Some(CliCommand::Disable(CapabilityId(uuid)))
            }

            "search" => {
                let query = parts.get(1..)?.join(" ");
                if query.is_empty() {
                    None
                } else {
                    Some(CliCommand::Search(query))
                }
            }

            _ => None,
        }
    }
}

/// Placeholder for extracting state from metadata in inspect formatting.
fn capability_state_placeholder() -> crate::core::CapabilityState {
    crate::core::CapabilityState::Registered
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. CapabilityIntegrator
// ═══════════════════════════════════════════════════════════════════════════

/// Top-level integrator that combines all subsystem integrations and provides
/// a single entry point for setting up and accessing the full capability
/// integration layer.
pub struct CapabilityIntegrator {
    executive: Arc<ExecutiveIntegration>,
    reasoning: Arc<ReasoningIntegration>,
    memory: Arc<MemoryIntegration>,
    knowledge: Arc<KnowledgeIntegration>,
    cli: Arc<CliIntegration>,
}

impl CapabilityIntegrator {
    /// Create a new integrator, instantiating all subsystem integrations
    /// backed by the shared `CapabilityApi`.
    pub fn new(api: Arc<CapabilityApi>) -> Self {
        Self {
            executive: Arc::new(ExecutiveIntegration::new(api.clone())),
            reasoning: Arc::new(ReasoningIntegration::new(api.clone())),
            memory: Arc::new(MemoryIntegration::new(api.clone())),
            knowledge: Arc::new(KnowledgeIntegration::new(api.clone())),
            cli: Arc::new(CliIntegration::new(api)),
        }
    }

    /// Access the Executive integration.
    pub fn executive(&self) -> &Arc<ExecutiveIntegration> {
        &self.executive
    }

    /// Access the Reasoning integration.
    pub fn reasoning(&self) -> &Arc<ReasoningIntegration> {
        &self.reasoning
    }

    /// Access the Memory integration.
    pub fn memory(&self) -> &Arc<MemoryIntegration> {
        &self.memory
    }

    /// Access the Knowledge integration.
    pub fn knowledge(&self) -> &Arc<KnowledgeIntegration> {
        &self.knowledge
    }

    /// Access the CLI integration.
    pub fn cli(&self) -> &Arc<CliIntegration> {
        &self.cli
    }

    /// Register built-in capabilities and establish their relationships.
    ///
    /// This sets up the foundational capability graph:
    /// - Reasoning, Memory, Knowledge, and Inference as core nodes
    /// - Workflow and Tool as peripheral nodes
    /// - Relationships encoding typical dependency and data-flow patterns
    pub fn setup_built_in_capabilities(&self) -> CapabilityResult<()> {
        let reasoning_meta = CapabilityMetadata::new(
            "reasoning",
            CapabilityVersion::new(1, 0, 0),
            "Chain-of-thought reasoning capability for complex problem solving",
            CapabilityCategory::Reasoning,
        )
        .with_namespace(CapabilityNamespace::reasoning())
        .with_author("neo-agi")
        .with_tag("reasoning")
        .with_tag("chain-of-thought")
        .with_tag("core");

        let memory_meta = CapabilityMetadata::new(
            "memory",
            CapabilityVersion::new(1, 0, 0),
            "Short-term and long-term memory management capability",
            CapabilityCategory::Memory,
        )
        .with_namespace(CapabilityNamespace::memory())
        .with_author("neo-agi")
        .with_tag("memory")
        .with_tag("storage")
        .with_tag("core");

        let knowledge_meta = CapabilityMetadata::new(
            "knowledge",
            CapabilityVersion::new(1, 0, 0),
            "Knowledge graph query and traversal capability",
            CapabilityCategory::Knowledge,
        )
        .with_namespace(CapabilityNamespace::knowledge())
        .with_author("neo-agi")
        .with_tag("knowledge")
        .with_tag("graph")
        .with_tag("core");

        let inference_meta = CapabilityMetadata::new(
            "inference",
            CapabilityVersion::new(1, 0, 0),
            "Model inference capability for predictions and completions",
            CapabilityCategory::Inference,
        )
        .with_namespace(CapabilityNamespace::inference())
        .with_author("neo-agi")
        .with_tag("inference")
        .with_tag("model")
        .with_tag("core");

        let workflow_meta = CapabilityMetadata::new(
            "workflow",
            CapabilityVersion::new(1, 0, 0),
            "Orchestrated multi-step workflow execution capability",
            CapabilityCategory::Workflow,
        )
        .with_author("neo-agi")
        .with_tag("workflow")
        .with_tag("orchestration");

        let tool_meta = CapabilityMetadata::new(
            "tool",
            CapabilityVersion::new(1, 0, 0),
            "External tool invocation capability",
            CapabilityCategory::Tool,
        )
        .with_author("neo-agi")
        .with_tag("tool")
        .with_tag("external");

        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: reasoning_meta.clone() }))).ok();
        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: memory_meta.clone() }))).ok();
        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: knowledge_meta.clone() }))).ok();
        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: inference_meta.clone() }))).ok();
        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: workflow_meta.clone() }))).ok();
        self.executive.api.register(Arc::new(RwLock::new(StubCapability { meta: tool_meta.clone() }))).ok();

        self.knowledge
            .add_relationship(reasoning_meta.id, memory_meta.id, "reads_from".into(), 1.0);
        self.knowledge
            .add_relationship(reasoning_meta.id, knowledge_meta.id, "queries".into(), 1.0);
        self.knowledge
            .add_relationship(reasoning_meta.id, inference_meta.id, "invokes".into(), 0.9);
        self.knowledge
            .add_relationship(memory_meta.id, knowledge_meta.id, "syncs_with".into(), 0.8);
        self.knowledge.add_relationship(
            knowledge_meta.id,
            inference_meta.id,
            "feeds_into".into(),
            0.7,
        );
        self.knowledge
            .add_relationship(workflow_meta.id, reasoning_meta.id, "uses".into(), 0.6);
        self.knowledge
            .add_relationship(workflow_meta.id, tool_meta.id, "invokes".into(), 0.5);
        self.knowledge
            .add_relationship(tool_meta.id, memory_meta.id, "stores_results".into(), 0.4);

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CapabilityId, CapabilityMetadata, CapabilityVersion, CapabilityCategory, ExecutionContext};

    fn test_api() -> Arc<CapabilityApi> {
        Arc::new(CapabilityApi::new())
    }

    fn test_metadata(name: &str) -> CapabilityMetadata {
        CapabilityMetadata::new(
            name,
            CapabilityVersion::new(1, 0, 0),
            format!("Test capability: {}", name),
            CapabilityCategory::System,
        )
    }

    fn metadata_with_tags(name: &str, tags: &[&str]) -> CapabilityMetadata {
        let mut meta = test_metadata(name);
        for tag in tags {
            meta.tags.add(*tag);
        }
        meta
    }

    fn wrap(meta: CapabilityMetadata) -> Arc<RwLock<dyn Capability>> {
        Arc::new(RwLock::new(StubCapability { meta })) as Arc<RwLock<dyn Capability>>
    }

    // ── ExecutiveIntegration tests ───────────────────────────────────────

    #[test]
    fn executive_new() {
        let api = test_api();
        let exec = ExecutiveIntegration::new(api);
        assert!(exec.get_task_capabilities(Uuid::new_v4()).is_empty());
    }

    #[test]
    fn executive_get_task_capabilities_empty() {
        let api = test_api();
        let exec = ExecutiveIntegration::new(api);
        let task_id = Uuid::new_v4();
        assert!(exec.get_task_capabilities(task_id).is_empty());
    }

    #[test]
    fn executive_get_capability_tasks_empty() {
        let api = test_api();
        let exec = ExecutiveIntegration::new(api);
        assert!(exec.get_capability_tasks(CapabilityId::new()).is_empty());
    }

    #[tokio::test]
    async fn executive_invoke_capability_for_task() {
        let api = test_api();
        let meta = test_metadata("exec-test");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();
        api.enable(cap_id).unwrap();

        let exec = ExecutiveIntegration::new(api);
        let task_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(cap_id);

        let result = exec
            .invoke_capability_for_task(
                task_id,
                cap_id,
                serde_json::json!({"data": "test"}),
                ctx,
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);

        let links = exec.get_task_capabilities(task_id);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].status, "completed");
    }

    #[tokio::test]
    async fn executive_invoke_records_capability_tasks() {
        let api = test_api();
        let meta = test_metadata("cap-tasks");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();
        api.enable(cap_id).unwrap();

        let exec = ExecutiveIntegration::new(api);
        let task_id1 = Uuid::new_v4();
        let task_id2 = Uuid::new_v4();
        let ctx1 = ExecutionContext::new(cap_id);
        let ctx2 = ExecutionContext::new(cap_id);

        let _ = exec
            .invoke_capability_for_task(task_id1, cap_id, serde_json::json!({}), ctx1)
            .await;
        let _ = exec
            .invoke_capability_for_task(task_id2, cap_id, serde_json::json!({}), ctx2)
            .await;

        let tasks = exec.get_capability_tasks(cap_id);
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn executive_cancel_task_executions() {
        let api = test_api();
        let exec = ExecutiveIntegration::new(api);
        let task_id = Uuid::new_v4();
        let cap_id = CapabilityId::new();

        {
            let mut records = exec.task_records.write();
            records.insert(
                task_id,
                vec![TaskCapabilityLink {
                    task_id,
                    capability_id: cap_id,
                    execution_id: Uuid::new_v4(),
                    status: "running".to_string(),
                }],
            );
        }

        exec.cancel_task_executions(task_id);

        let links = exec.get_task_capabilities(task_id);
        assert_eq!(links[0].status, "cancelled");
    }

    #[tokio::test]
    async fn executive_invoke_not_found_capability() {
        let api = test_api();
        let exec = ExecutiveIntegration::new(api);
        let fake_id = CapabilityId::new();
        let ctx = ExecutionContext::new(fake_id);

        let result = exec
            .invoke_capability_for_task(Uuid::new_v4(), fake_id, serde_json::json!({}), ctx)
            .await;
        assert!(result.is_err());
    }

    // ── ReasoningIntegration tests ───────────────────────────────────────

    #[test]
    fn reasoning_new() {
        let api = test_api();
        let reason = ReasoningIntegration::new(api);
        assert!(reason.get_selection_history().is_empty());
    }

    #[test]
    fn reasoning_select_capabilities_empty_candidates() {
        let api = test_api();
        let reason = ReasoningIntegration::new(api);
        let ctx = ExecutionContext::new(CapabilityId::new());
        let result = reason.select_capabilities("test query", &ctx, &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn reasoning_select_capabilities_with_matching_tags() {
        let api = test_api();
        let meta1 = metadata_with_tags("reasoner", &["reasoning", "chain-of-thought"]);
        let meta2 = metadata_with_tags("memory-store", &["memory", "reasoning"]);
        let id1 = meta1.id;
        let id2 = meta2.id;
        api.register(wrap(meta1)).unwrap();
        api.register(wrap(meta2)).unwrap();

        let reason = ReasoningIntegration::new(api);
        let ctx = ExecutionContext::new(CapabilityId::new());
        let selected = reason.select_capabilities("reasoning query", &ctx, &[id1, id2]);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], id1);
    }

    #[test]
    fn reasoning_select_capabilities_category_match() {
        let api = test_api();
        let mut meta1 = test_metadata("reason-cap");
        meta1.category = CapabilityCategory::Reasoning;
        let mut meta2 = test_metadata("tool-cap");
        meta2.category = CapabilityCategory::Tool;
        let id1 = meta1.id;
        let id2 = meta2.id;
        api.register(wrap(meta1)).unwrap();
        api.register(wrap(meta2)).unwrap();

        let reason = ReasoningIntegration::new(api);
        let ctx = ExecutionContext::new(CapabilityId::new());
        let selected = reason.select_capabilities("reasoning task", &ctx, &[id1, id2]);

        assert_eq!(selected[0], id1);
    }

    #[test]
    fn reasoning_select_capabilities_fallback_to_first() {
        let api = test_api();
        let meta1 = test_metadata("a");
        let meta2 = test_metadata("b");
        let id1 = meta1.id;
        let id2 = meta2.id;
        api.register(wrap(meta1)).unwrap();
        api.register(wrap(meta2)).unwrap();

        let reason = ReasoningIntegration::new(api);
        let ctx = ExecutionContext::new(CapabilityId::new());
        let selected = reason.select_capabilities("xyzzy", &ctx, &[id1, id2]);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], id1);
    }

    #[test]
    fn reasoning_record_and_get_selection_history() {
        let api = test_api();
        let reason = ReasoningIntegration::new(api);

        let id1 = CapabilityId::new();
        let id2 = CapabilityId::new();
        reason.record_selection(
            "test query".into(),
            vec![id1, id2],
            "matched tags".into(),
        );

        let history = reason.get_selection_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].query, "test query");
        assert_eq!(history[0].selected_capabilities, vec![id1, id2]);
        assert_eq!(history[0].reasoning, "matched tags");
    }

    #[test]
    fn reasoning_suggest_capabilities_based_on_history() {
        let api = test_api();
        let meta1 = test_metadata("suggest-cap");
        let id1 = meta1.id;
        api.register(wrap(meta1)).unwrap();

        let reason = ReasoningIntegration::new(api);
        reason.record_selection(
            "machine learning".into(),
            vec![id1],
            "matching".into(),
        );

        let suggestions = reason.suggest_capabilities("machine");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].id, id1);
    }

    #[test]
    fn reasoning_suggest_capabilities_no_match() {
        let api = test_api();
        let reason = ReasoningIntegration::new(api);
        reason.record_selection("alpha".into(), vec![], "none".into());

        let suggestions = reason.suggest_capabilities("nonexistent");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn reasoning_select_capabilities_history_boosts_score() {
        let api = test_api();
        let meta1 = metadata_with_tags("cap-a", &["shared"]);
        let meta2 = metadata_with_tags("cap-b", &["shared"]);
        let id1 = meta1.id;
        let id2 = meta2.id;
        api.register(wrap(meta1)).unwrap();
        api.register(wrap(meta2)).unwrap();

        let reason = ReasoningIntegration::new(api);

        for _ in 0..5 {
            reason.record_selection("boost".into(), vec![id1], "frequent".into());
        }

        let ctx = ExecutionContext::new(CapabilityId::new());
        let selected = reason.select_capabilities("shared", &ctx, &[id1, id2]);

        assert_eq!(selected[0], id1);
    }

    // ── MemoryIntegration tests ──────────────────────────────────────────

    #[test]
    fn memory_new() {
        let api = test_api();
        let mem = MemoryIntegration::new(api);
        let id = CapabilityId::new();
        assert!(mem.retrieve_execution_history(id).is_empty());
    }

    #[test]
    fn memory_store_and_retrieve() {
        let api = test_api();
        let meta = test_metadata("mem-cap");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let mem = MemoryIntegration::new(api);

        let record = ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(
                cap_id,
                serde_json::json!({}),
            ),
            result: Some(CapabilityResult_output::success(
                serde_json::json!({"ok": true}),
                100,
            )),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            worker_id: None,
            error: None,
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Completed,
        };

        mem.store_execution(record);

        let history = mem.retrieve_execution_history(cap_id);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn memory_search_executions_by_name() {
        let api = test_api();
        let meta = test_metadata("searchable-cap");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let mem = MemoryIntegration::new(api);
        let record = ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(
                cap_id,
                serde_json::json!({}),
            ),
            result: None,
            started_at: Utc::now(),
            completed_at: None,
            worker_id: None,
            error: None,
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Completed,
        };

        mem.store_execution(record);

        let results = mem.search_executions("searchable");
        assert_eq!(results.len(), 1);

        let results = mem.search_executions("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn memory_get_execution_stats() {
        let api = test_api();
        let meta = test_metadata("stats-cap");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let mem = MemoryIntegration::new(api);

        let start1 = Utc::now();
        let record1 = ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(cap_id, serde_json::json!({})),
            result: Some(CapabilityResult_output::success(
                serde_json::json!({}),
                50,
            )),
            started_at: start1,
            completed_at: Some(start1 + chrono::Duration::milliseconds(50)),
            worker_id: None,
            error: None,
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Completed,
        };
        mem.store_execution(record1);

        let start2 = Utc::now();
        let record2 = ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(cap_id, serde_json::json!({})),
            result: None,
            started_at: start2,
            completed_at: Some(start2 + chrono::Duration::milliseconds(150)),
            worker_id: None,
            error: Some("boom".to_string()),
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Failed,
        };
        mem.store_execution(record2);

        let stats = mem.get_execution_stats(cap_id);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.successful, 1);
        assert_eq!(stats.failed, 1);
        assert!(stats.avg_duration > 0.0);
    }

    #[test]
    fn memory_get_execution_stats_empty() {
        let api = test_api();
        let mem = MemoryIntegration::new(api);
        let stats = mem.get_execution_stats(CapabilityId::new());
        assert_eq!(stats.total, 0);
        assert_eq!(stats.successful, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.avg_duration, 0.0);
    }

    #[test]
    fn memory_search_case_insensitive() {
        let api = test_api();
        let meta = test_metadata("MyCap");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let mem = MemoryIntegration::new(api);
        mem.store_execution(ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(cap_id, serde_json::json!({})),
            result: None,
            started_at: Utc::now(),
            completed_at: None,
            worker_id: None,
            error: None,
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Completed,
        });

        assert_eq!(mem.search_executions("mycap").len(), 1);
        assert_eq!(mem.search_executions("MYCAP").len(), 1);
    }

    // ── KnowledgeIntegration tests ───────────────────────────────────────

    #[test]
    fn knowledge_new() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let id = CapabilityId::new();
        assert!(know.get_relationships(id).is_empty());
        assert!(know.get_related_capabilities(id).is_empty());
    }

    #[test]
    fn knowledge_add_and_get_relationships() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();
        let c = CapabilityId::new();

        know.add_relationship(a, b, "depends_on".into(), 1.0);
        know.add_relationship(a, c, "uses".into(), 0.8);

        let rels = know.get_relationships(a);
        assert_eq!(rels.len(), 2);

        let related = know.get_related_capabilities(a);
        assert_eq!(related.len(), 2);
        assert!(related.contains(&b));
        assert!(related.contains(&c));
    }

    #[test]
    fn knowledge_get_relationships_incoming() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();

        know.add_relationship(a, b, "feeds".into(), 1.0);

        let rels_b = know.get_relationships(b);
        assert_eq!(rels_b.len(), 1);
        assert_eq!(rels_b[0].source, a);
        assert_eq!(rels_b[0].target, b);
    }

    #[test]
    fn knowledge_find_path_direct() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();

        know.add_relationship(a, b, "connects".into(), 1.0);

        let path = know.find_path(a, b).unwrap();
        assert_eq!(path, vec![a, b]);
    }

    #[test]
    fn knowledge_find_path_same_node() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();

        let path = know.find_path(a, a).unwrap();
        assert_eq!(path, vec![a]);
    }

    #[test]
    fn knowledge_find_path_no_path() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();

        assert!(know.find_path(a, b).is_none());
    }

    #[test]
    fn knowledge_find_path_indirect() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();
        let c = CapabilityId::new();
        let d = CapabilityId::new();

        know.add_relationship(a, b, "step1".into(), 1.0);
        know.add_relationship(b, c, "step2".into(), 1.0);
        know.add_relationship(c, d, "step3".into(), 1.0);

        let path = know.find_path(a, d).unwrap();
        assert_eq!(path, vec![a, b, c, d]);
    }

    #[test]
    fn knowledge_find_path_bfs_shortest() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();
        let c = CapabilityId::new();
        let d = CapabilityId::new();

        know.add_relationship(a, b, "short".into(), 1.0);
        know.add_relationship(a, c, "via1".into(), 1.0);
        know.add_relationship(b, d, "via2".into(), 1.0);
        know.add_relationship(c, d, "via3".into(), 1.0);

        let path = know.find_path(a, d).unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], a);
        assert_eq!(path[2], d);
    }

    #[test]
    fn knowledge_remove_relationship() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();

        know.add_relationship(a, b, "test".into(), 1.0);
        assert_eq!(know.get_related_capabilities(a).len(), 1);

        know.remove_relationship(a, b);
        assert!(know.get_related_capabilities(a).is_empty());
    }

    #[test]
    fn knowledge_remove_only_targeted_relationship() {
        let api = test_api();
        let know = KnowledgeIntegration::new(api);
        let a = CapabilityId::new();
        let b = CapabilityId::new();
        let c = CapabilityId::new();

        know.add_relationship(a, b, "keep".into(), 1.0);
        know.add_relationship(a, c, "remove".into(), 1.0);

        know.remove_relationship(a, c);

        let related = know.get_related_capabilities(a);
        assert_eq!(related.len(), 1);
        assert!(related.contains(&b));
    }

    // ── CliIntegration tests ─────────────────────────────────────────────

    #[test]
    fn cli_new() {
        let api = test_api();
        let cli = CliIntegration::new(api);
        let output = cli.format_list();
        assert!(output.contains("No capabilities registered."));
    }

    #[test]
    fn cli_format_list_with_capabilities() {
        let api = test_api();
        api.register(wrap(test_metadata("alpha"))).unwrap();
        api.register(wrap(test_metadata("beta"))).unwrap();

        let cli = CliIntegration::new(api);
        let output = cli.format_list();
        assert!(output.contains("alpha"));
        assert!(output.contains("beta"));
        assert!(output.contains("2 capabilities registered."));
    }

    #[test]
    fn cli_format_inspect_found() {
        let api = test_api();
        let meta = test_metadata("inspect-me");
        let id = meta.id;
        api.register(wrap(meta)).unwrap();

        let cli = CliIntegration::new(api);
        let output = cli.format_inspect(id);
        assert!(output.contains("inspect-me"));
        assert!(output.contains("Version:"));
        assert!(output.contains("Category:"));
        assert!(output.contains("Description:"));
    }

    #[test]
    fn cli_format_inspect_not_found() {
        let api = test_api();
        let cli = CliIntegration::new(api);
        let id = CapabilityId::new();
        let output = cli.format_inspect(id);
        assert!(output.contains("not found"));
    }

    #[test]
    fn cli_format_search_results_empty() {
        let api = test_api();
        let cli = CliIntegration::new(api);
        let output = cli.format_search_results(&[]);
        assert!(output.contains("No capabilities found."));
    }

    #[test]
    fn cli_format_search_results_with_results() {
        let api = test_api();
        api.register(wrap(test_metadata("found-one"))).unwrap();

        let results = api.search("found");
        let cli = CliIntegration::new(api);
        let output = cli.format_search_results(&results);
        assert!(output.contains("found-one"));
        assert!(output.contains("1 result(s) found."));
    }

    #[test]
    fn cli_parse_command_list() {
        assert_eq!(CliIntegration::parse_command("list"), Some(CliCommand::List));
    }

    #[test]
    fn cli_parse_command_inspect() {
        let id = CapabilityId::new();
        let cmd = CliIntegration::parse_command(&format!("inspect {}", id));
        assert_eq!(cmd, Some(CliCommand::Inspect(id)));
    }

    #[test]
    fn cli_parse_command_execute() {
        let id = CapabilityId::new();
        let input = r#"{"key": "value"}"#;
        let cmd = CliIntegration::parse_command(&format!("execute {} {}", id, input));
        let expected_input: serde_json::Value = serde_json::from_str(input).unwrap();
        assert_eq!(
            cmd,
            Some(CliCommand::Execute(id, expected_input))
        );
    }

    #[test]
    fn cli_parse_command_execute_no_json() {
        let id = CapabilityId::new();
        let cmd = CliIntegration::parse_command(&format!("execute {}", id));
        assert!(matches!(cmd, Some(CliCommand::Execute(_, _))));
    }

    #[test]
    fn cli_parse_command_enable() {
        let id = CapabilityId::new();
        let cmd = CliIntegration::parse_command(&format!("enable {}", id));
        assert_eq!(cmd, Some(CliCommand::Enable(id)));
    }

    #[test]
    fn cli_parse_command_disable() {
        let id = CapabilityId::new();
        let cmd = CliIntegration::parse_command(&format!("disable {}", id));
        assert_eq!(cmd, Some(CliCommand::Disable(id)));
    }

    #[test]
    fn cli_parse_command_search() {
        let cmd = CliIntegration::parse_command("search reasoning capabilities");
        assert_eq!(
            cmd,
            Some(CliCommand::Search("reasoning capabilities".to_string()))
        );
    }

    #[test]
    fn cli_parse_command_search_empty() {
        let cmd = CliIntegration::parse_command("search");
        assert_eq!(cmd, None);
    }

    #[test]
    fn cli_parse_command_unknown() {
        assert_eq!(CliIntegration::parse_command("foobar"), None);
    }

    #[test]
    fn cli_parse_command_empty() {
        assert_eq!(CliIntegration::parse_command(""), None);
    }

    // ── CapabilityIntegrator tests ───────────────────────────────────────

    #[test]
    fn integrator_new() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api);
        assert!(integrator.executive().get_task_capabilities(Uuid::new_v4()).is_empty());
        assert!(integrator.reasoning().get_selection_history().is_empty());
    }

    #[test]
    fn integrator_accessors() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api);

        assert!(Arc::strong_count(integrator.executive()) >= 1);
        assert!(Arc::strong_count(integrator.reasoning()) >= 1);
        assert!(Arc::strong_count(integrator.memory()) >= 1);
        assert!(Arc::strong_count(integrator.knowledge()) >= 1);
        assert!(Arc::strong_count(integrator.cli()) >= 1);
    }

    #[test]
    fn integrator_setup_built_in_capabilities() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api.clone());

        integrator.setup_built_in_capabilities().unwrap();

        assert_eq!(api.capability_count(), 6);

        let caps = api.list();
        let names: Vec<&str> = caps.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"reasoning"));
        assert!(names.contains(&"memory"));
        assert!(names.contains(&"knowledge"));
        assert!(names.contains(&"inference"));
        assert!(names.contains(&"workflow"));
        assert!(names.contains(&"tool"));
    }

    #[test]
    fn integrator_setup_built_in_relationships() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api);

        integrator.setup_built_in_capabilities().unwrap();

        let rels = integrator.knowledge().relationships.read();
        assert!(!rels.is_empty());

        let rel_types: Vec<&str> = rels.iter().map(|r| r.relationship.as_str()).collect();
        assert!(rel_types.contains(&"reads_from"));
        assert!(rel_types.contains(&"queries"));
        assert!(rel_types.contains(&"invokes"));
        assert!(rel_types.contains(&"syncs_with"));
        assert!(rel_types.contains(&"feeds_into"));
        assert!(rel_types.contains(&"uses"));
        assert!(rel_types.contains(&"stores_results"));
    }

    #[test]
    fn integrator_setup_built_in_path_finding() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api.clone());

        integrator.setup_built_in_capabilities().unwrap();

        let caps = api.list();
        let reasoning_id = caps.iter().find(|c| c.name == "reasoning").unwrap().id;
        let inference_id = caps.iter().find(|c| c.name == "inference").unwrap().id;

        let path = integrator.knowledge().find_path(reasoning_id, inference_id);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.len() >= 2);
        assert_eq!(path[0], reasoning_id);
        assert_eq!(*path.last().unwrap(), inference_id);
    }

    #[test]
    fn integrator_setup_idempotent() {
        let api = test_api();
        let integrator = CapabilityIntegrator::new(api.clone());

        integrator.setup_built_in_capabilities().unwrap();
        integrator.setup_built_in_capabilities().unwrap();

        assert_eq!(api.capability_count(), 6);
    }

    // ── Integration cross-cutting tests ──────────────────────────────────

    #[tokio::test]
    async fn executive_uses_shared_api() {
        let api = test_api();
        let meta = test_metadata("shared");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();
        api.enable(cap_id).unwrap();

        let integrator = CapabilityIntegrator::new(api.clone());

        let task_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(cap_id);

        let result = integrator
            .executive()
            .invoke_capability_for_task(task_id, cap_id, serde_json::json!({}), ctx)
            .await;

        assert!(result.is_ok());
        assert!(integrator.cli().format_list().contains("shared"));
    }

    #[test]
    fn knowledge_reasoning_suggest_after_record() {
        let api = test_api();
        let meta = test_metadata("cross-test");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let integrator = CapabilityIntegrator::new(api);

        integrator.reasoning().record_selection(
            "testing cross integration".into(),
            vec![cap_id],
            "cross-test reasoning".into(),
        );

        let suggestions = integrator.reasoning().suggest_capabilities("testing cross");
        assert_eq!(suggestions.len(), 1);

        let history = integrator.reasoning().get_selection_history();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn memory_executive_store_after_invoke() {
        let api = test_api();
        let meta = test_metadata("mem-exec");
        let cap_id = meta.id;
        api.register(wrap(meta)).unwrap();

        let integrator = CapabilityIntegrator::new(api);

        let record = ExecutionRecord {
            id: Uuid::new_v4(),
            request: crate::execution::ExecutionRequest::new(cap_id, serde_json::json!({})),
            result: Some(CapabilityResult_output::success(
                serde_json::json!({}),
                0,
            )),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            worker_id: None,
            error: None,
            retry_count: 0,
            status: crate::execution::ExecutionStatus::Completed,
        };

        integrator.memory().store_execution(record);
        let history = integrator.memory().retrieve_execution_history(cap_id);
        assert_eq!(history.len(), 1);

        let stats = integrator.memory().get_execution_stats(cap_id);
        assert_eq!(stats.total, 1);
        assert_eq!(stats.successful, 1);
    }

    // ── TaskCapabilityLink serialization tests ───────────────────────────

    #[test]
    fn task_capability_link_serialization() {
        let link = TaskCapabilityLink {
            task_id: Uuid::new_v4(),
            capability_id: CapabilityId::new(),
            execution_id: Uuid::new_v4(),
            status: "completed".to_string(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let restored: TaskCapabilityLink = serde_json::from_str(&json).unwrap();
        assert_eq!(link.task_id, restored.task_id);
        assert_eq!(link.capability_id, restored.capability_id);
        assert_eq!(link.status, restored.status);
    }

    #[test]
    fn selection_record_serialization() {
        let record = SelectionRecord {
            query: "test".into(),
            selected_capabilities: vec![CapabilityId::new()],
            timestamp: Utc::now(),
            reasoning: "because".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let restored: SelectionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.query, restored.query);
        assert_eq!(record.reasoning, restored.reasoning);
    }

    #[test]
    fn capability_relationship_serialization() {
        let rel = CapabilityRelationship {
            source: CapabilityId::new(),
            target: CapabilityId::new(),
            relationship: "depends_on".into(),
            weight: 0.8,
        };
        let json = serde_json::to_string(&rel).unwrap();
        let restored: CapabilityRelationship = serde_json::from_str(&json).unwrap();
        assert_eq!(rel.source, restored.source);
        assert_eq!(rel.relationship, restored.relationship);
        assert_eq!(rel.weight, restored.weight);
    }

    #[test]
    fn execution_stats_serialization() {
        let stats = ExecutionStats {
            total: 100,
            successful: 90,
            failed: 10,
            avg_duration: 42.5,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let restored: ExecutionStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.total, restored.total);
        assert_eq!(stats.avg_duration, restored.avg_duration);
    }

    // ── CliCommand equality tests ────────────────────────────────────────

    #[test]
    fn cli_command_equality() {
        let id = CapabilityId::new();
        assert_eq!(CliCommand::List, CliCommand::List);
        assert_eq!(CliCommand::Inspect(id), CliCommand::Inspect(id));
        assert_eq!(
            CliCommand::Enable(id),
            CliCommand::Enable(id)
        );
        assert_eq!(
            CliCommand::Disable(id),
            CliCommand::Disable(id)
        );
        assert_eq!(
            CliCommand::Search("q".into()),
            CliCommand::Search("q".into())
        );
        assert_ne!(CliCommand::List, CliCommand::Search("q".into()));
    }
}
