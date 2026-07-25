pub mod error;
pub mod routes;

use std::sync::Arc;

use axum::routing::{delete, get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use self::routes::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_enabled: bool,
    pub auth_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 7600,
            cors_enabled: true,
            auth_enabled: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NeoAppState {
    pub conversation_manager: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub metrics_collector: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub health_checker: Option<Arc<dyn std::any::Any + Send + Sync>>,
    pub security_manager: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl Default for NeoAppState {
    fn default() -> Self {
        Self {
            conversation_manager: None,
            metrics_collector: None,
            health_checker: None,
            security_manager: None,
        }
    }
}

#[derive(Debug)]
pub struct NeoRestServer {
    config: ServerConfig,
    state: NeoAppState,
    router: Router,
}

impl NeoRestServer {
    pub fn new() -> Self {
        let config = ServerConfig::default();
        let state = NeoAppState::default();
        let router = create_router(state.clone());
        Self { config, state, router }
    }

    pub fn with_config(config: ServerConfig) -> Self {
        let state = NeoAppState::default();
        let router = create_router(state.clone());
        Self { config, state, router }
    }

    pub fn with_state(state: NeoAppState) -> Self {
        let config = ServerConfig::default();
        let router = create_router(state.clone());
        Self { config, state, router }
    }

    pub fn into_router(self) -> Router {
        self.router
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    pub fn state(&self) -> &NeoAppState {
        &self.state
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        info!("Starting Neo REST server on {}", addr);

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, self.router.clone()).await?;

        Ok(())
    }
}

pub fn create_router(state: NeoAppState) -> Router {
    let mut router = Router::new()
        .route("/health", get(system::health_check_handler))
        .route("/metrics", get(system::metrics_handler))
        .route("/conversation/chat", post(conversation::chat_handler))
        .route("/conversation/stream", post(conversation::stream_handler))
        .route("/conversation/session", post(conversation::create_session_handler))
        .route(
            "/conversation/session/:id",
            get(conversation::get_session_handler)
                .delete(conversation::delete_session_handler),
        )
        .route(
            "/conversation/history/:id",
            get(conversation::get_history_handler),
        )
        .route("/world/entities", get(world::list_entities_handler))
        .route(
            "/world/entity/:id",
            get(world::get_entity_handler)
                .put(world::update_entity_handler)
                .delete(world::delete_entity_handler),
        )
        .route("/world/entity", post(world::create_entity_handler))
        .route("/world/events", get(world::list_events_handler))
        .route("/world/snapshot", get(world::get_snapshot_handler))
        .route("/world/simulate", post(world::simulate_handler))
        .route("/world/predict", post(world::predict_handler))
        .route("/memory/search", get(memory::search_handler))
        .route("/memory/store", post(memory::store_handler))
        .route(
            "/memory/:id",
            delete(memory::delete_handler),
        )
        .route("/memory/statistics", get(memory::statistics_handler))
        .route("/knowledge/entity", get(knowledge::get_entity_handler))
        .route("/knowledge/search", get(knowledge::search_handler))
        .route("/knowledge/graph", get(knowledge::get_graph_handler))
        .route("/knowledge/query", post(knowledge::query_handler))
        .route("/planning/create", post(planning::create_plan_handler))
        .route(
            "/planning/:id",
            get(planning::get_plan_handler)
                .delete(planning::delete_plan_handler),
        )
        .route("/agents", get(agents::list_agents_handler))
        .route("/agents/start", post(agents::start_agent_handler))
        .route("/agents/stop", post(agents::stop_agent_handler))
        .route("/agents/status", get(agents::get_agent_status_handler))
        .route("/workflow/start", post(workflow::start_workflow_handler))
        .route("/workflow/cancel", post(workflow::cancel_workflow_handler))
        .route("/workflow/status", get(workflow::get_workflow_status_handler))
        .with_state(state);

    router = router.layer(TraceLayer::new_for_http());
    router = router.layer(CorsLayer::permissive());

    router
}
