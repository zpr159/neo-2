use neo_agents::{
    AgentConfiguration, AgentManager, AgentMessage, AgentRole, MessageChannel,
    MessageChannelRegistry, MessagePriority, MessageType,
};
use tokio::sync::mpsc;

#[tokio::test]
async fn direct_message_delivery() {
    let registry = MessageChannelRegistry::new(64);
    let (tx, mut rx) = mpsc::channel::<AgentMessage>(64);
    let from = neo_agents::AgentId::new();
    let to = neo_agents::AgentId::new();
    registry.register_inbox(to, tx);

    let msg = AgentMessage::new(from, to, MessageType::Request, serde_json::json!("hello"));
    registry.send_direct(msg).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.payload, serde_json::json!("hello"));
}

#[tokio::test]
async fn broadcast_delivery() {
    let registry = MessageChannelRegistry::new(64);
    let (_tx1, _rx1) = mpsc::channel::<AgentMessage>(64);
    let (_tx2, _rx2) = mpsc::channel::<AgentMessage>(64);
    let sub1 = neo_agents::AgentId::new();
    let sub2 = neo_agents::AgentId::new();

    let broadcaster = registry.get_or_create_broadcast("alerts");

    let mut sub_rx1 = broadcaster.subscribe();
    let mut sub_rx2 = broadcaster.subscribe();

    let msg = AgentMessage::new(
        sub1,
        sub1,
        MessageType::Notification,
        serde_json::json!("alert!"),
    );
    broadcaster.send(msg).unwrap();

    let r1 = sub_rx1.recv().await.unwrap();
    let r2 = sub_rx2.recv().await.unwrap();
    assert_eq!(r1.payload, serde_json::json!("alert!"));
    assert_eq!(r2.payload, serde_json::json!("alert!"));
}

#[tokio::test]
async fn message_channel_buffering() {
    let ch = MessageChannel::new(3);
    let from = neo_agents::AgentId::new();
    let to = neo_agents::AgentId::new();

    ch.send(AgentMessage::new(
        from,
        to,
        MessageType::Request,
        serde_json::json!(1),
    ))
    .await
    .unwrap();
    ch.send(AgentMessage::new(
        from,
        to,
        MessageType::Request,
        serde_json::json!(2),
    ))
    .await
    .unwrap();
    ch.send(AgentMessage::new(
        from,
        to,
        MessageType::Request,
        serde_json::json!(3),
    ))
    .await
    .unwrap();

    let result = ch
        .send(AgentMessage::new(
            from,
            to,
            MessageType::Request,
            serde_json::json!(4),
        ))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn message_queue_priority() {
    let q = neo_agents::MessageQueue::new();
    let from = neo_agents::AgentId::new();
    let to = neo_agents::AgentId::new();

    q.enqueue(
        AgentMessage::new(from, to, MessageType::Request, serde_json::json!("low"))
            .with_priority(MessagePriority::Low),
    )
    .await;
    q.enqueue(
        AgentMessage::new(
            from,
            to,
            MessageType::Request,
            serde_json::json!("critical"),
        )
        .with_priority(MessagePriority::Critical),
    )
    .await;
    q.enqueue(
        AgentMessage::new(from, to, MessageType::Request, serde_json::json!("normal"))
            .with_priority(MessagePriority::Normal),
    )
    .await;

    let m1 = q.dequeue().await.unwrap();
    assert_eq!(m1.payload, serde_json::json!("critical"));
    let m2 = q.dequeue().await.unwrap();
    assert_eq!(m2.payload, serde_json::json!("normal"));
    let m3 = q.dequeue().await.unwrap();
    assert_eq!(m3.payload, serde_json::json!("low"));
}

#[tokio::test]
async fn message_ttl_expiry() {
    let msg = AgentMessage::new(
        neo_agents::AgentId::new(),
        neo_agents::AgentId::new(),
        MessageType::Request,
        serde_json::json!("expires"),
    )
    .with_ttl(0);
    assert!(!msg.is_expired());

    let msg2 = AgentMessage::new(
        neo_agents::AgentId::new(),
        neo_agents::AgentId::new(),
        MessageType::Request,
        serde_json::json!("lives"),
    );
    assert!(!msg2.is_expired());

    let mut msg3 = AgentMessage::new(
        neo_agents::AgentId::new(),
        neo_agents::AgentId::new(),
        MessageType::Request,
        serde_json::json!("past"),
    );
    msg3.ttl_secs = Some(0);
    msg3.timestamp = chrono::Utc::now() - chrono::Duration::seconds(1);
    assert!(msg3.is_expired());
}

#[tokio::test]
async fn message_correlation() {
    let from = neo_agents::AgentId::new();
    let to = neo_agents::AgentId::new();

    let request = AgentMessage::new(from, to, MessageType::Request, serde_json::json!("req"))
        .with_correlation_id(uuid::Uuid::new_v4());

    let correlation_id = request.correlation_id.unwrap();

    let reply = request.reply(to, serde_json::json!("resp"));

    assert_eq!(reply.correlation_id, Some(correlation_id));
    assert_eq!(reply.reply_to, Some(request.id));
}

#[tokio::test]
async fn message_channel_registry_direct() {
    let registry = MessageChannelRegistry::new(64);
    let from = neo_agents::AgentId::new();
    let to = neo_agents::AgentId::new();

    let ch = registry.get_or_create_channel(from, to);
    ch.send(AgentMessage::new(
        from,
        to,
        MessageType::Request,
        serde_json::json!("test"),
    ))
    .await
    .unwrap();

    let received = ch.receive().await.unwrap();
    assert_eq!(received.payload, serde_json::json!("test"));
}

#[tokio::test]
async fn manager_send_message() {
    let mgr = AgentManager::new(10);
    let from = mgr
        .create_agent(
            AgentConfiguration::new("sender")
                .with_role(AgentRole::Executor)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();
    let to = mgr
        .create_agent(
            AgentConfiguration::new("receiver")
                .with_role(AgentRole::Executor)
                .with_heartbeat_interval(10),
        )
        .await
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    mgr.message_channels.register_inbox(to, tx);

    let msg = AgentMessage::new(from, to, MessageType::Request, serde_json::json!("ping"));
    mgr.send_message(msg).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.payload, serde_json::json!("ping"));
}
