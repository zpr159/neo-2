use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::{
    Capability, CapabilityCategory, CapabilityId, CapabilityMetadata, CapabilityVersion,
    CancellationToken, ExecutionContext, ResourceRequirements,
};
use crate::error::{CapabilityError, CapabilityResult};
use crate::execution::{ExecutionPipeline, TimeoutConfig};

/// Strategy for composing multiple capabilities together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositionStrategy {
    Sequential,
    Parallel,
    Conditional,
    Fallback,
}

impl std::fmt::Display for CompositionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequential => write!(f, "sequential"),
            Self::Parallel => write!(f, "parallel"),
            Self::Conditional => write!(f, "conditional"),
            Self::Fallback => write!(f, "fallback"),
        }
    }
}

/// What to do when a composition step fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureAction {
    Abort,
    Continue,
    Fallback(CapabilityId),
    Retry(u32),
}

/// A single step inside a composition template or composed capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionStep {
    pub name: String,
    pub capability_id: CapabilityId,
    pub input_template: serde_json::Value,
    pub condition: Option<String>,
    pub on_failure: FailureAction,
}

/// A reusable template describing how to compose several capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionTemplate {
    pub name: String,
    pub description: String,
    pub strategy: CompositionStrategy,
    pub steps: Vec<CompositionStep>,
    pub timeout_ms: Option<u64>,
}

