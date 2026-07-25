//! Neo AGI OS — Shared test utilities and fixtures.

use std::collections::HashMap;
use std::path::PathBuf;

/// Test context providing a temporary directory and default configuration.
pub struct TestContext {
    pub temp_dir: PathBuf,
    pub config: serde_json::Value,
}

impl TestContext {
    pub fn new() -> Self {
        let temp_dir = tempfile::tempdir()
            .expect("failed to create temp dir")
            .into_path();
        let config = create_test_config();
        Self { temp_dir, config }
    }

    pub fn temp_path(&self, name: &str) -> PathBuf {
        self.temp_dir.join(name)
    }
}

/// A mock agent for testing.
pub struct MockAgent {
    pub id: String,
    pub name: String,
    pub state: String,
    messages: Vec<serde_json::Value>,
}

impl MockAgent {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            state: "stopped".to_string(),
            messages: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        self.state = "running".to_string();
    }

    pub fn stop(&mut self) {
        self.state = "stopped".to_string();
    }

    pub fn send(&mut self, msg: serde_json::Value) {
        self.messages.push(msg);
    }

    pub fn receive(&mut self) -> Option<serde_json::Value> {
        self.messages.pop()
    }
}

/// A mock tool for testing.
pub struct MockTool {
    pub id: String,
    pub name: String,
}

impl MockTool {
    pub fn new(name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
        }
    }

    pub fn execute(&self, params: &serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "status": "ok",
            "params": params
        })
    }
}

/// A mock in-memory store for testing.
pub struct MockMemory {
    entries: HashMap<String, serde_json::Value>,
}

impl MockMemory {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn store(&mut self, content: serde_json::Value) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        self.entries.insert(id.clone(), content);
        id
    }

    pub fn recall(&self, id: &str) -> Option<&serde_json::Value> {
        self.entries.get(id)
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

/// Assert that an error message contains expected text.
pub fn assert_neo_error(result: Result<(), String>, expected: &str) {
    match result {
        Ok(()) => panic!("Expected error containing '{}', got Ok(())", expected),
        Err(e) => assert!(
            e.contains(expected),
            "Expected error containing '{}', got: {}",
            expected,
            e
        ),
    }
}

/// Create a default test configuration.
pub fn create_test_config() -> serde_json::Value {
    serde_json::json!({
        "name": "neo-test-config",
        "version": "0.1.0",
        "debug": true,
        "log_level": "debug"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = TestContext::new();
        assert!(ctx.temp_dir.exists());
        assert_eq!(ctx.config["name"], "neo-test-config");
    }

    #[test]
    fn test_mock_agent() {
        let mut agent = MockAgent::new("test");
        assert_eq!(agent.name, "test");
        assert_eq!(agent.state, "stopped");
        agent.start();
        assert_eq!(agent.state, "running");
    }

    #[test]
    fn test_mock_tool() {
        let tool = MockTool::new("calc");
        assert_eq!(tool.name, "calc");
        let result = tool.execute(&serde_json::json!({"op": "add"}));
        assert_eq!(result["status"], "ok");
    }

    #[test]
    fn test_mock_memory() {
        let mut mem = MockMemory::new();
        let id = mem.store(serde_json::json!({"text": "hello"}));
        assert_eq!(mem.count(), 1);
        assert!(mem.recall(&id).is_some());
    }
}
