#[cfg(test)]
mod tests {
    use neo_core::plugins::*;
    use neo_core::plugins::capabilities::*;
    use neo_core::plugins::loader::*;
    use neo_core::plugins::manifest::*;
    use neo_core::plugins::sandbox::*;
    use std::collections::HashMap;

    // ── PluginManifest ────────────────────────────────────────────────

    #[test]
    fn test_plugin_manifest_creation() {
        let manifest = PluginManifest {
            id: "test-plugin".into(),
            name: "Test Plugin".into(),
            version: "1.0.0".into(),
            author: "test".into(),
            description: "A test plugin".into(),
            neo_version_req: ">=0.1.0".into(),
            capabilities: PluginType::Tool,
            entry_point: "libtest".into(),
            dependencies: vec![],
            config_schema: None,
        };
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.capabilities, PluginType::Tool);
    }

    #[test]
    fn test_plugin_manifest_serde_roundtrip() {
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "Plugin One".into(),
            version: "2.0.0".into(),
            author: "neo".into(),
            description: "desc".into(),
            neo_version_req: ">=1.0".into(),
            capabilities: PluginType::Provider,
            entry_point: "main".into(),
            dependencies: vec![Dependency {
                name: "dep1".into(),
                version_req: ">=1.0".into(),
                optional: false,
            }],
            config_schema: None,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "p1");
        assert_eq!(deserialized.capabilities, PluginType::Provider);
        assert_eq!(deserialized.dependencies.len(), 1);
    }

    #[test]
    fn test_plugin_manifest_all_plugin_types() {
        let types = [
            PluginType::Tool,
            PluginType::Workflow,
            PluginType::Provider,
            PluginType::Capability,
            PluginType::PromptTemplate,
            PluginType::Retriever,
            PluginType::Planner,
        ];
        for pt in &types {
            let manifest = PluginManifest {
                id: "id".into(),
                name: "name".into(),
                version: "1.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: pt.clone(),
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            };
            let json = serde_json::to_string(&manifest).unwrap();
            let deserialized: PluginManifest = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.capabilities, *pt);
        }
    }

    // ── PluginType ────────────────────────────────────────────────────

    #[test]
    fn test_plugin_type_serde_roundtrip() {
        let types = [
            PluginType::Tool,
            PluginType::Workflow,
            PluginType::Provider,
            PluginType::Capability,
            PluginType::PromptTemplate,
            PluginType::Retriever,
            PluginType::Planner,
        ];
        for pt in &types {
            let json = serde_json::to_string(pt).unwrap();
            let deserialized: PluginType = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, pt);
        }
    }

    // ── Dependency ────────────────────────────────────────────────────

    #[test]
    fn test_dependency_serde_roundtrip() {
        let dep = Dependency {
            name: "serde".into(),
            version_req: ">=1.0".into(),
            optional: false,
        };
        let json = serde_json::to_string(&dep).unwrap();
        let deserialized: Dependency = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "serde");
        assert!(!deserialized.optional);
    }

    #[test]
    fn test_dependency_optional() {
        let dep = Dependency {
            name: "optional-dep".into(),
            version_req: ">=2.0".into(),
            optional: true,
        };
        let json = serde_json::to_string(&dep).unwrap();
        let deserialized: Dependency = serde_json::from_str(&json).unwrap();
        assert!(deserialized.optional);
    }

    // ── PluginCapabilities ────────────────────────────────────────────

    #[test]
    fn test_plugin_capabilities_default() {
        let caps = PluginCapabilities::default();
        assert!(caps.tools.is_empty());
        assert!(caps.workflows.is_empty());
        assert!(caps.providers.is_empty());
        assert!(caps.prompt_templates.is_empty());
        assert!(caps.retrievers.is_empty());
        assert!(caps.planners.is_empty());
        assert!(caps.custom.is_empty());
    }

    #[test]
    fn test_plugin_capabilities_serde_roundtrip() {
        let mut caps = PluginCapabilities::default();
        caps.tools.insert(
            "search".into(),
            ToolCapability {
                name: "search".into(),
                description: "search tool".into(),
                parameters_schema: None,
                requires_approval: false,
            },
        );
        caps.prompt_templates.insert("greeting".into(), "Hello!".into());
        caps.custom.insert("key".into(), serde_json::json!({"a": 1}));

        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: PluginCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tools.len(), 1);
        assert!(deserialized.tools.contains_key("search"));
        assert_eq!(deserialized.prompt_templates["greeting"], "Hello!");
    }

    // ── ToolCapability ────────────────────────────────────────────────

    #[test]
    fn test_tool_capability_serde_roundtrip() {
        let tc = ToolCapability {
            name: "calculator".into(),
            description: "calculates things".into(),
            parameters_schema: Some(serde_json::json!({"type": "object"})),
            requires_approval: true,
        };
        let json = serde_json::to_string(&tc).unwrap();
        let deserialized: ToolCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "calculator");
        assert!(deserialized.requires_approval);
        assert!(deserialized.parameters_schema.is_some());
    }

    // ── WorkflowCapability ────────────────────────────────────────────

    #[test]
    fn test_workflow_capability_serde_roundtrip() {
        let wc = WorkflowCapability {
            name: "etl".into(),
            description: "extract-transform-load".into(),
            input_types: vec!["csv".into(), "json".into()],
            output_types: vec!["parquet".into()],
        };
        let json = serde_json::to_string(&wc).unwrap();
        let deserialized: WorkflowCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.input_types.len(), 2);
        assert_eq!(deserialized.output_types, vec!["parquet".to_string()]);
    }

    // ── ProviderCapability ────────────────────────────────────────────

    #[test]
    fn test_provider_capability_serde_roundtrip() {
        let pc = ProviderCapability {
            name: "openai".into(),
            supported_models: vec!["gpt-4".into(), "gpt-3.5-turbo".into()],
            features: vec!["chat".into(), "completion".into()],
        };
        let json = serde_json::to_string(&pc).unwrap();
        let deserialized: ProviderCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "openai");
        assert_eq!(deserialized.supported_models.len(), 2);
    }

    // ── SandboxConfig ─────────────────────────────────────────────────

    #[test]
    fn test_sandbox_config_default() {
        let cfg = SandboxConfig::default();
        assert_eq!(cfg.level, SandboxLevel::None);
        assert!(cfg.allowed_modules.is_empty());
        assert_eq!(cfg.max_memory_bytes, u64::MAX);
        assert_eq!(cfg.max_cpu_time_ms, u64::MAX);
        assert!(cfg.network_access);
        assert!(cfg.filesystem_access);
    }

    #[test]
    fn test_sandbox_config_serde_roundtrip() {
        let cfg = SandboxConfig {
            level: SandboxLevel::Restricted,
            allowed_modules: vec!["mod_a".into()],
            max_memory_bytes: 1024 * 1024,
            max_cpu_time_ms: 5000,
            network_access: false,
            filesystem_access: false,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: SandboxConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.level, SandboxLevel::Restricted);
        assert!(!deserialized.network_access);
    }

    // ── SandboxLevel ──────────────────────────────────────────────────

    #[test]
    fn test_sandbox_level_display() {
        assert_eq!(format!("{}", SandboxLevel::None), "none");
        assert_eq!(format!("{}", SandboxLevel::Basic), "basic");
        assert_eq!(format!("{}", SandboxLevel::Restricted), "restricted");
        assert_eq!(format!("{}", SandboxLevel::Full), "full");
    }

    #[test]
    fn test_sandbox_level_serde_roundtrip() {
        let levels = [
            SandboxLevel::None,
            SandboxLevel::Basic,
            SandboxLevel::Restricted,
            SandboxLevel::Full,
        ];
        for l in &levels {
            let json = serde_json::to_string(l).unwrap();
            let deserialized: SandboxLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, l);
        }
    }

    // ── PluginSandbox ─────────────────────────────────────────────────

    #[test]
    fn test_plugin_sandbox_new() {
        let sb = PluginSandbox::new(SandboxConfig::default());
        assert!(sb.validate().is_ok());
    }

    #[test]
    fn test_plugin_sandbox_from_level_none() {
        let sb = PluginSandbox::from_level(SandboxLevel::None);
        assert!(sb.validate().is_ok());
        assert!(sb.check_permission("network"));
        assert!(sb.check_permission("filesystem"));
    }

    #[test]
    fn test_plugin_sandbox_from_level_restricted() {
        let sb = PluginSandbox::from_level(SandboxLevel::Restricted);
        assert!(sb.validate().is_ok());
        assert!(!sb.check_permission("network"));
        assert!(!sb.check_permission("filesystem"));
    }

    #[test]
    fn test_plugin_sandbox_from_level_full() {
        let sb = PluginSandbox::from_level(SandboxLevel::Full);
        assert!(sb.validate().is_ok());
        assert!(!sb.check_permission("network"));
        assert!(!sb.check_permission("filesystem"));
    }

    #[test]
    fn test_plugin_sandbox_validate_zero_memory() {
        let cfg = SandboxConfig {
            max_memory_bytes: 0,
            ..SandboxConfig::default()
        };
        let sb = PluginSandbox::new(cfg);
        assert!(sb.validate().is_err());
    }

    #[test]
    fn test_plugin_sandbox_validate_zero_cpu() {
        let cfg = SandboxConfig {
            max_cpu_time_ms: 0,
            ..SandboxConfig::default()
        };
        let sb = PluginSandbox::new(cfg);
        assert!(sb.validate().is_err());
    }

    #[test]
    fn test_plugin_sandbox_enforce_limits_ok() {
        let sb = PluginSandbox::from_level(SandboxLevel::None);
        assert!(sb.enforce_limits(100, 100).is_ok());
    }

    #[test]
    fn test_plugin_sandbox_enforce_limits_memory_exceeded() {
        let cfg = SandboxConfig {
            max_memory_bytes: 100,
            ..SandboxConfig::default()
        };
        let sb = PluginSandbox::new(cfg);
        assert!(sb.enforce_limits(200, 0).is_err());
    }

    #[test]
    fn test_plugin_sandbox_enforce_limits_cpu_exceeded() {
        let cfg = SandboxConfig {
            max_cpu_time_ms: 100,
            ..SandboxConfig::default()
        };
        let sb = PluginSandbox::new(cfg);
        assert!(sb.enforce_limits(0, 200).is_err());
    }

    #[test]
    fn test_plugin_sandbox_check_module_permission() {
        let cfg = SandboxConfig {
            allowed_modules: vec!["allowed_mod".into()],
            ..SandboxConfig::default()
        };
        let sb = PluginSandbox::new(cfg);
        assert!(sb.check_permission("allowed_mod"));
        assert!(!sb.check_permission("denied_mod"));
    }

    #[test]
    fn test_plugin_sandbox_check_unknown_action() {
        let sb = PluginSandbox::from_level(SandboxLevel::None);
        assert!(sb.check_permission("unknown_action"));
        assert!(!sb.check_permission("module:submodule"));
    }

    #[test]
    fn test_plugin_sandbox_restrict() {
        let mut sb = PluginSandbox::from_level(SandboxLevel::None);
        sb.restrict(SandboxLevel::Restricted);
        assert!(!sb.check_permission("network"));
    }

    #[test]
    fn test_plugin_sandbox_restrict_noop() {
        let mut sb = PluginSandbox::from_level(SandboxLevel::Restricted);
        sb.restrict(SandboxLevel::None);
        assert!(!sb.check_permission("network"));
    }

    // ── PluginRegistry ────────────────────────────────────────────────

    #[test]
    fn test_plugin_registry_new() {
        let reg = PluginRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_plugin_registry_default() {
        let reg = PluginRegistry::default();
        assert!(reg.is_empty());
    }

    #[test]
    fn test_plugin_registry_register_and_list() {
        let reg = PluginRegistry::new();
        let info = PluginInfo {
            id: "p1".into(),
            name: "Plugin One".into(),
            version: "1.0.0".into(),
            state: PluginState::Registered,
            capabilities: PluginCapabilities::default(),
            loaded_at: None,
        };
        reg.register(info).unwrap();
        assert_eq!(reg.len(), 1);
        assert!(!reg.is_empty());
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "p1");
    }

    #[test]
    fn test_plugin_registry_register_duplicate() {
        let reg = PluginRegistry::new();
        let info = PluginInfo {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            state: PluginState::Registered,
            capabilities: PluginCapabilities::default(),
            loaded_at: None,
        };
        reg.register(info).unwrap();
        let info2 = PluginInfo {
            id: "p1".into(),
            name: "P1 v2".into(),
            version: "2.0".into(),
            state: PluginState::Registered,
            capabilities: PluginCapabilities::default(),
            loaded_at: None,
        };
        assert!(reg.register(info2).is_err());
    }

    #[test]
    fn test_plugin_registry_unregister() {
        let reg = PluginRegistry::new();
        reg.register(PluginInfo {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            state: PluginState::Registered,
            capabilities: PluginCapabilities::default(),
            loaded_at: None,
        })
        .unwrap();
        assert!(reg.unregister("p1"));
        assert!(reg.is_empty());
    }

    #[test]
    fn test_plugin_registry_unregister_nonexistent() {
        let reg = PluginRegistry::new();
        assert!(!reg.unregister("missing"));
    }

    #[test]
    fn test_plugin_registry_get() {
        let reg = PluginRegistry::new();
        reg.register(PluginInfo {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0.0".into(),
            state: PluginState::Active,
            capabilities: PluginCapabilities::default(),
            loaded_at: Some(1700000000),
        })
        .unwrap();
        let info = reg.get("p1").unwrap();
        assert_eq!(info.name, "P1");
        assert_eq!(info.state, PluginState::Active);
        assert_eq!(info.loaded_at, Some(1700000000));
    }

    #[test]
    fn test_plugin_registry_get_nonexistent() {
        let reg = PluginRegistry::new();
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn test_plugin_registry_multiple_plugins() {
        let reg = PluginRegistry::new();
        for i in 0..5 {
            reg.register(PluginInfo {
                id: format!("p{i}"),
                name: format!("Plugin {i}"),
                version: "1.0".into(),
                state: PluginState::Registered,
                capabilities: PluginCapabilities::default(),
                loaded_at: None,
            })
            .unwrap();
        }
        assert_eq!(reg.len(), 5);
        let list = reg.list();
        assert_eq!(list.len(), 5);
    }

    #[test]
    fn test_plugin_registry_load_from_manifest() {
        let reg = PluginRegistry::new();
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        reg.load_from_manifest(manifest).unwrap();
        assert_eq!(reg.len(), 1);
        let info = reg.get("p1").unwrap();
        assert_eq!(info.state, PluginState::Registered);
    }

    #[test]
    fn test_plugin_registry_load_from_manifest_duplicate() {
        let reg = PluginRegistry::new();
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        reg.load_from_manifest(manifest).unwrap();
        let manifest2 = PluginManifest {
            id: "p1".into(),
            name: "P1-2".into(),
            version: "2.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Workflow,
            entry_point: "ep2".into(),
            dependencies: vec![],
            config_schema: None,
        };
        assert!(reg.load_from_manifest(manifest2).is_err());
    }

    // ── PluginState ───────────────────────────────────────────────────

    #[test]
    fn test_plugin_state_display() {
        assert_eq!(format!("{}", PluginState::Registered), "registered");
        assert_eq!(format!("{}", PluginState::Loading), "loading");
        assert_eq!(format!("{}", PluginState::Active), "active");
        assert_eq!(format!("{}", PluginState::Disabled), "disabled");
        assert_eq!(format!("{}", PluginState::Error), "error");
    }

    // ── PluginInfo ────────────────────────────────────────────────────

    #[test]
    fn test_plugin_info_clone() {
        let info = PluginInfo {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            state: PluginState::Active,
            capabilities: PluginCapabilities::default(),
            loaded_at: Some(100),
        };
        let cloned = info.clone();
        assert_eq!(cloned.id, info.id);
        assert_eq!(cloned.state, info.state);
    }

    // ── LoadResult ────────────────────────────────────────────────────

    #[test]
    fn test_load_result_ok() {
        let lr = LoadResult::ok("plugin-1".into());
        assert!(lr.success);
        assert_eq!(lr.plugin_id, Some("plugin-1".into()));
        assert!(lr.error.is_none());
    }

    #[test]
    fn test_load_result_err() {
        let lr = LoadResult::err("something went wrong".into());
        assert!(!lr.success);
        assert!(lr.plugin_id.is_none());
        assert_eq!(lr.error, Some("something went wrong".into()));
    }

    // ── PluginLoader ──────────────────────────────────────────────────

    #[test]
    fn test_plugin_loader_new() {
        let loader = PluginLoader::new();
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        let result = loader.load_plugin(manifest);
        assert!(result.success);
    }

    #[test]
    fn test_plugin_loader_with_sandbox_level() {
        let loader = PluginLoader::with_sandbox_level(SandboxLevel::Restricted);
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "P1".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        let result = loader.load_plugin(manifest);
        assert!(result.success);
    }

    #[test]
    fn test_plugin_loader_missing_id() {
        let loader = PluginLoader::new();
        let manifest = PluginManifest {
            id: "".into(),
            name: "P1".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        let result = loader.load_plugin(manifest);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("missing id"));
    }

    #[test]
    fn test_plugin_loader_missing_name() {
        let loader = PluginLoader::new();
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "".into(),
            version: "1.0".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        let result = loader.load_plugin(manifest);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("missing name"));
    }

    #[test]
    fn test_plugin_loader_missing_version() {
        let loader = PluginLoader::new();
        let manifest = PluginManifest {
            id: "p1".into(),
            name: "P1".into(),
            version: "".into(),
            author: "a".into(),
            description: "d".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Tool,
            entry_point: "ep".into(),
            dependencies: vec![],
            config_schema: None,
        };
        let result = loader.load_plugin(manifest);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("missing version"));
    }

    #[test]
    fn test_plugin_loader_load_all() {
        let loader = PluginLoader::new();
        let manifests = vec![
            PluginManifest {
                id: "p1".into(),
                name: "P1".into(),
                version: "1.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Tool,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
            PluginManifest {
                id: "p2".into(),
                name: "P2".into(),
                version: "2.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Workflow,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
        ];
        let results = loader.load_all(manifests);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.success));
    }

    #[test]
    fn test_plugin_loader_load_all_with_failure() {
        let loader = PluginLoader::new();
        let manifests = vec![
            PluginManifest {
                id: "p1".into(),
                name: "P1".into(),
                version: "1.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Tool,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
            PluginManifest {
                id: "".into(),
                name: "P2".into(),
                version: "2.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Workflow,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
        ];
        let results = loader.load_all(manifests);
        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert!(!results[1].success);
    }

    // ── PluginEntry ───────────────────────────────────────────────────

    #[test]
    fn test_plugin_entry_creation() {
        let entry = PluginEntry {
            manifest: PluginManifest {
                id: "p1".into(),
                name: "P1".into(),
                version: "1.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Tool,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
            state: PluginState::Registered,
        };
        assert_eq!(entry.state, PluginState::Registered);
        assert_eq!(entry.manifest.id, "p1");
    }

    #[test]
    fn test_plugin_entry_clone() {
        let entry = PluginEntry {
            manifest: PluginManifest {
                id: "p1".into(),
                name: "P1".into(),
                version: "1.0".into(),
                author: "a".into(),
                description: "d".into(),
                neo_version_req: ">=0.1".into(),
                capabilities: PluginType::Provider,
                entry_point: "ep".into(),
                dependencies: vec![],
                config_schema: None,
            },
            state: PluginState::Active,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.manifest.id, "p1");
        assert_eq!(cloned.state, PluginState::Active);
    }

    // ── Integration: load from manifest then register ─────────────────

    #[test]
    fn test_load_from_manifest_then_register() {
        let reg = PluginRegistry::new();
        let manifest = PluginManifest {
            id: "integrated".into(),
            name: "Integrated Plugin".into(),
            version: "1.0".into(),
            author: "neo".into(),
            description: "tests integration".into(),
            neo_version_req: ">=0.1".into(),
            capabilities: PluginType::Capability,
            entry_point: "lib".into(),
            dependencies: vec![Dependency {
                name: "base".into(),
                version_req: ">=1.0".into(),
                optional: false,
            }],
            config_schema: Some(serde_json::json!({"type": "object"})),
        };
        reg.load_from_manifest(manifest).unwrap();
        let info = reg.get("integrated").unwrap();
        assert_eq!(info.version, "1.0");
        assert!(info.capabilities.custom.is_empty());
    }

    // ── PluginState equality ──────────────────────────────────────────

    #[test]
    fn test_plugin_state_equality() {
        assert_eq!(PluginState::Active, PluginState::Active);
        assert_ne!(PluginState::Active, PluginState::Disabled);
    }

    // ── SandboxLevel equality ─────────────────────────────────────────

    #[test]
    fn test_sandbox_level_equality() {
        assert_eq!(SandboxLevel::None, SandboxLevel::None);
        assert_ne!(SandboxLevel::Basic, SandboxLevel::Full);
    }

    // ── Full capabilities serialization ───────────────────────────────

    #[test]
    fn test_full_plugin_capabilities_serde_roundtrip() {
        let mut caps = PluginCapabilities::default();
        caps.tools.insert(
            "t1".into(),
            ToolCapability {
                name: "t1".into(),
                description: "tool one".into(),
                parameters_schema: Some(serde_json::json!({"type": "object", "properties": {"q": {"type": "string"}}})),
                requires_approval: false,
            },
        );
        caps.workflows.insert(
            "w1".into(),
            WorkflowCapability {
                name: "w1".into(),
                description: "workflow one".into(),
                input_types: vec!["text".into()],
                output_types: vec!["json".into()],
            },
        );
        caps.providers.insert(
            "prov1".into(),
            ProviderCapability {
                name: "prov1".into(),
                supported_models: vec!["model-a".into()],
                features: vec!["streaming".into()],
            },
        );
        caps.prompt_templates.insert("t".into(), "tmpl".into());
        caps.retrievers.insert("r".into(), "ret".into());
        caps.planners.insert("p".into(), "plan".into());
        caps
            .custom
            .insert("x".into(), serde_json::json!([1, 2, 3]));

        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: PluginCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tools.len(), 1);
        assert_eq!(deserialized.workflows.len(), 1);
        assert_eq!(deserialized.providers.len(), 1);
        assert_eq!(deserialized.prompt_templates.len(), 1);
        assert_eq!(deserialized.retrievers.len(), 1);
        assert_eq!(deserialized.planners.len(), 1);
        assert_eq!(deserialized.custom.len(), 1);
    }
}
