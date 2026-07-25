pub mod error;
pub mod types;
pub mod conversation;
pub mod world;
pub mod memory;
pub mod knowledge;
pub mod planning;
pub mod workflow;
pub mod agent;

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};

pub use error::{SdkError, SdkResult};
pub use conversation::ConversationClient;
pub use world::WorldClient;
pub use memory::MemoryClient;
pub use knowledge::KnowledgeClient;
pub use planning::PlanningClient;
pub use workflow::WorkflowClient;
pub use agent::AgentClient;

#[derive(Clone)]
pub struct NeoClient {
    base_url: String,
    client: reqwest::Client,
}

impl NeoClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let url = base_url.into();
        let base_url = url.trim_end_matches('/').to_string();
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        let builder = reqwest::Client::builder().timeout(timeout);
        self.client = builder.build().expect("failed to build reqwest client");
        self
    }

    pub fn with_headers(mut self, headers: HeaderMap<HeaderValue>) -> Self {
        let builder = reqwest::Client::builder().default_headers(headers);
        self.client = builder.build().expect("failed to build reqwest client");
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn conversation(&self) -> ConversationClient {
        ConversationClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn world(&self) -> WorldClient {
        WorldClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn memory(&self) -> MemoryClient {
        MemoryClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn knowledge(&self) -> KnowledgeClient {
        KnowledgeClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn planning(&self) -> PlanningClient {
        PlanningClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn workflow(&self) -> WorkflowClient {
        WorkflowClient::new(self.base_url.clone(), self.client.clone())
    }

    pub fn agent(&self) -> AgentClient {
        AgentClient::new(self.base_url.clone(), self.client.clone())
    }
}
