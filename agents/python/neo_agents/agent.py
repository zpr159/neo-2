"""Neo Agents — agent definitions."""
from __future__ import annotations

import uuid
from collections import deque


class AgentConfig:
    """Configuration for an agent."""

    def __init__(
        self,
        name: str,
        personality: str | None = None,
        capabilities: list[str] | None = None,
        max_memory: int = 1024 * 1024,
    ) -> None:
        self.name = name
        self.personality = personality
        self.capabilities = capabilities or []
        self.max_memory = max_memory


class Agent:
    """An autonomous agent with messaging and lifecycle."""

    def __init__(self, config: AgentConfig) -> None:
        self._id: str = str(uuid.uuid4())
        self._config = config
        self._state: str = "stopped"
        self._inbox: deque[dict] = deque()
        self._outbox: deque[dict] = deque()
        self._message_count: int = 0

    @property
    def id(self) -> str:
        return self._id

    @property
    def name(self) -> str:
        return self._config.name

    @property
    def state(self) -> str:
        return self._state

    def start(self) -> None:
        self._state = "running"

    def stop(self) -> None:
        self._state = "stopped"

    def send_message(self, msg: dict) -> None:
        self._outbox.append(msg)
        self._message_count += 1

    def receive_message(self) -> dict | None:
        if self._inbox:
            return self._inbox.popleft()
        return None

    def inject_message(self, msg: dict) -> None:
        """Internal: place a message in the inbox for receiving."""
        self._inbox.append(msg)

    def metrics(self) -> dict:
        return {
            "id": self._id,
            "name": self._config.name,
            "state": self._state,
            "messages_sent": self._message_count,
            "inbox_size": len(self._inbox),
        }
