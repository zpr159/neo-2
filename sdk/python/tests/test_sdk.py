"""Tests for neo_sdk."""
from neo_sdk.client import NeoClient
from neo_sdk.types import AgentHandle, TaskHandle


class TestNeoClient:
    def test_create_client(self):
        c = NeoClient()
        assert c.host == "localhost"
        assert c.port == 8080
        assert c.is_connected() is False

    def test_connect_disconnect(self):
        c = NeoClient()
        c.connect()
        assert c.is_connected() is True
        c.disconnect()
        assert c.is_connected() is False

    def test_create_agent(self):
        c = NeoClient()
        h = c.create_agent("TestBot")
        assert h.name == "TestBot"
        assert h.state == "running"

    def test_submit_task(self):
        c = NeoClient()
        h = c.submit_task("agent-1", {"type": "query", "text": "hello"})
        assert h.agent_id == "agent-1"
        assert h.status == "submitted"

    def test_list_agents(self):
        c = NeoClient()
        assert c.list_agents() == []

    def test_health(self):
        c = NeoClient()
        h = c.health()
        assert h["status"] == "ok"


class TestAgentHandle:
    def test_to_dict(self):
        h = AgentHandle(id="1", name="A", state="running", created_at="2025-01-01T00:00:00")
        d = h.to_dict()
        assert d["id"] == "1"
        assert d["name"] == "A"


class TestTaskHandle:
    def test_to_dict(self):
        h = TaskHandle(id="1", agent_id="a", status="done", result={"out": 1}, created_at="2025-01-01T00:00:00")
        d = h.to_dict()
        assert d["agent_id"] == "a"
        assert d["result"] == {"out": 1}
