"""Tests for neo_agents."""
from neo_agents.agent import Agent, AgentConfig


class TestAgentConfig:
    def test_create_config(self):
        c = AgentConfig(name="TestAgent", personality="helpful")
        assert c.name == "TestAgent"
        assert c.personality == "helpful"
        assert c.max_memory == 1024 * 1024

    def test_config_defaults(self):
        c = AgentConfig(name="A")
        assert c.capabilities == []


class TestAgent:
    def test_create_agent(self):
        a = Agent(AgentConfig(name="Agent1"))
        assert a.name == "Agent1"
        assert a.state == "stopped"
        assert len(a.id) == 36

    def test_start_stop(self):
        a = Agent(AgentConfig(name="A"))
        a.start()
        assert a.state == "running"
        a.stop()
        assert a.state == "stopped"

    def test_send_receive(self):
        a = Agent(AgentConfig(name="A"))
        a.inject_message({"text": "hello"})
        msg = a.receive_message()
        assert msg == {"text": "hello"}
        assert a.receive_message() is None

    def test_send_message(self):
        a = Agent(AgentConfig(name="A"))
        a.send_message({"to": "b", "text": "hi"})
        m = a.metrics()
        assert m["messages_sent"] == 1

    def test_metrics(self):
        a = Agent(AgentConfig(name="A"))
        m = a.metrics()
        assert "id" in m
        assert "state" in m
