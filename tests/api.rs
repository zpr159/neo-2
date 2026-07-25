#[cfg(test)]
mod tests {
    use neo_core::api::*;
    use std::collections::HashMap;

    // ── PaginationParams ──────────────────────────────────────────────

    #[test]
    fn test_pagination_params_default() {
        let p = PaginationParams::default();
        assert_eq!(p.offset, 0);
        assert_eq!(p.limit, 50);
    }

    #[test]
    fn test_pagination_params_custom() {
        let p = PaginationParams { offset: 10, limit: 25 };
        assert_eq!(p.offset, 10);
        assert_eq!(p.limit, 25);
    }

    // ── PaginatedResponse ─────────────────────────────────────────────

    #[test]
    fn test_paginated_response_serde_roundtrip_strings() {
        let resp = PaginatedResponse {
            items: vec!["a".to_string(), "b".to_string()],
            total: 100,
            offset: 0,
            limit: 50,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: PaginatedResponse<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.items, vec!["a".to_string(), "b".to_string()]);
        assert_eq!(deserialized.total, 100);
        assert_eq!(deserialized.offset, 0);
        assert_eq!(deserialized.limit, 50);
    }

    #[test]
    fn test_paginated_response_empty() {
        let resp = PaginatedResponse::<i32> {
            items: vec![],
            total: 0,
            offset: 0,
            limit: 10,
        };
        assert!(resp.items.is_empty());
        assert_eq!(resp.total, 0);
    }

    #[test]
    fn test_paginated_response_serde_roundtrip_integers() {
        let resp = PaginatedResponse {
            items: vec![1, 2, 3],
            total: 3,
            offset: 0,
            limit: 10,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: PaginatedResponse<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.items, vec![1, 2, 3]);
    }

    // ── ApiError ──────────────────────────────────────────────────────

    #[test]
    fn test_api_error_display() {
        let err = ApiError { code: 404, message: "not found".into(), details: None };
        assert_eq!(format!("{err}"), "[404] not found");
    }

    #[test]
    fn test_api_error_display_with_details() {
        let err = ApiError { code: 500, message: "internal".into(), details: Some("stack trace".into()) };
        assert_eq!(format!("{err}"), "[500] internal");
    }

    #[test]
    fn test_api_error_is_std_error() {
        let err = ApiError { code: 403, message: "forbidden".into(), details: None };
        let e: &dyn std::error::Error = &err;
        assert!(e.source().is_none());
    }

