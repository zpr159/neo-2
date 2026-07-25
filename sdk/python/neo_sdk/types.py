"""Neo SDK — type definitions for handles."""
from __future__ import annotations


class AgentHandle:
    """A handle representing an agent on the server."""

    def __init__(self, id: str, name: str, state: str, created_at: str) -> None:
        self.id = id
        self.name = name
        self.state = state
        self.created_at = created_at

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "name": self.name,
            "state": self.state,
            "created_at": self.created_at,
        }


class TaskHandle:
    """A handle representing a submitted task."""

    def __init__(self, id: str, agent_id: str, status: str, result: dict | None, created_at: str) -> None:
        self.id = id
        self.agent_id = agent_id
        self.status = status
        self.result = result
        self.created_at = created_at

    def to_dict(self) -> dict:
        return {
            "id": self.id,
            "agent_id": self.agent_id,
            "status": self.status,
            "result": self.result,
            "created_at": self.created_at,
        }
