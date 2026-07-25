use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::core::{
    Capability, CapabilityCategory, CapabilityId,
    CapabilityMetadata, CapabilityNamespace,
    CapabilityVersion, ExecutionContext, CapabilityResult_output, InputSchema, OutputSchema,
    ResourceRequirements,
};
use crate::error::{CapabilityError, CapabilityResult};

type ExecuteFn = Arc<
    dyn Fn(serde_json::Value, ExecutionContext) -> BoxFuture<'static, CapabilityResult<CapabilityResult_output>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// CapabilityBuilder
// ---------------------------------------------------------------------------

/// A builder for creating capabilities programmatically with a fluent API.
#[derive(Clone)]
pub struct CapabilityBuilder {
    metadata: CapabilityMetadata,
    execute_fn: Option<ExecuteFn>,
}

impl CapabilityBuilder {
    pub fn new(
        name: impl Into<String>,
        version: CapabilityVersion,
        description: impl Into<String>,
        category: CapabilityCategory,
    ) -> Self {
        Self {
            metadata: CapabilityMetadata::new(name, version, description, category),
            execute_fn: None,
        }
    }

    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.metadata.namespace = CapabilityNamespace::new(ns);
        self
    }

    pub fn tag(mut self, t: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_tag(t);
        self
    }

    pub fn alias(mut self, a: impl Into<String>) -> Self {
        self.metadata = self.metadata.with_alias(a);
        self
    }

    pub fn author(mut self, a: impl Into<String>) -> Self {
        self.metadata.author = a.into();
        self
    }

    pub fn license(mut self, l: impl Into<String>) -> Self {
        self.metadata.license = l.into();
        self
    }

    pub fn input(mut self, schema: serde_json::Value, description: impl Into<String>) -> Self {
        self.metadata.inputs.push(InputSchema::new(schema, description));
        self
    }

    pub fn output(mut self, schema: serde_json::Value, description: impl Into<String>) -> Self {
        self.metadata.output = OutputSchema::new(schema, description);
        self
    }

    pub fn dependency(mut self, cap_id: CapabilityId) -> Self {
        self.metadata.dependencies.push(cap_id);
        self
    }

    pub fn permission(mut self, perm: impl Into<String>) -> Self {
        self.metadata.required_permissions.push(perm.into());
        self
    }

    pub fn resources(mut self, cpu: f64, gpu: f64, memory: u64, inference: u32) -> Self {
        self.metadata.resource_requirements = ResourceRequirements {
            cpu_units: cpu,
            gpu_units: gpu,
            memory_bytes: memory,
            inference_tokens: inference,
            disk_bytes: 0,
        };
        self
    }

    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.metadata.timeout_ms = Some(ms);
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.metadata.max_retries = n;
        self
    }

    pub fn requires_approval(mut self) -> Self {
        self.metadata.requires_approval = true;
        self
    }

    pub fn on_execute<F>(mut self, f: F) -> Self
    where
        F: Fn(serde_json::Value, ExecutionContext) -> BoxFuture<'static, CapabilityResult<CapabilityResult_output>>
            + Send
            + Sync
            + 'static,
    {
        self.execute_fn = Some(Arc::new(f));
        self
    }

    pub fn build(self) -> CapabilityResult<Arc<RwLock<dyn Capability>>> {
        self.metadata.validate()?;
        let execute_fn = self.execute_fn.ok_or_else(|| {
            CapabilityError::validation_failed(
                "no execute handler registered; call on_execute() before build()",
            )
        })?;
        Ok(Arc::new(RwLock::new(DefaultCapability {
            metadata: self.metadata,
            execute_fn,
        })))
    }
}

// ---------------------------------------------------------------------------
// DefaultCapability
// ---------------------------------------------------------------------------

/// A capability implementation backed by a builder-provided execute function.
pub(crate) struct DefaultCapability {
    metadata: CapabilityMetadata,
    execute_fn: ExecuteFn,
}

#[async_trait]
impl Capability for DefaultCapability {
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
    ) -> CapabilityResult<CapabilityResult_output> {
        (self.execute_fn)(input, context).await
    }
}

// ---------------------------------------------------------------------------
// ForeignLanguage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ForeignLanguage {
    Python,
    Go,
    TypeScript,
    Ruby,
    Java,
}

impl fmt::Display for ForeignLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Python => write!(f, "python"),
            Self::Go => write!(f, "go"),
            Self::TypeScript => write!(f, "typescript"),
            Self::Ruby => write!(f, "ruby"),
            Self::Java => write!(f, "java"),
        }
    }
}

// ---------------------------------------------------------------------------
// ForeignLanguageHook
// ---------------------------------------------------------------------------

/// Describes a hook that bridges a foreign language runtime into the
/// capability framework.
pub struct ForeignLanguageHook {
    language: ForeignLanguage,
    entry_point: String,
    function_name: String,
    config: HashMap<String, serde_json::Value>,
}

