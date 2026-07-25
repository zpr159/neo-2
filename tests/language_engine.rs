use neo_core::language::*;

#[tokio::test]
async fn test_token_estimator() {
    let estimator = TokenEstimator::new();
    let tokens = estimator.estimate("Hello, world! This is a test.");
    assert!(tokens > 0);
    assert!(tokens < 20);
}

#[tokio::test]
async fn test_token_estimator_messages() {
    let estimator = TokenEstimator::new();
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("What is 2+2?"),
        Message::assistant("4"),
    ];
    let tokens = estimator.estimate_messages(&messages);
    assert!(tokens > 0);
}

#[tokio::test]
async fn test_token_counter() {
    let counter = TokenCounter::new();
    let usage = TokenUsage {
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
    };
    counter.record("session-1", &usage).await;
    counter.record("session-1", &usage).await;

    let total = counter.total_usage();
    assert_eq!(total.prompt_tokens, 200);
    assert_eq!(total.completion_tokens, 100);
    assert_eq!(total.total_tokens, 300);
    assert_eq!(counter.request_count(), 2);

    let session = counter.session_usage("session-1").await;
    assert_eq!(session.total_tokens, 300);
}

#[tokio::test]
async fn test_generation_config() {
    let config = GenerationConfig::default();
    assert_eq!(config.max_tokens, 2048);
    assert!((config.temperature - 0.7).abs() < f32::EPSILON);
    assert!((config.top_p - 1.0).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_generation_response() {
    let response = GenerationResponse::new("test-provider", "test-model");
    assert_eq!(response.provider, "test-provider");
    assert_eq!(response.model, "test-model");
    assert!(response.text.is_empty());
    assert_eq!(response.finish_reason, FinishReason::Stop);
}

#[tokio::test]
async fn test_stream_chunk() {
    let chunk = StreamChunk::new("hello", "hello world", 1);
    assert_eq!(chunk.token, "hello");
    assert_eq!(chunk.accumulated, "hello world");
    assert!(!chunk.finished);

    let done = StreamChunk::done("hello world", 2, FinishReason::Stop);
    assert!(done.finished);
    assert_eq!(done.finish_reason, Some(FinishReason::Stop));
}

#[tokio::test]
async fn test_cancellation_token() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    token.cancel();
    assert!(token.is_cancelled());
}

#[tokio::test]
async fn test_provider_health() {
    let healthy = ProviderHealth::healthy();
    assert!(healthy.healthy);
    assert!(healthy.message.is_none());

    let unhealthy = ProviderHealth::unhealthy("test error");
    assert!(!unhealthy.healthy);
    assert_eq!(unhealthy.message.unwrap(), "test error");
}

#[tokio::test]
async fn test_provider_config() {
    let config = ProviderConfig::default();
    assert_eq!(config.provider_type, ProviderType::Ollama);
    assert_eq!(config.endpoint, "http://localhost:11434");
    assert!(config.enabled);
}

#[tokio::test]
async fn test_language_engine_config() {
    let config = LanguageEngineConfig::default();
    assert_eq!(config.active_provider, "default");
    assert_eq!(config.retry_count, 3);
}

#[tokio::test]
async fn test_language_engine_config_ollama() {
    let config = LanguageEngineConfig::ollama_default();
    assert_eq!(config.providers.len(), 1);
    assert_eq!(config.providers[0].provider_type, ProviderType::Ollama);
    assert_eq!(
        config.providers[0].endpoint,
        "http://localhost:11434"
    );
}

#[tokio::test]
async fn test_language_error_codes() {
    let err = LanguageError::ConnectionFailed("test".to_string());
    assert_eq!(err.code(), LanguageErrorCode::ConnectionFailed);
    assert!(err.is_retriable());

    let err = LanguageError::AuthenticationFailed("test".to_string());
    assert_eq!(err.code(), LanguageErrorCode::AuthenticationFailed);
    assert!(err.is_fatal());

    let err = LanguageError::ContextTooLarge {
        provided: 100000,
        maximum: 4096,
    };
    assert_eq!(err.code(), LanguageErrorCode::ContextTooLarge);
    assert!(!err.is_retriable());
}

#[tokio::test]
async fn test_message_roles() {
    assert_eq!(MessageRole::System.to_string(), "system");
    assert_eq!(MessageRole::User.to_string(), "user");
    assert_eq!(MessageRole::Assistant.to_string(), "assistant");
    assert_eq!(MessageRole::Tool.to_string(), "tool");
}

#[tokio::test]
async fn test_message_constructors() {
    let sys = Message::system("test system");
    assert_eq!(sys.role, MessageRole::System);
    assert_eq!(sys.content, "test system");

    let user = Message::user("test user");
    assert_eq!(user.role, MessageRole::User);

    let assistant = Message::assistant("test assistant");
    assert_eq!(assistant.role, MessageRole::Assistant);

    let tool = Message::tool("tool result", "call-123");
    assert_eq!(tool.role, MessageRole::Tool);
    assert_eq!(tool.tool_call_id.unwrap(), "call-123");
}

#[tokio::test]
async fn test_model_info() {
    let model = ModelInfo {
        name: "test-model".to_string(),
        display_name: Some("Test Model".to_string()),
        version: Some("1.0".to_string()),
        context_length: Some(4096),
        max_output_tokens: Some(2048),
        quantization: Some("Q4_K_M".to_string()),
        parameter_count: Some("7B".to_string()),
        license: Some("MIT".to_string()),
        memory_requirements: Some("4.5 GB".to_string()),
        capabilities: ModelCapabilities::default(),
    };
    assert_eq!(model.name, "test-model");
}

#[tokio::test]
async fn test_model_state() {
    assert_eq!(ModelState::Unloaded.to_string(), "unloaded");
    assert_eq!(ModelState::Loaded.to_string(), "loaded");
    assert_eq!(ModelState::Warm.to_string(), "warm");
}

#[tokio::test]
async fn test_finish_reason() {
    assert_eq!(FinishReason::Stop.to_string(), "stop");
    assert_eq!(FinishReason::Length.to_string(), "length");
    assert_eq!(FinishReason::ToolCalls.to_string(), "tool_calls");
}

#[tokio::test]
async fn test_token_usage() {
    let usage = TokenUsage::default();
    assert_eq!(usage.prompt_tokens, 0);
    assert_eq!(usage.completion_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}

#[tokio::test]
async fn test_provider_capabilities() {
    let caps = ProviderCapabilities::default();
    assert!(!caps.streaming);
    assert!(!caps.function_calling);
    assert_eq!(caps.max_context, 0);
}

#[tokio::test]
async fn test_provider_metrics() {
    let metrics = ProviderMetrics::default();
    assert_eq!(metrics.request_latency_ms, 0.0);
    assert_eq!(metrics.total_tokens_generated, 0);
    assert_eq!(metrics.active_requests, 0);
}

#[tokio::test]
async fn test_model_capabilities() {
    let caps = ModelCapabilities::default();
    assert!(!caps.supports_streaming);
    assert!(!caps.supports_vision);
    assert!(!caps.supports_offline);
}

#[tokio::test]
async fn test_message_serialization() {
    let msg = Message::user("test message");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("user"));
    assert!(json.contains("test message"));

    let deserialized: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.role, MessageRole::User);
    assert_eq!(deserialized.content, "test message");
}

#[tokio::test]
async fn test_generation_config_serialization() {
    let config = GenerationConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: GenerationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.max_tokens, config.max_tokens);
    assert!((deserialized.temperature - config.temperature).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_language_engine_config_serialization() {
    let config = LanguageEngineConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: LanguageEngineConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.active_provider, config.active_provider);
    assert_eq!(deserialized.retry_count, config.retry_count);
}

#[tokio::test]
async fn test_provider_registry() {
    let registry = ProviderRegistry::new();
    assert!(registry.list_descriptors().await.is_empty());
    assert!(!registry.is_registered(&ProviderType::Ollama).await);
}

#[tokio::test]
async fn test_metrics_collector() {
    let collector = MetricsCollector::new();
    let snapshot = collector.snapshot().await;
    assert_eq!(snapshot.total_requests, 0);
}

#[tokio::test]
async fn test_token_estimator_ratio() {
    let estimator = TokenEstimator::with_ratio(2.0);
    let tokens = estimator.estimate("one two three four five");
    assert!(tokens > 0);
}