    #[test]
    fn test_api_error_serde_roundtrip() {
        let err = ApiError { code: 422, message: "bad request".into(), details: Some("field missing".into()) };
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.code, 422);
        assert_eq!(deserialized.message, "bad request");
        assert_eq!(deserialized.details, Some("field missing".into()));
    }

    // ── HealthStatus / SubsystemHealth ────────────────────────────────

    #[test]
    fn test_health_status_serde_roundtrip() {
        let mut subsystems = HashMap::new();
        subsystems.insert(
            "core".to_string(),
            SubsystemHealth {
                healthy: true,
                latency_ms: Some(1.5),
                message: None,
            },
        );
        let hs = HealthStatus {
            status: "healthy".into(),
            version: "1.0.0".into(),
            uptime_secs: 3600,
            subsystems,
        };
        let json = serde_json::to_string(&hs).unwrap();
        let deserialized: HealthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, "healthy");
        assert_eq!(deserialized.version, "1.0.0");
        assert_eq!(deserialized.uptime_secs, 3600);
        assert!(deserialized.subsystems.contains_key("core"));
    }

    #[test]
    fn test_subsystem_health_degraded() {
        let sh = SubsystemHealth {
            healthy: false,
            latency_ms: None,
            message: Some("high latency".into()),
        };
        let json = serde_json::to_string(&sh).unwrap();
        let deserialized: SubsystemHealth = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.healthy);
        assert!(deserialized.latency_ms.is_none());
        assert_eq!(deserialized.message, Some("high latency".into()));
    }

    // ── Conversation types ────────────────────────────────────────────

    #[test]
    fn test_chat_request_serde_roundtrip() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "value".to_string());
        let req = ChatRequest {
            session_id: Some("s1".into()),
            conversation_id: Some("c1".into()),
            message: "hello".into(),
            stream: false,
            metadata: meta.clone(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message, "hello");
        assert!(!deserialized.stream);
        assert_eq!(deserialized.metadata.get("key").unwrap(), "value");
    }

    #[test]
    fn test_chat_response_serde_roundtrip() {
        let resp = ChatResponse {
            conversation_id: "c1".into(),
            session_id: "s1".into(),
            message: "world".into(),
            tool_calls: None,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: ChatResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.conversation_id, "c1");
        assert!(deserialized.tool_calls.is_none());
    }

    #[test]
    fn test_session_info_serde_roundtrip() {
        let si = SessionInfo {
            session_id: "sess-1".into(),
            conversation_ids: vec!["c1".into(), "c2".into()],
            active_conversation: Some("c1".into()),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&si).unwrap();
        let deserialized: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.conversation_ids.len(), 2);
    }

    #[test]
    fn test_create_session_request_serde_roundtrip() {
        let req = CreateSessionRequest { user_id: Some("u1".into()) };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CreateSessionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.user_id, Some("u1".into()));
    }

    #[test]
    fn test_history_entry_serde_roundtrip() {
        let he = HistoryEntry {
            id: "h1".into(),
            role: "user".into(),
            content: "hi".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&he).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "user");
    }

    // ── World Model types ─────────────────────────────────────────────

    #[test]
    fn test_world_entity_serde_roundtrip() {
        let mut props = HashMap::new();
        props.insert("age".to_string(), serde_json::json!(30));
        let entity = WorldEntity {
            id: "e1".into(),
            entity_type: "person".into(),
            name: "Alice".into(),
            properties: props,
            relationships: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-02T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: WorldEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "e1");
        assert_eq!(deserialized.properties["age"], serde_json::json!(30));
    }

    #[test]
    fn test_entity_relationship_serde_roundtrip() {
        let rel = EntityRelationship {
            target_id: "t1".into(),
            relationship_type: "knows".into(),
            weight: 0.8,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&rel).unwrap();
        let deserialized: EntityRelationship = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.weight, 0.8);
    }

    #[test]
    fn test_world_event_serde_roundtrip() {
        let evt = WorldEvent {
            id: "ev1".into(),
            event_type: "created".into(),
            entity_id: Some("e1".into()),
            timestamp: "2026-01-01T00:00:00Z".into(),
            data: serde_json::json!({"key": "val"}),
        };
        let json = serde_json::to_string(&evt).unwrap();
        let deserialized: WorldEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_type, "created");
    }

    #[test]
    fn test_world_snapshot_serde_roundtrip() {
        let snap = WorldSnapshot {
            entities: vec![],
            events: vec![],
            captured_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: WorldSnapshot = serde_json::from_str(&json).unwrap();
        assert!(deserialized.entities.is_empty());
    }

    #[test]
    fn test_prediction_request_serde_roundtrip() {
        let req = PredictionRequest {
            entity_id: "e1".into(),
            horizon_steps: 5,
            factors: vec!["trend".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: PredictionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.horizon_steps, 5);
    }

    #[test]
    fn test_prediction_result_serde_roundtrip() {
        let res = PredictionResult {
            entity_id: "e1".into(),
            predictions: vec![PredictionEntry {
                step: 1,
                value: serde_json::json!(42),
                confidence: 0.9,
            }],
            confidence: 0.85,
        };
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: PredictionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.predictions.len(), 1);
        assert_eq!(deserialized.confidence, 0.85);
    }

    #[test]
    fn test_simulation_request_serde_roundtrip() {
        let req = SimulationRequest {
            initial_state: WorldSnapshot { entities: vec![], events: vec![], captured_at: "t".into() },
            steps: 10,
            parameters: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: SimulationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.steps, 10);
    }

    #[test]
    fn test_simulation_result_serde_roundtrip() {
        let res = SimulationResult {
            final_state: WorldSnapshot { entities: vec![], events: vec![], captured_at: "t".into() },
            trajectory: vec![],
            metrics: HashMap::new(),
        };
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: SimulationResult = serde_json::from_str(&json).unwrap();
        assert!(deserialized.trajectory.is_empty());
    }

    // ── Memory types ──────────────────────────────────────────────────

    #[test]
    fn test_memory_search_request_serde_roundtrip() {
        let req = MemorySearchRequest {
            query: "find me".into(),
            memory_type: Some("episodic".into()),
            limit: 10,
            min_relevance: 0.5,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: MemorySearchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.query, "find me");
    }

    #[test]
    fn test_memory_search_result_serde_roundtrip() {
        let res = MemorySearchResult {
            id: "m1".into(),
            content: "remember this".into(),
            memory_type: "episodic".into(),
            relevance: 0.95,
            created_at: "2026-01-01T00:00:00Z".into(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: MemorySearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.relevance, 0.95);
    }

    #[test]
    fn test_memory_store_request_serde_roundtrip() {
        let req = MemoryStoreRequest {
            content: "store this".into(),
            memory_type: "semantic".into(),
            metadata: HashMap::new(),
            importance: 0.8,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: MemoryStoreRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.importance, 0.8);
    }

    #[test]
    fn test_memory_statistics_serde_roundtrip() {
        let mut by_type = HashMap::new();
        by_type.insert("episodic".to_string(), 5);
        let stats = MemoryStatistics {
            total_memories: 100,
            memories_by_type: by_type,
            total_size_bytes: 4096,
            oldest_memory: Some("2025-01-01".into()),
            newest_memory: Some("2026-01-01".into()),
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: MemoryStatistics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_memories, 100);
    }

    // ── Knowledge types ───────────────────────────────────────────────

    #[test]
    fn test_knowledge_entity_serde_roundtrip() {
        let ent = KnowledgeEntity {
            id: "ke1".into(),
            entity_type: "concept".into(),
            label: "Rust".into(),
            properties: HashMap::new(),
        };
        let json = serde_json::to_string(&ent).unwrap();
        let deserialized: KnowledgeEntity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.label, "Rust");
    }

    #[test]
    fn test_knowledge_edge_serde_roundtrip() {
        let edge = KnowledgeEdge {
            id: "edge1".into(),
            source_id: "s1".into(),
            target_id: "t1".into(),
            relationship: "depends_on".into(),
            weight: 0.7,
            properties: HashMap::new(),
        };
        let json = serde_json::to_string(&edge).unwrap();
        let deserialized: KnowledgeEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.relationship, "depends_on");
    }

    #[test]
    fn test_knowledge_graph_serde_roundtrip() {
        let graph = KnowledgeGraph {
            entities: vec![],
            edges: vec![],
        };
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: KnowledgeGraph = serde_json::from_str(&json).unwrap();
        assert!(deserialized.entities.is_empty());
    }

    #[test]
    fn test_knowledge_query_request_serde_roundtrip() {
        let req = KnowledgeQueryRequest {
            query: "what is Rust".into(),
            query_type: "semantic".into(),
            max_depth: Some(3),
            limit: Some(10),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: KnowledgeQueryRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_depth, Some(3));
    }

    #[test]
    fn test_knowledge_search_result_serde_roundtrip() {
        let res = KnowledgeSearchResult {
            entities: vec![],
            edges: vec![],
            relevance: 0.9,
        };
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: KnowledgeSearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.relevance, 0.9);
    }

    // ── Planning types ────────────────────────────────────────────────

    #[test]
    fn test_plan_task_serde_roundtrip() {
        let task = PlanTask {
            id: "t1".into(),
            description: "do something".into(),
            dependencies: vec!["t0".into()],
            estimated_cost: 1.5,
            status: "pending".into(),
        };
        let json = serde_json::to_string(&task).unwrap();
        let deserialized: PlanTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.status, "pending");
    }

    #[test]
    fn test_plan_serde_roundtrip() {
        let plan = Plan {
            id: "plan-1".into(),
            goal: "accomplish mission".into(),
            tasks: vec![PlanTask {
                id: "t1".into(),
                description: "step 1".into(),
                dependencies: vec![],
                estimated_cost: 1.0,
                status: "done".into(),
            }],
            total_cost: 1.0,
            status: "active".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&plan).unwrap();
        let deserialized: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tasks.len(), 1);
        assert_eq!(deserialized.total_cost, 1.0);
    }

    #[test]
    fn test_create_plan_request_serde_roundtrip() {
        let req = CreatePlanRequest {
            goal: "build system".into(),
            constraints: vec!["budget".into()],
            max_depth: Some(5),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CreatePlanRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.constraints.len(), 1);
    }

    // ── Reasoning types ───────────────────────────────────────────────

    #[test]
    fn test_reasoning_step_serde_roundtrip() {
        let step = ReasoningStep {
            step_type: "deduction".into(),
            input: "all men are mortal".into(),
            output: "socrates is mortal".into(),
            confidence: 0.95,
        };
        let json = serde_json::to_string(&step).unwrap();
        let deserialized: ReasoningStep = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.step_type, "deduction");
    }

    #[test]
    fn test_reasoning_result_serde_roundtrip() {
        let res = ReasoningResult {
            conclusion: "therefore X".into(),
            steps: vec![],
            confidence: 0.88,
            contradictions: vec!["contradiction 1".into()],
        };
        let json = serde_json::to_string(&res).unwrap();
        let deserialized: ReasoningResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.contradictions.len(), 1);
    }

    #[test]
    fn test_reasoning_request_serde_roundtrip() {
        let req = ReasoningRequest {
            query: "why is the sky blue".into(),
            depth: "deep".into(),
            context: vec!["physics".into()],
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ReasoningRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.depth, "deep");
    }

    // ── Workflow types ────────────────────────────────────────────────

    #[test]
    fn test_workflow_info_serde_roundtrip() {
        let info = WorkflowInfo {
            id: "wf1".into(),
            name: "deploy".into(),
            status: "running".into(),
            steps_completed: 3,
            total_steps: 5,
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: WorkflowInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.steps_completed, 3);
    }

    #[test]
    fn test_workflow_status_serde_roundtrip() {
        let status = WorkflowStatus {
            id: "wf1".into(),
            status: "completed".into(),
            progress: 1.0,
            current_step: None,
            error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: WorkflowStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.progress, 1.0);
    }

    #[test]
    fn test_workflow_status_with_error() {
        let status = WorkflowStatus {
            id: "wf2".into(),
            status: "failed".into(),
            progress: 0.5,
            current_step: Some("step3".into()),
            error: Some("timeout".into()),
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: WorkflowStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error, Some("timeout".into()));
    }

    // ── Agent types ───────────────────────────────────────────────────

    #[test]
    fn test_agent_info_serde_roundtrip() {
        let info = AgentInfo {
            id: "agent1".into(),
            name: "Research Agent".into(),
            status: "active".into(),
            capabilities: vec!["search".into(), "summarize".into()],
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: AgentInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.capabilities.len(), 2);
    }

    #[test]
    fn test_agent_status_detail_serde_roundtrip() {
        let detail = AgentStatusDetail {
            id: "agent1".into(),
            status: "idle".into(),
            current_task: None,
            tasks_completed: 42,
            uptime_secs: 86400,
        };
        let json = serde_json::to_string(&detail).unwrap();
        let deserialized: AgentStatusDetail = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tasks_completed, 42);
    }

    // ── Language types ────────────────────────────────────────────────

    #[test]
    fn test_provider_status_serde_roundtrip() {
        let ps = ProviderStatus {
            name: "openai".into(),
            healthy: true,
            models_loaded: vec!["gpt-4".into()],
            latency_ms: Some(12.5),
        };
        let json = serde_json::to_string(&ps).unwrap();
        let deserialized: ProviderStatus = serde_json::from_str(&json).unwrap();
        assert!(deserialized.healthy);
        assert_eq!(deserialized.models_loaded.len(), 1);
    }

    // ── Edge cases ────────────────────────────────────────────────────

    #[test]
    fn test_api_version_constant() {
        assert_eq!(API_VERSION, "v1");
    }

    #[test]
    fn test_empty_chat_request() {
        let req = ChatRequest {
            session_id: None,
            conversation_id: None,
            message: String::new(),
            stream: false,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
        assert!(deserialized.message.is_empty());
        assert!(deserialized.session_id.is_none());
    }

    #[test]
    fn test_world_entity_with_empty_relationships() {
        let entity = WorldEntity {
            id: "e1".into(),
            entity_type: "node".into(),
            name: "isolated".into(),
            properties: HashMap::new(),
            relationships: vec![],
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: WorldEntity = serde_json::from_str(&json).unwrap();
        assert!(deserialized.relationships.is_empty());
    }
}