impl ForeignLanguageHook {
    pub fn new(
        language: ForeignLanguage,
        entry_point: impl Into<String>,
        function_name: impl Into<String>,
    ) -> Self {
        Self {
            language,
            entry_point: entry_point.into(),
            function_name: function_name.into(),
            config: HashMap::new(),
        }
    }

    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    pub fn to_metadata(&self) -> CapabilityMetadata {
        let name = format!("{}_{}", self.language, self.function_name);
        let description = format!(
            "Foreign language hook for {}: {}.{}",
            self.language, self.entry_point, self.function_name
        );
        let mut metadata = CapabilityMetadata::new(
            &name,
            CapabilityVersion::initial(),
            &description,
            CapabilityCategory::Custom(self.language.to_string()),
        );
        metadata.custom.insert(
            "language".to_string(),
            serde_json::json!(self.language.to_string()),
        );
        metadata
            .custom
            .insert("entry_point".to_string(), serde_json::json!(&self.entry_point));
        metadata.custom.insert(
            "function_name".to_string(),
            serde_json::json!(&self.function_name),
        );
        metadata
            .custom
            .insert("config".to_string(), serde_json::json!(&self.config));
        metadata
    }
}

// ---------------------------------------------------------------------------
// PluginCapability
// ---------------------------------------------------------------------------

/// A single capability declared by a plugin manifest.
pub struct PluginCapability {
    pub name: String,
    pub description: String,
    pub category: CapabilityCategory,
    pub version: String,
    pub inputs: Vec<InputSchema>,
    pub output: OutputSchema,
}

impl PluginCapability {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        category: CapabilityCategory,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category,
            version: "1.0.0".to_string(),
            inputs: Vec::new(),
            output: OutputSchema::new(serde_json::Value::Null, "no output"),
        }
    }

    pub fn validate(&self) -> CapabilityResult<()> {
        if self.name.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin capability name cannot be empty",
            ));
        }
        if self.description.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin capability description cannot be empty",
            ));
        }
        self.version.parse::<CapabilityVersion>().map_err(|e| {
            CapabilityError::validation_failed(format!(
                "invalid capability version '{}': {}",
                self.version, e
            ))
        })?;
        Ok(())
    }

    pub fn to_metadata(&self) -> CapabilityMetadata {
        let version = self.version.parse::<CapabilityVersion>()
            .unwrap_or_else(|_| CapabilityVersion::initial());
        let mut metadata = CapabilityMetadata::new(
            &self.name,
            version,
            &self.description,
            self.category.clone(),
        );
        metadata.inputs = self.inputs.clone();
        metadata.output = self.output.clone();
        metadata
    }
}

// ---------------------------------------------------------------------------
// PluginManifest
// ---------------------------------------------------------------------------

/// Describes a plugin and the capabilities it provides.
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<PluginCapability>,
    pub dependencies: Vec<String>,
    pub permissions: Vec<String>,
    pub min_neo_version: String,
}