impl CompositionTemplate {
    /// Create a new template with the given strategy.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        strategy: CompositionStrategy,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            strategy,
            steps: Vec::new(),
            timeout_ms: None,
        }
    }

    /// Append a step to this template.
    pub fn add_step(&mut self, step: CompositionStep) {
        self.steps.push(step);
    }

    /// Validate the template's internal consistency.
    ///
    /// Checks:
    /// - name is non-empty
    /// - at least one step exists
    /// - every step has a non-empty name
    pub fn validate(&self) -> CapabilityResult<()> {
        if self.name.is_empty() {
            return Err(CapabilityError::validation_failed(
                "template name cannot be empty",
            ));
        }
        if self.steps.is_empty() {
            return Err(CapabilityError::validation_failed(
                "template must have at least one step",
            ));
        }
        for (i, step) in self.steps.iter().enumerate() {
            if step.name.is_empty() {
                return Err(CapabilityError::validation_failed(format!(
                    "step {} has an empty name",
                    i
                )));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ComposedCapability
// ---------------------------------------------------------------------------

/// A capability built by composing several other capabilities according to a
/// `CompositionStrategy`.
pub struct ComposedCapability {
    metadata: CapabilityMetadata,
    strategy: CompositionStrategy,
    steps: RwLock<Vec<CompositionStep>>,
    capability_store: RwLock<HashMap<CapabilityId, Arc<dyn Capability>>>,
    timeout_ms: u64,
}

impl ComposedCapability {
    /// Create a new empty composed capability with the given strategy.
    pub fn new(name: impl Into<String>, strategy: CompositionStrategy) -> Self {
        let name_str = name.into();
        let metadata = CapabilityMetadata::new(
            &name_str,
            CapabilityVersion::initial(),
            format!("Composed capability using {} strategy", strategy),
            CapabilityCategory::Workflow,
        );
        Self {
            metadata,
            strategy,
            steps: RwLock::new(Vec::new()),
            capability_store: RwLock::new(HashMap::new()),
            timeout_ms: 60_000,
        }
    }

    /// Set the composition-level timeout in milliseconds.
    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self.metadata.timeout_ms = Some(timeout_ms);
        self
    }

    /// Add a step and register the capability that implements it.
    pub fn add_step(&self, step: CompositionStep, capability: Arc<dyn Capability>) {
        self.capability_store
            .write()
            .insert(step.capability_id, capability);
        self.steps.write().push(step);
    }

    /// Remove a step by index (0-based). Returns the removed step if the index
    /// was valid.
    pub fn remove_step(&self, index: usize) -> Option<CompositionStep> {
        let mut steps = self.steps.write();
        if index >= steps.len() {
            return None;
        }
        let removed = steps.remove(index);
        self.capability_store.write().remove(&removed.capability_id);
        Some(removed)
    }

    /// Number of steps currently in the composition.
    pub fn get_step_count(&self) -> usize {
        self.steps.read().len()
    }

    /// Verify that every step's `capability_id` is present in the internal
    /// capability store.
    pub fn validate_composition(&self) -> CapabilityResult<()> {
        let steps = self.steps.read();
        let store = self.capability_store.read();
        for step in steps.iter() {
            if !store.contains_key(&step.capability_id) {
                return Err(CapabilityError::dependency_missing(format!(
                    "step '{}' references capability {} which is not registered",
                    step.name, step.capability_id
                )));
            }
        }
        Ok(())
    }

    // -- private execution helpers ------------------------------------------

    fn get_capability(
        &self,
        id: &CapabilityId,
    ) -> CapabilityResult<Arc<dyn Capability>> {
        self.capability_store
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| {
                CapabilityError::not_found(format!("capability {} not in store", id))
            })
    }

    fn evaluate_condition(_condition: &str, _context: &ExecutionContext) -> bool {
        // A real implementation would parse and evaluate the condition string
        // against the execution context. For now a non-empty condition string
        // means "run this step".
        !_condition.is_empty()
    }

    async fn execute_sequential(
        &self,
        input: serde_json::Value,
        context: &ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        let steps = self.steps.read().clone();
        let total = steps.len() as u32;
        let mut current_input = input;
        let mut last_output = None;
        let mut total_duration: u64 = 0;

        for (i, step) in steps.iter().enumerate() {
            if context.is_cancelled() {
                return Err(CapabilityError::cancelled("composition cancelled"));
            }

            context.report_progress(crate::core::ProgressUpdate::new(
                i as u32,
                total,
                format!("step {}: {}", i, step.name),
            ));

            let cap = self.get_capability(&step.capability_id)?;

            let step_input = current_input.clone();
            let result = {
                let ctx = ExecutionContext {
                    execution_id: Uuid::new_v4(),
                    capability_id: step.capability_id,
                    permissions: context.permissions.clone(),
                    environment: context.environment.clone(),
                    timeout_ms: Some(self.timeout_ms),
                    cancel_token: context.cancel_token.clone(),
                    progress_callback: context.progress_callback.clone(),
                };
                tokio::time::timeout(
                    std::time::Duration::from_millis(self.timeout_ms),
                    cap.execute(step_input, ctx),
                )
                .await
            };

            match result {
                Ok(Ok(output)) => {
                    total_duration += output.duration_ms;
                    if !output.success {
                        match &step.on_failure {
                            FailureAction::Abort => {
                                return Err(CapabilityError::execution_failed(
                                    output
                                        .error
                                        .unwrap_or_else(|| "step failed".to_string()),
                                ));
                            }
                            FailureAction::Continue => {
                                current_input = output.output.clone();
                                last_output = Some(output);
                                continue;
                            }
                            FailureAction::Retry(max) => {
                                let mut succeeded = false;
                                let mut last_err = output
                                    .error
                                    .unwrap_or_else(|| "step failed".to_string());
                                for attempt in 1..=*max {
                                    let retry_cap = self.get_capability(&step.capability_id)?;
                                    let retry_input = current_input.clone();
                                    let retry_result = {
                                        let ctx = ExecutionContext {
                                            execution_id: Uuid::new_v4(),
                                            capability_id: step.capability_id,
                                            permissions: context.permissions.clone(),
                                            environment: context.environment.clone(),
                                            timeout_ms: Some(self.timeout_ms),
                                            cancel_token: context.cancel_token.clone(),
                                            progress_callback: context.progress_callback
                                                .clone(),
                                        };
                                        tokio::time::timeout(
                                            std::time::Duration::from_millis(self.timeout_ms),
                                            retry_cap.execute(retry_input, ctx),
                                        )
                                        .await
                                    };

                                    match retry_result {
                                        Ok(Ok(out)) => {
                                            total_duration += out.duration_ms;
                                            if out.success {
                                                current_input = out.output.clone();
                                                last_output = Some(out);
                                                succeeded = true;
                                                break;
                                            }
                                            last_err =
                                                out.error.unwrap_or_else(|| "step failed".into());
                                        }
                                        Ok(Err(e)) => {
                                            last_err = e.to_string();
                                        }
                                        Err(_) => {
                                            last_err = format!(
                                                "step '{}' timed out on retry {}",
                                                step.name, attempt
                                            );
                                        }
                                    }
                                }
                                if !succeeded {
                                    return Err(CapabilityError::execution_failed(format!(
                                        "step '{}' failed after {} retries: {}",
                                        step.name, max, last_err
                                    )));
                                }
                            }
                            FailureAction::Fallback(fallback_id) => {
                                let fb_cap = self.get_capability(fallback_id)?;
                                let fb_input = current_input.clone();
                                let fb_result = {
                                    let ctx = ExecutionContext {
                                        execution_id: Uuid::new_v4(),
                                        capability_id: *fallback_id,
                                        permissions: context.permissions.clone(),
                                        environment: context.environment.clone(),
                                        timeout_ms: Some(self.timeout_ms),
                                        cancel_token: context.cancel_token.clone(),
                                        progress_callback: context.progress_callback.clone(),
                                    };
                                    tokio::time::timeout(
                                        std::time::Duration::from_millis(self.timeout_ms),
                                        fb_cap.execute(fb_input, ctx),
                                    )
                                    .await
                                };

                                match fb_result {
                                    Ok(Ok(out)) => {
                                        total_duration += out.duration_ms;
                                        if !out.success {
                                            return Err(CapabilityError::execution_failed(
                                                out.error.unwrap_or_else(|| {
                                                    "fallback step failed".to_string()
                                                }),
                                            ));
                                        }
                                        current_input = out.output.clone();
                                        last_output = Some(out);
                                    }
                                    Ok(Err(e)) => {
                                        return Err(e);
                                    }
                                    Err(_) => {
                                        return Err(CapabilityError::timeout(format!(
                                            "fallback step for '{}' timed out",
                                            step.name
                                        )));
                                    }
                                }
                            }
                        }
                    } else {
                        current_input = output.output.clone();
                        last_output = Some(output);
                    }
                }
                Ok(Err(e)) => {
                    match &step.on_failure {
                        FailureAction::Abort => {
                            return Err(e);
                        }
                        FailureAction::Retry(max) => {
                            let mut succeeded = false;
                            let mut last_err = e.to_string();
                            for attempt in 1..=*max {
                                let retry_cap = self.get_capability(&step.capability_id)?;
                                let retry_input = current_input.clone();
                                let retry_result = {
                                    let ctx = ExecutionContext {
                                        execution_id: Uuid::new_v4(),
                                        capability_id: step.capability_id,
                                        permissions: context.permissions.clone(),
                                        environment: context.environment.clone(),
                                        timeout_ms: Some(self.timeout_ms),
                                        cancel_token: context.cancel_token.clone(),
                                        progress_callback: context.progress_callback.clone(),
                                    };
                                    tokio::time::timeout(
                                        std::time::Duration::from_millis(self.timeout_ms),
                                        retry_cap.execute(retry_input, ctx),
                                    )
                                    .await
                                };

                                match retry_result {
                                    Ok(Ok(out)) => {
                                        total_duration += out.duration_ms;
                                        if out.success {
                                            current_input = out.output.clone();
                                            last_output = Some(out);
                                            succeeded = true;
                                            break;
                                        }
                                        last_err =
                                            out.error.unwrap_or_else(|| "step failed".into());
                                    }
                                    Ok(Err(re)) => {
                                        last_err = re.to_string();
                                    }
                                    Err(_) => {
                                        last_err = format!(
                                            "step '{}' timed out on retry {}",
                                            step.name, attempt
                                        );
                                    }
                                }
                            }
                            if !succeeded {
                                return Err(CapabilityError::execution_failed(format!(
                                    "step '{}' failed after {} retries: {}",
                                    step.name, max, last_err
                                )));
                            }
                        }
                        FailureAction::Continue => {
                            last_output = Some(crate::core::CapabilityResult_output::failure(
                                e.to_string(),
                                0,
                            ));
                            continue;
                        }
                        FailureAction::Fallback(fallback_id) => {
                            let fb_cap = self.get_capability(fallback_id)?;
                            let fb_result = fb_cap
                                .execute(
                                    current_input.clone(),
                                    ExecutionContext {
                                        execution_id: Uuid::new_v4(),
                                        capability_id: *fallback_id,
                                        permissions: context.permissions.clone(),
                                        environment: context.environment.clone(),
                                        timeout_ms: Some(self.timeout_ms),
                                        cancel_token: context.cancel_token.clone(),
                                        progress_callback: context.progress_callback.clone(),
                                    },
                                )
                                .await;

                            match fb_result {
                                Ok(out) => {
                                    total_duration += out.duration_ms;
                                    if !out.success {
                                        return Err(CapabilityError::execution_failed(
                                            out.error.unwrap_or_else(|| {
                                                "fallback step failed".to_string()
                                            }),
                                        ));
                                    }
                                    current_input = out.output.clone();
                                    last_output = Some(out);
                                }
                                Err(e2) => {
                                    return Err(e2);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    return Err(CapabilityError::timeout(format!(
                        "step '{}' timed out after {}ms",
                        step.name, self.timeout_ms
                    )));
                }
            }
        }

        Ok(last_output.unwrap_or_else(|| {
            crate::core::CapabilityResult_output::success(serde_json::Value::Null, total_duration)
        }))
    }

    async fn execute_parallel(
        &self,
        input: serde_json::Value,
        context: &ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        let steps = self.steps.read().clone();
        if steps.is_empty() {
            return Err(CapabilityError::composition_failed(
                "composition has no steps",
            ));
        }

        let total = steps.len() as u32;
        context.report_progress(crate::core::ProgressUpdate::new(0, total, "parallel execution"));

        let mut futures = Vec::with_capacity(steps.len());
        let mut step_names = Vec::with_capacity(steps.len());
        let mut step_indices = Vec::with_capacity(steps.len());

        for (i, step) in steps.iter().enumerate() {
            let cap = self.get_capability(&step.capability_id)?;
            let step_input = input.clone();
            let ctx = ExecutionContext {
                execution_id: Uuid::new_v4(),
                capability_id: step.capability_id,
                permissions: context.permissions.clone(),
                environment: context.environment.clone(),
                timeout_ms: Some(self.timeout_ms),
                cancel_token: context.cancel_token.clone(),
                progress_callback: context.progress_callback.clone(),
            };
            let timeout_ms = self.timeout_ms;

            let future = async move {
                tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    cap.execute(step_input, ctx),
                )
                .await
            };
            futures.push(future);
            step_names.push(step.name.clone());
            step_indices.push(i);
        }

        let results = futures::future::join_all(futures).await;

        let mut outputs: Vec<(
            usize,
            String,
            crate::core::CapabilityResult_output,
        )> = Vec::with_capacity(results.len());
        let mut errors: Vec<(usize, String, CapabilityError)> = Vec::new();

        for ((idx, name), result) in step_indices
            .iter()
            .zip(step_names.iter())
            .zip(results.into_iter())
        {
            match result {
                Ok(Ok(output)) => {
                    if !output.success {
                        errors.push((
                            *idx,
                            name.clone(),
                            CapabilityError::execution_failed(
                                output.error.clone().unwrap_or_else(|| "step failed".into()),
                            ),
                        ));
                    }
                    outputs.push((*idx, name.clone(), output));
                }
                Ok(Err(e)) => {
                    errors.push((*idx, name.clone(), e));
                }
                Err(_) => {
                    errors.push((
                        *idx,
                        name.clone(),
                        CapabilityError::timeout(format!(
                            "step '{}' timed out after {}ms",
                            name, self.timeout_ms
                        )),
                    ));
                }
            }
        }

        if !errors.is_empty() {
            let error_msgs: Vec<String> = errors
                .iter()
                .map(|(_, name, e)| format!("{}: {}", name, e))
                .collect();
            return Err(CapabilityError::composition_failed(format!(
                "parallel execution had {} failure(s): {}",
                errors.len(),
                error_msgs.join("; ")
            )));
        }

        outputs.sort_by_key(|(idx, _, _)| *idx);

        let combined_output: Vec<serde_json::Value> = outputs
            .into_iter()
            .map(|(_, name, out)| {
                serde_json::json!({
                    "step": name,
                    "output": out.output,
                })
            })
            .collect();

        let total_duration: u64 = 0;
        Ok(crate::core::CapabilityResult_output::success(
            serde_json::json!({ "results": combined_output }),
            total_duration,
        ))
    }

    async fn execute_conditional(
        &self,
        input: serde_json::Value,
        context: &ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        let steps = self.steps.read().clone();
        let total = steps.len() as u32;
        let mut current_input = input;
        let mut last_output = None;
        let mut total_duration: u64 = 0;

        for (i, step) in steps.iter().enumerate() {
            if context.is_cancelled() {
                return Err(CapabilityError::cancelled("composition cancelled"));
            }

            // Evaluate condition if one is present
            if let Some(ref cond) = step.condition {
                if !Self::evaluate_condition(cond, context) {
                    context.report_progress(crate::core::ProgressUpdate::new(
                        i as u32,
                        total,
                        format!("skipping step {}: {} (condition not met)", i, step.name),
                    ));
                    continue;
                }
            }

            context.report_progress(crate::core::ProgressUpdate::new(
                i as u32,
                total,
                format!("step {}: {}", i, step.name),
            ));

            let cap = self.get_capability(&step.capability_id)?;

            let step_input = current_input.clone();
            let result = {
                let ctx = ExecutionContext {
                    execution_id: Uuid::new_v4(),
                    capability_id: step.capability_id,
                    permissions: context.permissions.clone(),
                    environment: context.environment.clone(),
                    timeout_ms: Some(self.timeout_ms),
                    cancel_token: context.cancel_token.clone(),
                    progress_callback: context.progress_callback.clone(),
                };
                tokio::time::timeout(
                    std::time::Duration::from_millis(self.timeout_ms),
                    cap.execute(step_input, ctx),
                )
                .await
            };

            match result {
                Ok(Ok(output)) => {
                    total_duration += output.duration_ms;
                    if !output.success {
                        return Err(CapabilityError::execution_failed(
                            output.error.unwrap_or_else(|| "conditional step failed".into()),
                        ));
                    }
                    current_input = output.output.clone();
                    last_output = Some(output);
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(_) => {
                    return Err(CapabilityError::timeout(format!(
                        "conditional step '{}' timed out after {}ms",
                        step.name, self.timeout_ms
                    )));
                }
            }
        }

        Ok(last_output.unwrap_or_else(|| {
            crate::core::CapabilityResult_output::success(serde_json::Value::Null, total_duration)
        }))
    }

    async fn execute_fallback(
        &self,
        input: serde_json::Value,
        context: &ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        let steps = self.steps.read().clone();
        let total = steps.len() as u32;
        let mut last_error: Option<CapabilityError> = None;

        for (i, step) in steps.iter().enumerate() {
            if context.is_cancelled() {
                return Err(CapabilityError::cancelled("composition cancelled"));
            }

            context.report_progress(crate::core::ProgressUpdate::new(
                i as u32,
                total,
                format!("fallback attempt {}: {}", i, step.name),
            ));

            let cap = self.get_capability(&step.capability_id)?;

            let step_input = input.clone();
            let result = {
                let ctx = ExecutionContext {
                    execution_id: Uuid::new_v4(),
                    capability_id: step.capability_id,
                    permissions: context.permissions.clone(),
                    environment: context.environment.clone(),
                    timeout_ms: Some(self.timeout_ms),
                    cancel_token: context.cancel_token.clone(),
                    progress_callback: context.progress_callback.clone(),
                };
                tokio::time::timeout(
                    std::time::Duration::from_millis(self.timeout_ms),
                    cap.execute(step_input, ctx),
                )
                .await
            };

            match result {
                Ok(Ok(output)) => {
                    if output.success {
                        return Ok(output);
                    }
                    last_error = Some(CapabilityError::execution_failed(
                        output.error.unwrap_or_else(|| "step failed".into()),
                    ));
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(_) => {
                    last_error = Some(CapabilityError::timeout(format!(
                        "fallback step '{}' timed out after {}ms",
                        step.name, self.timeout_ms
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            CapabilityError::composition_failed("no fallback steps provided")
        }))
    }
}

#[async_trait]
impl Capability for ComposedCapability {
    fn metadata(&self) -> &CapabilityMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
        &mut self.metadata
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        context: ExecutionContext,
    ) -> CapabilityResult<crate::core::CapabilityResult_output> {
        self.validate_composition()?;

        match self.strategy {
            CompositionStrategy::Sequential => {
                self.execute_sequential(input, &context).await
            }
            CompositionStrategy::Parallel => self.execute_parallel(input, &context).await,
            CompositionStrategy::Conditional => {
                self.execute_conditional(input, &context).await
            }
            CompositionStrategy::Fallback => self.execute_fallback(input, &context).await,
        }
    }
}

// ---------------------------------------------------------------------------
// CompositionRegistry
// ---------------------------------------------------------------------------

/// Registry for storing, retrieving, and instantiating composition templates.
pub struct CompositionRegistry {
    templates: RwLock<HashMap<String, CompositionTemplate>>,
}

impl CompositionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Register a template. Overwrites any existing template with the same name.
    pub fn register_template(&self, template: CompositionTemplate) -> CapabilityResult<()> {
        template.validate()?;
        self.templates
            .write()
            .insert(template.name.clone(), template);
        Ok(())
    }

    /// Retrieve a clone of a template by name.
    pub fn get_template(&self, name: &str) -> Option<CompositionTemplate> {
        self.templates.read().get(name).cloned()
    }

    /// List all registered template names.
    pub fn list_templates(&self) -> Vec<String> {
        self.templates.read().keys().cloned().collect()
    }

    /// Remove a template by name. Returns `true` if it existed.
    pub fn remove_template(&self, name: &str) -> bool {
        self.templates.write().remove(name).is_some()
    }

    /// Number of registered templates.
    pub fn template_count(&self) -> usize {
        self.templates.read().len()
    }

    /// Create a new `ComposedCapability` from a registered template.
    ///
    /// The caller must separately register the step capabilities into the
    /// returned `ComposedCapability` via `add_step`.
    pub fn create_from_template(
        &self,
        name: &str,
    ) -> CapabilityResult<ComposedCapability> {
        let template = self
            .templates
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| CapabilityError::not_found(format!("template '{}' not found", name)))?;

        let mut composed = ComposedCapability::new(&template.name, template.strategy);
        if let Some(timeout) = template.timeout_ms {
            composed = composed.with_timeout_ms(timeout);
        }

        for step in &template.steps {
            composed.steps.write().push(step.clone());
        }

        Ok(composed)
    }
}

impl Default for CompositionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{CapabilityCategory, CapabilityMetadata, CapabilityNamespace, CapabilityVersion};

    // -- helpers ------------------------------------------------------------

    struct StubCapability {
        metadata: CapabilityMetadata,
        output_value: serde_json::Value,
        should_fail: bool,
        fail_message: String,
    }

    impl StubCapability {
        fn new(name: &str, output: serde_json::Value) -> Self {
            let metadata = CapabilityMetadata::new(
                name,
                CapabilityVersion::initial(),
                format!("stub: {}", name),
                CapabilityCategory::Tool,
            );
            Self {
                metadata,
                output_value: output,
                should_fail: false,
                fail_message: String::new(),
            }
        }

        fn failing(name: &str, msg: &str) -> Self {
            let metadata = CapabilityMetadata::new(
                name,
                CapabilityVersion::initial(),
                format!("failing stub: {}", name),
                CapabilityCategory::Tool,
            );
            Self {
                metadata,
                output_value: serde_json::Value::Null,
                should_fail: true,
                fail_message: msg.to_string(),
            }
        }

        fn wrap(self) -> Arc<dyn Capability> {
            Arc::new(self) as Arc<dyn Capability>
        }
    }

    #[async_trait]
    impl Capability for StubCapability {
        fn metadata(&self) -> &CapabilityMetadata {
            &self.metadata
        }

        fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
            &mut self.metadata
        }

        async fn execute(
            &self,
            input: serde_json::Value,
            _context: ExecutionContext,
        ) -> CapabilityResult<crate::core::CapabilityResult_output> {
            if self.should_fail {
                return Err(CapabilityError::execution_failed(&self.fail_message));
            }
            Ok(crate::core::CapabilityResult_output::success(
                self.output_value.clone(),
                1,
            ))
        }
    }

    fn make_context() -> ExecutionContext {
        ExecutionContext::new(CapabilityId::new())
    }

    // -- CompositionStrategy tests ------------------------------------------

    #[test]
    fn strategy_display() {
        assert_eq!(format!("{}", CompositionStrategy::Sequential), "sequential");
        assert_eq!(format!("{}", CompositionStrategy::Parallel), "parallel");
        assert_eq!(format!("{}", CompositionStrategy::Conditional), "conditional");
        assert_eq!(format!("{}", CompositionStrategy::Fallback), "fallback");
    }

    // -- FailureAction tests ------------------------------------------------

    #[test]
    fn failure_action_equality() {
        assert_eq!(FailureAction::Abort, FailureAction::Abort);
        assert_eq!(FailureAction::Continue, FailureAction::Continue);
        let id = CapabilityId::new();
        assert_eq!(
            FailureAction::Fallback(id),
            FailureAction::Fallback(id)
        );
        assert_eq!(FailureAction::Retry(3), FailureAction::Retry(3));
        assert_ne!(FailureAction::Abort, FailureAction::Continue);
    }

    // -- CompositionTemplate tests ------------------------------------------

    #[test]
    fn template_new() {
        let t = CompositionTemplate::new("t1", "desc", CompositionStrategy::Sequential);
        assert_eq!(t.name, "t1");
        assert_eq!(t.description, "desc");
        assert_eq!(t.strategy, CompositionStrategy::Sequential);
        assert!(t.steps.is_empty());
        assert!(t.timeout_ms.is_none());
    }

    #[test]
    fn template_add_step() {
        let mut t = CompositionTemplate::new("t", "d", CompositionStrategy::Parallel);
        t.add_step(CompositionStep {
            name: "s1".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert_eq!(t.steps.len(), 1);
    }

    #[test]
    fn template_validate_empty_name_fails() {
        let mut t = CompositionTemplate::new("", "d", CompositionStrategy::Sequential);
        t.add_step(CompositionStep {
            name: "s".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert!(t.validate().is_err());
    }

    #[test]
    fn template_validate_no_steps_fails() {
        let t = CompositionTemplate::new("t", "d", CompositionStrategy::Sequential);
        assert!(t.validate().is_err());
    }

    #[test]
    fn template_validate_step_empty_name_fails() {
        let mut t = CompositionTemplate::new("t", "d", CompositionStrategy::Sequential);
        t.add_step(CompositionStep {
            name: "".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert!(t.validate().is_err());
    }

    #[test]
    fn template_validate_ok() {
        let mut t = CompositionTemplate::new("t", "d", CompositionStrategy::Sequential);
        t.add_step(CompositionStep {
            name: "step1".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert!(t.validate().is_ok());
    }

    // -- ComposedCapability basic tests -------------------------------------

    #[test]
    fn composed_new() {
        let c = ComposedCapability::new("comp1", CompositionStrategy::Sequential);
        assert_eq!(c.metadata().name, "comp1");
        assert_eq!(c.strategy, CompositionStrategy::Sequential);
        assert_eq!(c.get_step_count(), 0);
    }

    #[test]
    fn composed_add_and_remove_step() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        let cap = StubCapability::new("s1", serde_json::json!({"v": 1})).wrap();
        let id = cap.metadata().id;

        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap,
        );
        assert_eq!(c.get_step_count(), 1);

        let removed = c.remove_step(0);
        assert!(removed.is_some());
        assert_eq!(c.get_step_count(), 0);
    }

    #[test]
    fn composed_remove_out_of_bounds() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        assert!(c.remove_step(0).is_none());
    }

    #[test]
    fn composed_validate_missing_capability() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        c.steps.write().push(CompositionStep {
            name: "orphan".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert!(c.validate_composition().is_err());
    }

    #[test]
    fn composed_validate_ok() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        let cap = StubCapability::new("s", serde_json::json!({})).wrap();
        let id = cap.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap,
        );
        assert!(c.validate_composition().is_ok());
    }

    // -- Sequential execution tests -----------------------------------------

    #[tokio::test]
    async fn sequential_single_step() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        let cap = StubCapability::new("s1", serde_json::json!({"result": "ok"})).wrap();
        let id = cap.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap,
        );

        let ctx = make_context();
        let result = c
            .execute(serde_json::json!({"x": 1}), ctx)
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["result"], "ok");
    }

    #[tokio::test]
    async fn sequential_chains_output() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);

        let cap1 = StubCapability::new("s1", serde_json::json!({"a": 1})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 = StubCapability::new("s2", serde_json::json!({"b": 2})).wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step2".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["b"], 2);
    }

    #[tokio::test]
    async fn sequential_abort_on_failure() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);

        let cap1 =
            StubCapability::failing("s1", "boom").wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn sequential_continue_on_failure() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);

        let cap1 =
            StubCapability::failing("s1", "oops").wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Continue,
            },
            cap1,
        );

        let cap2 = StubCapability::new("s2", serde_json::json!({"ok": true})).wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step2".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["ok"], true);
    }

    #[tokio::test]
    async fn sequential_fallback_on_failure() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);

        let primary = StubCapability::failing("primary", "fail").wrap();
        let primary_id = primary.metadata().id;

        let fallback = StubCapability::new("fallback", serde_json::json!({"via": "fallback"}))
            .wrap();
        let fallback_id = fallback.metadata().id;

        c.add_step(
            CompositionStep {
                name: "primary_step".into(),
                capability_id: primary_id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Fallback(fallback_id),
            },
            primary,
        );

        // Also register fallback capability in the store
        c.capability_store
            .write()
            .insert(fallback_id, fallback);

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["via"], "fallback");
    }

    #[tokio::test]
    async fn sequential_retry_on_failure() {
        use std::sync::atomic::{AtomicU32, Ordering};

        struct RetryCap {
            metadata: CapabilityMetadata,
            counter: std::sync::Arc<AtomicU32>,
        }

        impl RetryCap {
            fn new(name: &str, counter: std::sync::Arc<AtomicU32>) -> Self {
                let metadata = CapabilityMetadata::new(
                    name,
                    CapabilityVersion::initial(),
                    "retry stub",
                    CapabilityCategory::Tool,
                );
                Self { metadata, counter }
            }

            fn wrap(self) -> Arc<dyn Capability> {
                Arc::new(self) as Arc<dyn Capability>
            }
        }

        #[async_trait]
        impl Capability for RetryCap {
            fn metadata(&self) -> &CapabilityMetadata {
                &self.metadata
            }
            fn metadata_mut(&mut self) -> &mut CapabilityMetadata {
                &mut self.metadata
            }
            async fn execute(
                &self,
                _input: serde_json::Value,
                _context: ExecutionContext,
            ) -> CapabilityResult<crate::core::CapabilityResult_output> {
                let attempt = self.counter.fetch_add(1, Ordering::SeqCst);
                if attempt < 2 {
                    Err(CapabilityError::execution_failed(format!(
                        "fail on attempt {}",
                        attempt
                    )))
                } else {
                    Ok(crate::core::CapabilityResult_output::success(
                        serde_json::json!({"retried": true}),
                        1,
                    ))
                }
            }
        }

        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        let counter = std::sync::Arc::new(AtomicU32::new(0));
        let cap = RetryCap::new("retry_cap", counter).wrap();
        let id = cap.metadata().id;
        c.add_step(
            CompositionStep {
                name: "retry_step".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Retry(3),
            },
            cap,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["retried"], true);
    }

    // -- Parallel execution tests -------------------------------------------

    #[tokio::test]
    async fn parallel_all_succeed() {
        let c = ComposedCapability::new("c", CompositionStrategy::Parallel);

        let cap1 = StubCapability::new("s1", serde_json::json!({"a": 1})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 = StubCapability::new("s2", serde_json::json!({"b": 2})).wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step2".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        let results = result.output["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn parallel_one_fails() {
        let c = ComposedCapability::new("c", CompositionStrategy::Parallel);

        let cap1 = StubCapability::new("s1", serde_json::json!({"a": 1})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 =
            StubCapability::failing("s2", "parallel boom").wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step2".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn parallel_empty_fails() {
        let c = ComposedCapability::new("c", CompositionStrategy::Parallel);
        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    // -- Conditional execution tests ----------------------------------------

    #[tokio::test]
    async fn conditional_skips_when_condition_absent() {
        let c = ComposedCapability::new("c", CompositionStrategy::Conditional);

        // Step with a condition that will NOT be evaluated (None means always run)
        let cap1 = StubCapability::new("s1", serde_json::json!({"ran": true})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None, // no condition => always run
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["ran"], true);
    }

    #[tokio::test]
    async fn conditional_executes_when_condition_met() {
        let c = ComposedCapability::new("c", CompositionStrategy::Conditional);

        let cap1 = StubCapability::new("s1", serde_json::json!({"ran": true})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: Some("always_true".into()),
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn conditional_aborts_on_failure() {
        let c = ComposedCapability::new("c", CompositionStrategy::Conditional);

        let cap1 = StubCapability::failing("s1", "cond fail").wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    // -- Fallback execution tests -------------------------------------------

    #[tokio::test]
    async fn fallback_first_succeeds() {
        let c = ComposedCapability::new("c", CompositionStrategy::Fallback);

        let cap1 = StubCapability::new("primary", serde_json::json!({"winner": "primary"})).wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "primary".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 = StubCapability::new("secondary", serde_json::json!({"winner": "secondary"}))
            .wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "secondary".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["winner"], "primary");
    }

    #[tokio::test]
    async fn fallback_falls_to_second() {
        let c = ComposedCapability::new("c", CompositionStrategy::Fallback);

        let cap1 =
            StubCapability::failing("primary", "primary failed").wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "primary".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 = StubCapability::new("secondary", serde_json::json!({"winner": "secondary"}))
            .wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "secondary".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["winner"], "secondary");
    }

    #[tokio::test]
    async fn fallback_all_fail() {
        let c = ComposedCapability::new("c", CompositionStrategy::Fallback);

        let cap1 =
            StubCapability::failing("p1", "fail1").wrap();
        let id1 = cap1.metadata().id;
        c.add_step(
            CompositionStep {
                name: "p1".into(),
                capability_id: id1,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap1,
        );

        let cap2 =
            StubCapability::failing("p2", "fail2").wrap();
        let id2 = cap2.metadata().id;
        c.add_step(
            CompositionStep {
                name: "p2".into(),
                capability_id: id2,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap2,
        );

        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    #[tokio::test]
    async fn fallback_empty() {
        let c = ComposedCapability::new("c", CompositionStrategy::Fallback);
        let ctx = make_context();
        let err = c.execute(serde_json::json!({}), ctx).await;
        assert!(err.is_err());
    }

    // -- CompositionRegistry tests ------------------------------------------

    #[test]
    fn registry_new() {
        let reg = CompositionRegistry::new();
        assert_eq!(reg.template_count(), 0);
        assert!(reg.list_templates().is_empty());
    }

    #[test]
    fn registry_register_and_get() {
        let reg = CompositionRegistry::new();
        let mut t = CompositionTemplate::new("t1", "test", CompositionStrategy::Sequential);
        t.add_step(CompositionStep {
            name: "s".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        reg.register_template(t).unwrap();

        assert_eq!(reg.template_count(), 1);
        assert!(reg.get_template("t1").is_some());
        assert!(reg.get_template("missing").is_none());
        assert!(reg.list_templates().contains(&"t1".to_string()));
    }

    #[test]
    fn registry_register_invalid_template() {
        let reg = CompositionRegistry::new();
        let t = CompositionTemplate::new("", "bad", CompositionStrategy::Sequential);
        assert!(reg.register_template(t).is_err());
    }

    #[test]
    fn registry_remove() {
        let reg = CompositionRegistry::new();
        let mut t = CompositionTemplate::new("t1", "test", CompositionStrategy::Parallel);
        t.add_step(CompositionStep {
            name: "s".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        reg.register_template(t).unwrap();
        assert!(reg.remove_template("t1"));
        assert!(!reg.remove_template("t1"));
        assert_eq!(reg.template_count(), 0);
    }

    #[test]
    fn registry_create_from_template() {
        let reg = CompositionRegistry::new();
        let mut t = CompositionTemplate::new(
            "pipeline",
            "a pipeline",
            CompositionStrategy::Sequential,
        );
        t.timeout_ms = Some(5000);
        t.add_step(CompositionStep {
            name: "s1".into(),
            capability_id: CapabilityId::new(),
            input_template: serde_json::json!({"k": "v"}),
            condition: None,
            on_failure: FailureAction::Retry(2),
        });
        reg.register_template(t).unwrap();

        let composed = reg.create_from_template("pipeline").unwrap();
        assert_eq!(composed.metadata().name, "pipeline");
        assert_eq!(composed.strategy, CompositionStrategy::Sequential);
        assert_eq!(composed.timeout_ms, 5000);
        assert_eq!(composed.get_step_count(), 1);
    }

    #[test]
    fn registry_create_from_missing_template() {
        let reg = CompositionRegistry::new();
        assert!(reg.create_from_template("nope").is_err());
    }

    // -- Template serialization round-trip ----------------------------------

    #[test]
    fn template_serde_roundtrip() {
        let mut t = CompositionTemplate::new("rt", "round trip", CompositionStrategy::Fallback);
        t.timeout_ms = Some(3000);
        let id = CapabilityId::new();
        t.add_step(CompositionStep {
            name: "step_a".into(),
            capability_id: id,
            input_template: serde_json::json!({"data": [1, 2, 3]}),
            condition: Some("env.prod".into()),
            on_failure: FailureAction::Retry(5),
        });

        let json = serde_json::to_string(&t).unwrap();
        let deserialized: CompositionTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "rt");
        assert_eq!(deserialized.strategy, CompositionStrategy::Fallback);
        assert_eq!(deserialized.timeout_ms, Some(3000));
        assert_eq!(deserialized.steps.len(), 1);
        assert_eq!(deserialized.steps[0].name, "step_a");
        assert_eq!(deserialized.steps[0].capability_id, id);
        assert_eq!(
            deserialized.steps[0].condition,
            Some("env.prod".into())
        );
        assert_eq!(deserialized.steps[0].on_failure, FailureAction::Retry(5));
    }

    // -- Validate composition with all capabilities present -----------------

    #[tokio::test]
    async fn validate_then_execute() {
        let c = ComposedCapability::new("v", CompositionStrategy::Sequential);
        let cap = StubCapability::new("v1", serde_json::json!({"ok": true})).wrap();
        let id = cap.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap,
        );

        // validate_composition should succeed
        assert!(c.validate_composition().is_ok());

        // execution should also work
        let ctx = make_context();
        let result = c.execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
    }

    // -- Step with timeout -------------------------------------------------

    #[test]
    fn composed_with_timeout() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential).with_timeout_ms(5000);
        assert_eq!(c.timeout_ms, 5000);
        assert_eq!(c.metadata().timeout_ms, Some(5000));
    }

    // -- remove_step cleans up capability store ----------------------------

    #[test]
    fn remove_step_removes_from_store() {
        let c = ComposedCapability::new("c", CompositionStrategy::Sequential);
        let cap = StubCapability::new("s", serde_json::json!({})).wrap();
        let id = cap.metadata().id;
        c.add_step(
            CompositionStep {
                name: "step".into(),
                capability_id: id,
                input_template: serde_json::json!({}),
                condition: None,
                on_failure: FailureAction::Abort,
            },
            cap,
        );
        assert_eq!(c.capability_store.read().len(), 1);
        c.remove_step(0);
        assert_eq!(c.capability_store.read().len(), 0);
    }

    // -- Default impls -----------------------------------------------------

    #[test]
    fn composition_registry_default() {
        let reg = CompositionRegistry::default();
        assert_eq!(reg.template_count(), 0);
    }

    // -- Step name uniqueness not required but can coexist -----------------

    #[test]
    fn template_duplicate_step_names() {
        let mut t = CompositionTemplate::new("dup", "d", CompositionStrategy::Sequential);
        let id1 = CapabilityId::new();
        let id2 = CapabilityId::new();
        t.add_step(CompositionStep {
            name: "s".into(),
            capability_id: id1,
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        t.add_step(CompositionStep {
            name: "s".into(),
            capability_id: id2,
            input_template: serde_json::json!({}),
            condition: None,
            on_failure: FailureAction::Abort,
        });
        assert!(t.validate().is_ok());
    }

    // -- FailureAction variants round-trip ---------------------------------

    #[test]
    fn failure_action_serde_roundtrip() {
        let id = CapabilityId::new();
        let actions = vec![
            FailureAction::Abort,
            FailureAction::Continue,
            FailureAction::Fallback(id),
            FailureAction::Retry(7),
        ];
        for action in &actions {
            let json = serde_json::to_string(action).unwrap();
            let deserialized: FailureAction = serde_json::from_str(&json).unwrap();
            assert_eq!(*action, deserialized);
        }
    }
}
