use neo_agents::supervisor::AlertSeverity;
use neo_agents::{
    Agent, AgentConfiguration, AgentHealth, AgentId, AgentMetrics, AgentRole, AgentStatus,
    FailureDetector, RecoveryManager, RecoveryStrategy, SupervisorAgent,
};

fn make_agent(name: &str, status: AgentStatus) -> Agent {
    let mut agent = Agent::new(
        AgentConfiguration::new(name)
            .with_role(AgentRole::Executor)
            .with_heartbeat_interval(10),
    );
    agent.initialize().unwrap();
    match status {
        AgentStatus::Running => agent.start().unwrap(),
        AgentStatus::Stopped => {
            agent.start().unwrap();
            agent.stop().unwrap();
        }
        AgentStatus::Failed => {
            agent.start().unwrap();
            agent.fail("test failure");
        }
        AgentStatus::Terminated => {
            agent.start().unwrap();
            agent.terminate().unwrap();
        }
        AgentStatus::Ready => {}
        _ => {}
    }
    agent
}

#[test]
fn health_check_healthy_agent() {
    let supervisor = SupervisorAgent::new();
    let agent = make_agent("healthy", AgentStatus::Running);

    let check = supervisor.check_agent_health(&agent);
    assert_eq!(check.health, AgentHealth::Healthy);
    assert!(check.issues.is_empty());
}

#[test]
fn health_check_terminated_agent() {
    let supervisor = SupervisorAgent::new();
    let agent = make_agent("terminated", AgentStatus::Terminated);

    let check = supervisor.check_agent_health(&agent);
    assert_eq!(check.health, AgentHealth::Unhealthy);
}

#[test]
fn health_check_failed_agent() {
    let supervisor = SupervisorAgent::new();
    let agent = make_agent("failed", AgentStatus::Failed);

    let check = supervisor.check_agent_health(&agent);
    assert_eq!(check.health, AgentHealth::Unhealthy);
}

#[test]
fn failure_detection_and_recovery() {
    let supervisor = SupervisorAgent::new();
    let agent_id = AgentId::new();
    supervisor.supervise(agent_id);

    for _ in 0..4 {
        let strategy = supervisor.handle_failure(agent_id, "error".into());
        match strategy {
            RecoveryStrategy::Restart => {}
            RecoveryStrategy::FreshRestart => {}
            RecoveryStrategy::Migrate => {}
            RecoveryStrategy::SkipAndContinue => {}
            RecoveryStrategy::Escalate => {}
            RecoveryStrategy::Custom(_) => {}
        }
    }

    let fifth = supervisor.handle_failure(agent_id, "error".into());
    match fifth {
        RecoveryStrategy::Restart => {}
        RecoveryStrategy::FreshRestart => {}
        RecoveryStrategy::Migrate => {}
        RecoveryStrategy::SkipAndContinue => {}
        RecoveryStrategy::Escalate => {}
        RecoveryStrategy::Custom(_) => {}
    }

    let alerts = supervisor.unacknowledged_alerts();
    assert!(!alerts.is_empty());
}

#[test]
fn alert_acknowledgment() {
    let supervisor = SupervisorAgent::new();
    let id = AgentId::new();
    supervisor.raise_alert(id, AlertSeverity::Warning, "test".into());

    let alerts = supervisor.unacknowledged_alerts();
    assert_eq!(alerts.len(), 1);
    let alert_id = alerts[0].id;

    assert!(supervisor.acknowledge_alert(&alert_id));
    assert!(supervisor.unacknowledged_alerts().is_empty());
}

#[test]
fn load_balancing_selection() {
    let supervisor = SupervisorAgent::new();
    let a1 = AgentId::new();
    let a2 = AgentId::new();
    let a3 = AgentId::new();

    supervisor.update_agent_load(
        a1,
        &AgentMetrics {
            tasks_active: 5,
            tasks_completed: 10,
            error_count: 0,
            ..Default::default()
        },
        10,
    );
    supervisor.update_agent_load(
        a2,
        &AgentMetrics {
            tasks_active: 1,
            tasks_completed: 10,
            error_count: 0,
            ..Default::default()
        },
        10,
    );
    supervisor.update_agent_load(
        a3,
        &AgentMetrics {
            tasks_active: 8,
            tasks_completed: 10,
            error_count: 0,
            ..Default::default()
        },
        10,
    );

    let selected = supervisor.select_agent(&[a1, a2, a3]);
    assert_eq!(selected, Some(a2));
}

#[test]
fn supervised_agent_count() {
    let supervisor = SupervisorAgent::new();
    assert_eq!(supervisor.supervised_count(), 0);

    let a1 = AgentId::new();
    let a2 = AgentId::new();
    supervisor.supervise(a1);
    supervisor.supervise(a2);
    assert_eq!(supervisor.supervised_count(), 2);

    supervisor.unsupervise(&a1);
    assert_eq!(supervisor.supervised_count(), 1);
}

#[test]
fn recovery_strategy_configuration() {
    let rm = RecoveryManager::default();
    let agent_id = AgentId::new();

    rm.set_strategy(agent_id, RecoveryStrategy::FreshRestart);
    assert_eq!(rm.get_strategy(&agent_id), RecoveryStrategy::FreshRestart);

    rm.set_strategy(
        agent_id,
        RecoveryStrategy::Custom("retry-with-backoff".into()),
    );
    let strategy = rm.get_strategy(&agent_id);
    match strategy {
        RecoveryStrategy::Custom(s) => {
            assert_eq!(s, "retry-with-backoff");
        }
        _ => panic!("expected Custom"),
    }
}

#[test]
fn failure_detector_threshold() {
    let fd = FailureDetector::default();
    let agent_id = AgentId::new();

    for _ in 0..4 {
        assert!(!fd.record_failure(&agent_id));
    }
    let exceeded = fd.record_failure(&agent_id);
    assert!(exceeded);
}