impl PluginManifest {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        author: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            author: author.into(),
            description: description.into(),
            capabilities: Vec::new(),
            dependencies: Vec::new(),
            permissions: Vec::new(),
            min_neo_version: "1.0.0".to_string(),
        }
    }

    pub fn validate(&self) -> CapabilityResult<()> {
        if self.name.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin manifest name cannot be empty",
            ));
        }
        if self.version.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin manifest version cannot be empty",
            ));
        }
        if self.author.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin manifest author cannot be empty",
            ));
        }
        if self.description.is_empty() {
            return Err(CapabilityError::validation_failed(
                "plugin manifest description cannot be empty",
            ));
        }
        self.version.parse::<CapabilityVersion>().map_err(|e| {
            CapabilityError::validation_failed(format!(
                "invalid plugin version '{}': {}",
                self.version, e
            ))
        })?;
        self.min_neo_version.parse::<CapabilityVersion>().map_err(|e| {
            CapabilityError::validation_failed(format!(
                "invalid min_neo_version '{}': {}",
                self.min_neo_version, e
            ))
        })?;
        for (i, cap) in self.capabilities.iter().enumerate() {
            cap.validate().map_err(|e| {
                CapabilityError::validation_failed(format!(
                    "plugin capability '{}' (index {}): {}",
                    cap.name, i, e
                ))
            })?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// PluginAuthoringKit
// ---------------------------------------------------------------------------

/// Toolkit for plugin authors to define manifests and capabilities, then build
/// them into live capability instances.
pub struct PluginAuthoringKit {
    manifest: PluginManifest,
    capabilities: Vec<Arc<RwLock<dyn Capability>>>,
}

impl PluginAuthoringKit {
    pub fn new(manifest: PluginManifest) -> Self {
        Self {
            manifest,
            capabilities: Vec::new(),
        }
    }

    pub fn add_capability(&mut self, cap: Arc<RwLock<dyn Capability>>) {
        self.capabilities.push(cap);
    }

    pub fn build(self) -> CapabilityResult<Vec<Arc<RwLock<dyn Capability>>>> {
        Ok(self.capabilities)
    }

    pub fn validate(&self) -> CapabilityResult<()> {
        self.manifest.validate()?;

        if self.capabilities.len() != self.manifest.capabilities.len() {
            return Err(CapabilityError::validation_failed(format!(
                "manifest declares {} capabilities but {} were provided",
                self.manifest.capabilities.len(),
                self.capabilities.len()
            )));
        }

        for (i, (cap, manifest_cap)) in self
            .capabilities
            .iter()
            .zip(self.manifest.capabilities.iter())
            .enumerate()
        {
            let guard = cap.read();
            let meta = guard.metadata();
            if meta.name != manifest_cap.name {
                return Err(CapabilityError::validation_failed(format!(
                    "capability index {} name mismatch: code='{}' manifest='{}'",
                    i, meta.name, manifest_cap.name
                )));
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SdkRegistry
// ---------------------------------------------------------------------------

/// Central registry that manages all registered capability builders and
/// plugin registrations.
pub struct SdkRegistry {
    builders: RwLock<HashMap<String, CapabilityBuilder>>,
}

impl SdkRegistry {
    pub fn new() -> Self {
        Self {
            builders: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_builder(&self, name: impl Into<String>, builder: CapabilityBuilder) {
        self.builders.write().insert(name.into(), builder);
    }

    pub fn get_builder(&self, name: &str) -> Option<CapabilityBuilder> {
        self.builders.read().get(name).cloned()
    }

    pub fn build_capability(&self, name: &str) -> CapabilityResult<Arc<RwLock<dyn Capability>>> {
        let builder = self
            .builders
            .read()
            .get(name)
            .cloned()
            .ok_or_else(|| {
                CapabilityError::not_found(format!(
                    "no capability builder registered with name '{}'",
                    name
                ))
            })?;
        builder.build()
    }

    pub fn list_builders(&self) -> Vec<String> {
        self.builders.read().keys().cloned().collect()
    }

    pub fn remove_builder(&self, name: &str) -> bool {
        self.builders.write().remove(name).is_some()
    }

    pub fn register_plugin(
        &self,
        plugin: PluginAuthoringKit,
    ) -> CapabilityResult<Vec<CapabilityId>> {
        plugin.validate()?;
        let capabilities = plugin.build()?;
        let ids: Vec<CapabilityId> = capabilities
            .iter()
            .map(|cap| cap.read().metadata().id)
            .collect();
        Ok(ids)
    }
}

impl Default for SdkRegistry {
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

    fn make_context() -> ExecutionContext {
        ExecutionContext::new(CapabilityId::new())
    }

    fn make_builder() -> CapabilityBuilder {
        CapabilityBuilder::new(
            "test-cap",
            CapabilityVersion::new(1, 0, 0),
            "A test capability",
            CapabilityCategory::Tool,
        )
        .on_execute(|input, _ctx| {
            Box::pin(async move {
                let greeting = input
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("world");
                Ok(CapabilityResult_output::success(
                    serde_json::json!({ "greeting": format!("hello, {}", greeting) }),
                    5,
                ))
            })
        })
    }

    // ── CapabilityBuilder tests ────────────────────────────────────────────

    #[test]
    fn builder_new_sets_metadata() {
        let b = CapabilityBuilder::new(
            "my-cap",
            CapabilityVersion::new(2, 1, 0),
            "desc",
            CapabilityCategory::Inference,
        );
        assert_eq!(b.metadata.name, "my-cap");
        assert_eq!(b.metadata.version, CapabilityVersion::new(2, 1, 0));
        assert_eq!(b.metadata.description, "desc");
        assert_eq!(b.metadata.category, CapabilityCategory::Inference);
    }

    #[test]
    fn builder_chain_all_options() {
        let dep_id = CapabilityId::new();
        let b = CapabilityBuilder::new("full", CapabilityVersion::initial(), "d", CapabilityCategory::System)
            .namespace("my.ns")
            .tag("alpha")
            .tag("beta")
            .alias("ful")
            .author("alice")
            .license("MIT")
            .input(
                serde_json::json!({"type": "object", "required": ["x"]}),
                "x input",
            )
            .output(
                serde_json::json!({"type": "object"}),
                "output",
            )
            .dependency(dep_id)
            .permission("read")
            .permission("write")
            .resources(0.5, 0.25, 1024 * 1024, 512)
            .timeout_ms(30_000)
            .max_retries(3)
            .requires_approval()
            .on_execute(|_input, _ctx| {
                Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({}), 0)) })
            });

        assert_eq!(b.metadata.namespace.as_str(), "my.ns");
        assert!(b.metadata.tags.contains("alpha"));
        assert!(b.metadata.tags.contains("beta"));
        assert!(b.metadata.aliases.matches("ful"));
        assert_eq!(b.metadata.author, "alice");
        assert_eq!(b.metadata.license, "MIT");
        assert_eq!(b.metadata.inputs.len(), 1);
        assert_eq!(b.metadata.dependencies.len(), 1);
        assert_eq!(b.metadata.required_permissions.len(), 2);
        assert_eq!(b.metadata.resource_requirements.cpu_units, 0.5);
        assert_eq!(b.metadata.resource_requirements.gpu_units, 0.25);
        assert_eq!(b.metadata.resource_requirements.memory_bytes, 1024 * 1024);
        assert_eq!(b.metadata.resource_requirements.inference_tokens, 512);
        assert_eq!(b.metadata.timeout_ms, Some(30_000));
        assert_eq!(b.metadata.max_retries, 3);
        assert!(b.metadata.requires_approval);
        assert!(b.execute_fn.is_some());
    }

    #[test]
    fn builder_build_without_execute_fails() {
        let b = CapabilityBuilder::new(
            "no-exec",
            CapabilityVersion::initial(),
            "desc",
            CapabilityCategory::System,
        );
        match b.build() {
            Ok(_) => panic!("expected error"),
            Err(e) => {
                let msg = format!("{}", e);
                assert!(msg.contains("no execute handler"));
            }
        }
    }

    #[test]
    fn builder_build_validates_empty_name() {
        let b = CapabilityBuilder::new(
            "",
            CapabilityVersion::initial(),
            "desc",
            CapabilityCategory::System,
        )
        .on_execute(|_i, _c| {
            Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({}), 0)) })
        });
        assert!(b.build().is_err());
    }

    #[test]
    fn builder_build_validates_empty_description() {
        let b = CapabilityBuilder::new(
            "ok",
            CapabilityVersion::initial(),
            "",
            CapabilityCategory::System,
        )
        .on_execute(|_i, _c| {
            Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({}), 0)) })
        });
        assert!(b.build().is_err());
    }

    #[test]
    fn builder_build_succeeds() {
        let b = make_builder();
        let cap = b.build().unwrap();
        let guard = cap.read();
        assert_eq!(guard.metadata().name, "test-cap");
    }

    #[test]
    fn builder_execute_fn_replaces() {
        let b1 = CapabilityBuilder::new(
            "x",
            CapabilityVersion::initial(),
            "d",
            CapabilityCategory::System,
        )
        .on_execute(|_i, _c| {
            Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({"v": 1}), 0)) })
        });
        let b2 = b1.on_execute(|_i, _c| {
            Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({"v": 2}), 0)) })
        });
        let cap = b2.build().unwrap();
        let guard = cap.read();
        let meta = guard.metadata();
        assert_eq!(meta.name, "x");
    }

    // ── DefaultCapability tests ────────────────────────────────────────────

    #[tokio::test]
    async fn default_capability_execute() {
        let cap = make_builder().build().unwrap();
        let ctx = make_context();
        let result = cap
            .read()
            .execute(
                serde_json::json!({ "name": "neo" }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["greeting"], "hello, neo");
        assert_eq!(result.duration_ms, 5);
    }

    #[tokio::test]
    async fn default_capability_execute_default_input() {
        let cap = make_builder().build().unwrap();
        let ctx = make_context();
        let result = cap.read().execute(serde_json::json!({}), ctx).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output["greeting"], "hello, world");
    }

    #[test]
    fn default_capability_metadata_access() {
        let cap = make_builder().build().unwrap();
        let guard = cap.read();
        let meta = guard.metadata();
        assert_eq!(meta.name, "test-cap");
        assert_eq!(meta.version, CapabilityVersion::new(1, 0, 0));
        assert_eq!(meta.category, CapabilityCategory::Tool);
    }

    #[test]
    fn default_capability_metadata_mut() {
        let cap = make_builder().build().unwrap();
        {
            let mut guard = cap.write();
            guard.metadata_mut().author = "updated-author".to_string();
        }
        let guard = cap.read();
        assert_eq!(guard.metadata().author, "updated-author");
    }

    #[test]
    fn default_capability_validate_input_passes() {
        let cap = make_builder().build().unwrap();
        assert!(cap.read().validate_input(&serde_json::json!({ "name": "test" })).is_ok());
    }

    // ── ForeignLanguage tests ──────────────────────────────────────────────

    #[test]
    fn foreign_language_display() {
        assert_eq!(ForeignLanguage::Python.to_string(), "python");
        assert_eq!(ForeignLanguage::Go.to_string(), "go");
        assert_eq!(ForeignLanguage::TypeScript.to_string(), "typescript");
        assert_eq!(ForeignLanguage::Ruby.to_string(), "ruby");
        assert_eq!(ForeignLanguage::Java.to_string(), "java");
    }

    #[test]
    fn foreign_language_equality() {
        assert_eq!(ForeignLanguage::Python, ForeignLanguage::Python);
        assert_ne!(ForeignLanguage::Python, ForeignLanguage::Go);
    }

    #[test]
    fn foreign_language_hashset() {
        let mut set = std::collections::HashSet::new();
        set.insert(ForeignLanguage::Python);
        set.insert(ForeignLanguage::Python);
        assert_eq!(set.len(), 1);
        set.insert(ForeignLanguage::Go);
        assert_eq!(set.len(), 2);
    }

    // ── ForeignLanguageHook tests ──────────────────────────────────────────

    #[test]
    fn hook_new() {
        let hook = ForeignLanguageHook::new(
            ForeignLanguage::Python,
            "my_module.py",
            "process",
        );
        assert_eq!(hook.language, ForeignLanguage::Python);
        assert_eq!(hook.entry_point, "my_module.py");
        assert_eq!(hook.function_name, "process");
        assert!(hook.config.is_empty());
    }

    #[test]
    fn hook_with_config() {
        let hook = ForeignLanguageHook::new(
            ForeignLanguage::Go,
            "handler.go",
            "Handle",
        )
        .with_config("timeout", serde_json::json!(30))
        .with_config("retries", serde_json::json!(3));

        assert_eq!(hook.config.len(), 2);
        assert_eq!(hook.config["timeout"], serde_json::json!(30));
        assert_eq!(hook.config["retries"], serde_json::json!(3));
    }

    #[test]
    fn hook_to_metadata() {
        let hook = ForeignLanguageHook::new(
            ForeignLanguage::TypeScript,
            "src/handler.ts",
            "runInference",
        )
        .with_config("backend", serde_json::json!("wasm"));

        let meta = hook.to_metadata();
        assert_eq!(meta.name, "typescript_runInference");
        assert!(meta.description.contains("typescript"));
        assert!(meta.description.contains("src/handler.ts"));
        assert!(meta.description.contains("runInference"));
        assert_eq!(
            meta.category,
            CapabilityCategory::Custom("typescript".to_string())
        );
        assert_eq!(
            meta.custom["language"],
            serde_json::json!("typescript")
        );
        assert_eq!(
            meta.custom["entry_point"],
            serde_json::json!("src/handler.ts")
        );
        assert_eq!(
            meta.custom["function_name"],
            serde_json::json!("runInference")
        );
        assert_eq!(
            meta.custom["config"],
            serde_json::json!({"backend": "wasm"})
        );
        assert_eq!(meta.version, CapabilityVersion::initial());
    }

    #[test]
    fn hook_to_metadata_all_languages() {
        for lang in &[
            ForeignLanguage::Python,
            ForeignLanguage::Go,
            ForeignLanguage::TypeScript,
            ForeignLanguage::Ruby,
            ForeignLanguage::Java,
        ] {
            let hook = ForeignLanguageHook::new(*lang, "entry", "func");
            let meta = hook.to_metadata();
            assert!(meta.name.starts_with(&format!("{}_", lang)));
        }
    }

    // ── PluginCapability tests ─────────────────────────────────────────────

    #[test]
    fn plugin_capability_new() {
        let pc = PluginCapability::new(
            "summarize",
            "Summarizes text",
            CapabilityCategory::Inference,
        );
        assert_eq!(pc.name, "summarize");
        assert_eq!(pc.description, "Summarizes text");
        assert_eq!(pc.category, CapabilityCategory::Inference);
        assert_eq!(pc.version, "1.0.0");
        assert!(pc.inputs.is_empty());
    }

    #[test]
    fn plugin_capability_validate_ok() {
        let pc = PluginCapability::new("cap", "desc", CapabilityCategory::Tool);
        assert!(pc.validate().is_ok());
    }

    #[test]
    fn plugin_capability_validate_empty_name() {
        let pc = PluginCapability::new("", "desc", CapabilityCategory::Tool);
        assert!(pc.validate().is_err());
    }

    #[test]
    fn plugin_capability_validate_empty_description() {
        let pc = PluginCapability::new("cap", "", CapabilityCategory::Tool);
        assert!(pc.validate().is_err());
    }

    #[test]
    fn plugin_capability_validate_bad_version() {
        let mut pc = PluginCapability::new("cap", "desc", CapabilityCategory::Tool);
        pc.version = "not-a-version".to_string();
        assert!(pc.validate().is_err());
    }

    #[test]
    fn plugin_capability_to_metadata() {
        let pc = PluginCapability {
            name: "my-plugin-cap".to_string(),
            description: "does things".to_string(),
            category: CapabilityCategory::Workflow,
            version: "2.3.1".to_string(),
            inputs: vec![InputSchema::new(
                serde_json::json!({"type": "string"}),
                "input text",
            )],
            output: OutputSchema::new(
                serde_json::json!({"type": "string"}),
                "output text",
            ),
        };

        let meta = pc.to_metadata();
        assert_eq!(meta.name, "my-plugin-cap");
        assert_eq!(meta.version, CapabilityVersion::new(2, 3, 1));
        assert_eq!(meta.description, "does things");
        assert_eq!(meta.category, CapabilityCategory::Workflow);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.output.description, "output text");
    }

    #[test]
    fn plugin_capability_to_metadata_bad_version_defaults() {
        let mut pc = PluginCapability::new("x", "y", CapabilityCategory::System);
        pc.version = "bad".to_string();
        let meta = pc.to_metadata();
        assert_eq!(meta.version, CapabilityVersion::initial());
    }

    // ── PluginManifest tests ───────────────────────────────────────────────

    #[test]
    fn manifest_new() {
        let m = PluginManifest::new("my-plugin", "1.0.0", "author", "description");
        assert_eq!(m.name, "my-plugin");
        assert_eq!(m.version, "1.0.0");
        assert_eq!(m.author, "author");
        assert_eq!(m.description, "description");
        assert!(m.capabilities.is_empty());
        assert!(m.dependencies.is_empty());
        assert!(m.permissions.is_empty());
        assert_eq!(m.min_neo_version, "1.0.0");
    }

    #[test]
    fn manifest_validate_ok() {
        let mut m = PluginManifest::new("p", "1.0.0", "a", "d");
        m.capabilities
            .push(PluginCapability::new("c1", "desc", CapabilityCategory::Tool));
        assert!(m.validate().is_ok());
    }

    #[test]
    fn manifest_validate_empty_name() {
        let m = PluginManifest::new("", "1.0.0", "a", "d");
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_empty_version() {
        let m = PluginManifest::new("p", "", "a", "d");
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_empty_author() {
        let m = PluginManifest::new("p", "1.0.0", "", "d");
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_empty_description() {
        let m = PluginManifest::new("p", "1.0.0", "a", "");
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_bad_version() {
        let m = PluginManifest::new("p", "not-semver", "a", "d");
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_bad_min_neo_version() {
        let mut m = PluginManifest::new("p", "1.0.0", "a", "d");
        m.min_neo_version = "oops".to_string();
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_bad_capability_version() {
        let mut m = PluginManifest::new("p", "1.0.0", "a", "d");
        let mut cap = PluginCapability::new("c", "desc", CapabilityCategory::Tool);
        cap.version = "bad".to_string();
        m.capabilities.push(cap);
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_capability_empty_name() {
        let mut m = PluginManifest::new("p", "1.0.0", "a", "d");
        m.capabilities
            .push(PluginCapability::new("", "desc", CapabilityCategory::Tool));
        assert!(m.validate().is_err());
    }

    // ── PluginAuthoringKit tests ───────────────────────────────────────────

    fn make_kit_manifest() -> PluginManifest {
        let mut m = PluginManifest::new("test-plugin", "1.0.0", "tester", "A test plugin");
        m.capabilities
            .push(PluginCapability::new("cap-a", "first cap", CapabilityCategory::Tool));
        m.capabilities
            .push(PluginCapability::new("cap-b", "second cap", CapabilityCategory::System));
        m
    }

    fn make_kit_cap(name: &str) -> Arc<RwLock<dyn Capability>> {
        let b = CapabilityBuilder::new(
            name,
            CapabilityVersion::initial(),
            format!("desc for {}", name),
            CapabilityCategory::Tool,
        )
        .on_execute(|_i, _c| {
            Box::pin(async { Ok(CapabilityResult_output::success(serde_json::json!({}), 0)) })
        });
        b.build().unwrap()
    }

    #[test]
    fn kit_new() {
        let m = make_kit_manifest();
        let kit = PluginAuthoringKit::new(m);
        assert!(kit.capabilities.is_empty());
    }

    #[test]
    fn kit_add_capability() {
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("cap-a"));
        kit.add_capability(make_kit_cap("cap-b"));
        assert_eq!(kit.capabilities.len(), 2);
    }

    #[test]
    fn kit_build() {
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("cap-a"));
        kit.add_capability(make_kit_cap("cap-b"));

        let built = kit.build().unwrap();
        assert_eq!(built.len(), 2);
        for cap in &built {
            let guard = cap.read();
            assert!(!guard.metadata().name.is_empty());
        }
    }

    #[test]
    fn kit_validate_ok() {
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("cap-a"));
        kit.add_capability(make_kit_cap("cap-b"));
        assert!(kit.validate().is_ok());
    }

    #[test]
    fn kit_validate_count_mismatch() {
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("cap-a"));
        assert!(kit.validate().is_err());
        match kit.validate() {
            Ok(_) => panic!("expected error"),
            Err(e) => {
                assert!(format!("{}", e).contains("declares 2"));
            }
        }
    }

    #[test]
    fn kit_validate_name_mismatch() {
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("wrong-name"));
        kit.add_capability(make_kit_cap("cap-b"));
        assert!(kit.validate().is_err());
        match kit.validate() {
            Ok(_) => panic!("expected error"),
            Err(e) => {
                assert!(format!("{}", e).contains("mismatch"));
            }
        }
    }

    #[test]
    fn kit_validate_manifest_invalid() {
        let m = PluginManifest::new("", "1.0.0", "a", "d");
        let kit = PluginAuthoringKit::new(m);
        assert!(kit.validate().is_err());
    }

    // ── SdkRegistry tests ──────────────────────────────────────────────────

    #[test]
    fn registry_new() {
        let reg = SdkRegistry::new();
        assert!(reg.list_builders().is_empty());
    }

    #[test]
    fn registry_default() {
        let reg = SdkRegistry::default();
        assert!(reg.list_builders().is_empty());
    }

    #[test]
    fn registry_register_and_get() {
        let reg = SdkRegistry::new();
        let builder = make_builder();
        reg.register_builder("my-cap", builder);

        let retrieved = reg.get_builder("my-cap");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.name, "test-cap");
    }

    #[test]
    fn registry_get_nonexistent() {
        let reg = SdkRegistry::new();
        assert!(reg.get_builder("nope").is_none());
    }

    #[test]
    fn registry_build_capability() {
        let reg = SdkRegistry::new();
        reg.register_builder("built-cap", make_builder());

        let cap = reg.build_capability("built-cap").unwrap();
        assert_eq!(cap.read().metadata().name, "test-cap");
    }

    #[test]
    fn registry_build_not_found() {
        let reg = SdkRegistry::new();
        match reg.build_capability("missing") {
            Ok(_) => panic!("expected error"),
            Err(e) => {
                let msg = format!("{}", e);
                assert!(msg.contains("missing"));
            }
        }
    }

    #[tokio::test]
    async fn registry_build_and_execute() {
        let reg = SdkRegistry::new();
        reg.register_builder("exec-cap", make_builder());

        let cap = reg.build_capability("exec-cap").unwrap();
        let ctx = make_context();
        let result = cap
            .read()
            .execute(serde_json::json!({ "name": "registry" }), ctx)
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["greeting"], "hello, registry");
    }

    #[test]
    fn registry_list_builders() {
        let reg = SdkRegistry::new();
        reg.register_builder("a", make_builder());
        reg.register_builder("b", make_builder());
        reg.register_builder("c", make_builder());

        let mut list = reg.list_builders();
        list.sort();
        assert_eq!(list, vec!["a", "b", "c"]);
    }

    #[test]
    fn registry_remove_builder() {
        let reg = SdkRegistry::new();
        reg.register_builder("to-remove", make_builder());
        assert!(reg.get_builder("to-remove").is_some());

        assert!(reg.remove_builder("to-remove"));
        assert!(reg.get_builder("to-remove").is_none());
    }

    #[test]
    fn registry_remove_nonexistent() {
        let reg = SdkRegistry::new();
        assert!(!reg.remove_builder("nope"));
    }

    #[test]
    fn registry_overwrite_builder() {
        let reg = SdkRegistry::new();
        reg.register_builder("x", make_builder());
        reg.register_builder("x", make_builder());

        let list = reg.list_builders();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn registry_register_plugin() {
        let reg = SdkRegistry::new();
        let m = make_kit_manifest();
        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("cap-a"));
        kit.add_capability(make_kit_cap("cap-b"));

        let ids = reg.register_plugin(kit).unwrap();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
    }

    #[test]
    fn registry_register_plugin_invalid() {
        let reg = SdkRegistry::new();
        let m = PluginManifest::new("", "1.0.0", "a", "d");
        let kit = PluginAuthoringKit::new(m);
        assert!(reg.register_plugin(kit).is_err());
    }

    #[test]
    fn registry_register_plugin_count_mismatch() {
        let reg = SdkRegistry::new();
        let mut m = make_kit_manifest();
        m.capabilities
            .push(PluginCapability::new("cap-c", "extra", CapabilityCategory::Tool));
        let kit = PluginAuthoringKit::new(m);
        assert!(reg.register_plugin(kit).is_err());
    }

    // ── Integration: builder → registry → execute ──────────────────────────

    #[tokio::test]
    async fn full_pipeline_build_register_execute() {
        let reg = SdkRegistry::new();

        let builder = CapabilityBuilder::new(
            "math-add",
            CapabilityVersion::new(1, 2, 0),
            "Adds two numbers",
            CapabilityCategory::Tool,
        )
        .namespace("math")
        .tag("arithmetic")
        .author("neo")
        .license("MIT")
        .input(
            serde_json::json!({
                "type": "object",
                "required": ["a", "b"],
                "properties": {
                    "a": {"type": "number"},
                    "b": {"type": "number"}
                }
            }),
            "two numbers to add",
        )
        .output(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "sum": {"type": "number"}
                }
            }),
            "the sum",
        )
        .resources(0.01, 0.0, 1024, 0)
        .timeout_ms(1000)
        .on_execute(|input, _ctx| {
            Box::pin(async move {
                let a = input.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let b = input.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);
                Ok(CapabilityResult_output::success(
                    serde_json::json!({ "sum": a + b }),
                    1,
                ))
            })
        });

        reg.register_builder("math-add", builder);

        let cap = reg.build_capability("math-add").unwrap();
        assert_eq!(cap.read().metadata().name, "math-add");
        assert_eq!(cap.read().metadata().version, CapabilityVersion::new(1, 2, 0));
        assert_eq!(cap.read().metadata().namespace.as_str(), "math");
        assert!(cap.read().metadata().tags.contains("arithmetic"));
        assert_eq!(cap.read().metadata().author, "neo");
        assert_eq!(cap.read().metadata().license, "MIT");

        let ctx = make_context();
        let result = cap
            .read()
            .execute(
                serde_json::json!({ "a": 3.0, "b": 7.0 }),
                ctx,
            )
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["sum"], 10.0);
    }

    // ── Serialization round-trip for ForeignLanguage ───────────────────────

    #[test]
    fn foreign_language_serialization_roundtrip() {
        let langs = vec![
            ForeignLanguage::Python,
            ForeignLanguage::Go,
            ForeignLanguage::TypeScript,
            ForeignLanguage::Ruby,
            ForeignLanguage::Java,
        ];
        for lang in &langs {
            let json = serde_json::to_string(lang).unwrap();
            let restored: ForeignLanguage = serde_json::from_str(&json).unwrap();
            assert_eq!(*lang, restored);
        }
    }

    // ── Builder clone preserves execute function ───────────────────────────

    #[tokio::test]
    async fn builder_clone_preserves_execute() {
        let builder = make_builder();
        let cloned = builder.clone();

        let cap = cloned.build().unwrap();
        let ctx = make_context();
        let result = cap
            .read()
            .execute(serde_json::json!({ "name": "cloned" }), ctx)
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output["greeting"], "hello, cloned");
    }

    // ── Multiple builders in registry ──────────────────────────────────────

    #[tokio::test]
    async fn registry_multiple_builders_independent() {
        let reg = SdkRegistry::new();

        let b1 = CapabilityBuilder::new(
            "cap-alpha",
            CapabilityVersion::initial(),
            "alpha",
            CapabilityCategory::Tool,
        )
        .on_execute(|_i, _c| {
            Box::pin(async {
                Ok(CapabilityResult_output::success(
                    serde_json::json!({ "from": "alpha" }),
                    0,
                ))
            })
        });

        let b2 = CapabilityBuilder::new(
            "cap-beta",
            CapabilityVersion::new(2, 0, 0),
            "beta",
            CapabilityCategory::System,
        )
        .on_execute(|_i, _c| {
            Box::pin(async {
                Ok(CapabilityResult_output::success(
                    serde_json::json!({ "from": "beta" }),
                    0,
                ))
            })
        });

        reg.register_builder("alpha", b1);
        reg.register_builder("beta", b2);

        let cap_a = reg.build_capability("alpha").unwrap();
        let cap_b = reg.build_capability("beta").unwrap();

        assert_eq!(cap_a.read().metadata().name, "cap-alpha");
        assert_eq!(cap_b.read().metadata().name, "cap-beta");
        assert_eq!(cap_b.read().metadata().version, CapabilityVersion::new(2, 0, 0));

        let ctx1 = make_context();
        let r1 = cap_a
            .read()
            .execute(serde_json::json!({}), ctx1)
            .await
            .unwrap();
        assert_eq!(r1.output["from"], "alpha");

        let ctx2 = make_context();
        let r2 = cap_b
            .read()
            .execute(serde_json::json!({}), ctx2)
            .await
            .unwrap();
        assert_eq!(r2.output["from"], "beta");
    }

    // ── Plugin manifest with multiple capabilities and kit build ───────────

    #[test]
    fn kit_build_returns_arc_rwlock() {
        let mut m = PluginManifest::new("multi", "1.0.0", "author", "multi-cap plugin");
        m.capabilities
            .push(PluginCapability::new("a", "cap a", CapabilityCategory::Tool));
        m.capabilities
            .push(PluginCapability::new("b", "cap b", CapabilityCategory::Memory));
        m.capabilities
            .push(PluginCapability::new("c", "cap c", CapabilityCategory::Network));

        let mut kit = PluginAuthoringKit::new(m);
        kit.add_capability(make_kit_cap("a"));
        kit.add_capability(make_kit_cap("b"));
        kit.add_capability(make_kit_cap("c"));

        let built = kit.build().unwrap();
        assert_eq!(built.len(), 3);

        let names: Vec<String> = built
            .iter()
            .map(|c| c.read().metadata().name.clone())
            .collect();
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
        assert!(names.contains(&"c".to_string()));
    }
}
