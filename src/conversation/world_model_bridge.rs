use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::conversation::error::ConversationResult;
use crate::conversation::evidence::Evidence;
use crate::conversation::types::ConversationContext;

/// Current state of the world model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub entities: Vec<WorldEntity>,
    pub temporal_events: Vec<TemporalEvent>,
    pub causal_chains: Vec<CausalChain>,
    pub predictions: Vec<Prediction>,
    pub timestamp: crate::time::Timestamp,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub state: std::collections::HashMap<String, String>,
    pub location: Option<String>,
    pub last_updated: crate::time::Timestamp,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalEvent {
    pub id: String,
    pub description: String,
    pub event_type: String,
    pub start_time: crate::time::Timestamp,
    pub end_time: Option<crate::time::Timestamp>,
    pub participants: Vec<String>,
    pub significance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    pub id: String,
    pub events: Vec<String>,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub description: String,
    pub probability: f32,
    pub time_horizon_secs: u64,
    pub based_on: Vec<String>,
}

/// Simulation request and result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub scenario: String,
    pub initial_state: std::collections::HashMap<String, String>,
    pub steps: usize,
    pub parameters: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub steps: Vec<SimulationStep>,
    pub final_state: std::collections::HashMap<String, String>,
    pub confidence: f32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationStep {
    pub step: usize,
    pub state: std::collections::HashMap<String, String>,
    pub events: Vec<String>,
}

/// Bridge between the World Model subsystem and the Conversation layer.
#[async_trait]
pub trait WorldModelConversationBridge: Send + Sync {
    /// Get the current world state.
    async fn get_world_state(
        &self,
        context: &ConversationContext,
    ) -> ConversationResult<WorldState>;

    /// Get active entities.
    async fn get_active_entities(
        &self,
        context: &ConversationContext,
    ) -> ConversationResult<Vec<WorldEntity>>;

    /// Get entities at a location.
    async fn get_entities_at_location(
        &self,
        context: &ConversationContext,
        location: &str,
    ) -> ConversationResult<Vec<WorldEntity>>;

    /// Get temporal events.
    async fn get_temporal_events(
        &self,
        context: &ConversationContext,
        time_range: Option<(crate::time::Timestamp, crate::time::Timestamp)>,
    ) -> ConversationResult<Vec<TemporalEvent>>;

    /// Get causal chains.
    async fn get_causal_chains(
        &self,
        context: &ConversationContext,
    ) -> ConversationResult<Vec<CausalChain>>;

    /// Get predictions.
    async fn get_predictions(
        &self,
        context: &ConversationContext,
        time_horizon_secs: u64,
    ) -> ConversationResult<Vec<Prediction>>;

    /// Run a simulation.
    async fn simulate(
        &self,
        context: &ConversationContext,
        request: &SimulationRequest,
    ) -> ConversationResult<SimulationResult>;

    /// Query the world model, returned as evidence.
    async fn query_evidence(
        &self,
        context: &ConversationContext,
        query: &str,
    ) -> ConversationResult<Vec<Evidence>>;
}

/// Mock implementation for testing.
pub struct MockWorldModelBridge;

#[async_trait]
impl WorldModelConversationBridge for MockWorldModelBridge {
    async fn get_world_state(
        &self,
        _context: &ConversationContext,
    ) -> ConversationResult<WorldState> {
        Ok(WorldState {
            entities: Vec::new(),
            temporal_events: Vec::new(),
            causal_chains: Vec::new(),
            predictions: Vec::new(),
            timestamp: crate::time::Timestamp::now(),
            version: 0,
        })
    }

    async fn get_active_entities(
        &self,
        _context: &ConversationContext,
    ) -> ConversationResult<Vec<WorldEntity>> {
        Ok(Vec::new())
    }

    async fn get_entities_at_location(
        &self,
        _context: &ConversationContext,
        _location: &str,
    ) -> ConversationResult<Vec<WorldEntity>> {
        Ok(Vec::new())
    }

    async fn get_temporal_events(
        &self,
        _context: &ConversationContext,
        _time_range: Option<(crate::time::Timestamp, crate::time::Timestamp)>,
    ) -> ConversationResult<Vec<TemporalEvent>> {
        Ok(Vec::new())
    }

    async fn get_causal_chains(
        &self,
        _context: &ConversationContext,
    ) -> ConversationResult<Vec<CausalChain>> {
        Ok(Vec::new())
    }

    async fn get_predictions(
        &self,
        _context: &ConversationContext,
        _time_horizon_secs: u64,
    ) -> ConversationResult<Vec<Prediction>> {
        Ok(Vec::new())
    }

    async fn simulate(
        &self,
        _context: &ConversationContext,
        _request: &SimulationRequest,
    ) -> ConversationResult<SimulationResult> {
        Ok(SimulationResult {
            steps: Vec::new(),
            final_state: std::collections::HashMap::new(),
            confidence: 0.0,
            summary: "Mock simulation".to_string(),
        })
    }

    async fn query_evidence(
        &self,
        _context: &ConversationContext,
        _query: &str,
    ) -> ConversationResult<Vec<Evidence>> {
        Ok(Vec::new())
    }
}
