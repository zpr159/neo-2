"""Neo SDK — client for communicating with Neo AGI OS."""
from __future__ import annotations

import uuid
from datetime import datetime, timezone

from neo_sdk.types import AgentHandle, TaskHandle


class NeoClient:
    """Client for interacting with the Neo AGI OS API."""

    def __init__(self, host: str = "localhost", port: int = 8080, api_key: str | None = None) -> None:
        self.host = host
        self.port = port
        self.api_key = api_key
        self._connected: bool = False

    def connect(self) -> None:
        self._connected = True

    def disconnect(self) -> None:
        self._connected = False

    def is_connected(self) -> bool:
        return self._connected

    def create_agent(self, name: str, config: dict | None = None) -> AgentHandle:
        return AgentHandle(
            id=str(uuid.uuid4()),
            name=name,
            state="running",
            created_at=datetime.now(timezone.utc).isoformat(),
        )

    def submit_task(self, agent_id: str, task: dict) -> TaskHandle:
        return TaskHandle(
            id=str(uuid.uuid4()),
            agent_id=agent_id,
            status="submitted",
            result=None,
            created_at=datetime.now(timezone.utc).isoformat(),
        )

    def list_agents(self) -> list[AgentHandle]:
        return []

    def health(self) -> dict:
        return {"status": "ok", "connected": self._connected}
