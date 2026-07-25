use neo_inference::context::engine::ContextEngine;
use neo_inference::context::{ContextConfig, ContextId, Message, MessageRole};

fn default_config() -> ContextConfig {
    ContextConfig {
        max_context_tokens: 8192,
        sliding_window_size: 5,
        compression_threshold: 10000,
        enable_compression: false,
        enable_persistence: false,
        persistence_path: None,
    }
}

#[test]
fn test_create_context() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);
    assert_eq!(engine.total_contexts(), 1);
    let ctx = engine.get_context(id).unwrap();
    assert!(ctx.messages.is_empty());
}

#[test]
fn test_add_message_and_get_messages() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);

    engine.add_message(id, Message::user("hi")).unwrap();
    engine.add_message(id, Message::assistant("hello")).unwrap();

    let msgs = engine.get_messages(id).unwrap();
    assert_eq!(msgs.len(), 2);
    assert!(matches!(msgs[0].role, MessageRole::User));
    assert!(matches!(msgs[1].role, MessageRole::Assistant));
    assert_eq!(msgs[0].content, "hi");
    assert_eq!(msgs[1].content, "hello");
}

#[test]
fn test_sliding_window() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);

    for i in 0..10 {
        engine.add_message(id, Message::user(format!("msg-{}", i))).unwrap();
    }

    let msgs = engine.get_messages(id).unwrap();
    assert!(msgs.len() <= 5, "Expected at most 5 messages, got {}", msgs.len());
    assert_eq!(msgs[0].content, "msg-5");
}

#[test]
fn test_clear_context() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);

    engine.add_message(id, Message::user("hello")).unwrap();
    assert_eq!(engine.get_messages(id).unwrap().len(), 1);

    engine.clear_context(id).unwrap();
    assert!(engine.get_messages(id).unwrap().is_empty());
}

#[test]
fn test_delete_context() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);
    assert_eq!(engine.total_contexts(), 1);

    engine.delete_context(id).unwrap();
    assert_eq!(engine.total_contexts(), 0);
    assert!(engine.get_messages(id).is_err());
}

#[test]
fn test_merge_contexts() {
    let engine = ContextEngine::new(default_config());
    let id1 = engine.create_context(None);
    let id2 = engine.create_context(None);

    engine.add_message(id1, Message::user("from ctx1")).unwrap();
    engine.add_message(id2, Message::user("from ctx2")).unwrap();

    let merged_id = engine.merge_contexts(&[id1, id2]).unwrap();
    let msgs = engine.get_messages(merged_id).unwrap();
    assert_eq!(msgs.len(), 2);
}

#[test]
fn test_compress_context() {
    let config = ContextConfig {
        compression_threshold: 5,
        ..default_config()
    };
    let engine = ContextEngine::new(config);
    let id = engine.create_context(None);

    for i in 0..20 {
        engine.add_message(id, Message::user(format!("message {}", i))).unwrap();
    }

    let compressed = engine.compress_context(id).unwrap();
    assert!(compressed.len() < 20);
}

#[test]
fn test_add_message_nonexistent_context() {
    let engine = ContextEngine::new(default_config());
    let result = engine.add_message(ContextId::new(), Message::user("test"));
    assert!(result.is_err());
}

#[test]
fn test_get_messages_nonexistent_context() {
    let engine = ContextEngine::new(default_config());
    let result = engine.get_messages(ContextId::new());
    assert!(result.is_err());
}

#[test]
fn test_create_with_system_prompt() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_with_system_prompt("You are helpful", None);
    let ctx = engine.get_context(id).unwrap();
    assert_eq!(ctx.system_prompt.as_deref(), Some("You are helpful"));
}

#[test]
fn test_list_contexts() {
    let engine = ContextEngine::new(default_config());
    let _id1 = engine.create_context(None);
    let _id2 = engine.create_context(None);
    let _id3 = engine.create_context(None);
    assert_eq!(engine.list_contexts().len(), 3);
}

#[test]
fn test_context_system_and_tool_roles() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);

    engine.add_message(id, Message::system("sys")).unwrap();
    engine.add_message(id, Message::tool("tool output")).unwrap();

    let msgs = engine.get_messages(id).unwrap();
    assert!(matches!(msgs[0].role, MessageRole::System));
    assert!(matches!(msgs[1].role, MessageRole::Tool));
}

#[test]
fn test_multiple_contexts_isolated() {
    let engine = ContextEngine::new(default_config());
    let id1 = engine.create_context(None);
    let id2 = engine.create_context(None);

    engine.add_message(id1, Message::user("ctx1 msg")).unwrap();
    engine.add_message(id2, Message::user("ctx2 msg")).unwrap();

    let msgs1 = engine.get_messages(id1).unwrap();
    let msgs2 = engine.get_messages(id2).unwrap();
    assert_eq!(msgs1.len(), 1);
    assert_eq!(msgs2.len(), 1);
    assert_eq!(msgs1[0].content, "ctx1 msg");
    assert_eq!(msgs2[0].content, "ctx2 msg");
}

#[test]
fn test_sliding_window_trim() {
    let engine = ContextEngine::new(default_config());
    let id = engine.create_context(None);

    for i in 0..8 {
        engine.add_message(id, Message::user(format!("msg-{}", i))).unwrap();
    }
    engine.sliding_window_trim(id).unwrap();

    let msgs = engine.get_messages(id).unwrap();
    assert!(msgs.len() <= 5);
}
