"""Neo Test — mock objects for testing."""
from __future__ import annotations

import uuid


class MockAgent:
    """A lightweight mock agent for testing."""

    def __init__(self, name: str) -> None:
        self._id: str = str(uuid.uuid4())
        self._name = name
        self._state: str = "stopped"
        self._messages: list[dict] = []

    @property
    def id(self) -> str:
        return self._id

    @property
    def name(self) -> str:
        return self._name

    @property
    def state(self) -> str:
        return self._state

    def start(self) -> None:
        self._state = "running"

    def stop(self) -> None:
        self._state = "stopped"

    def send(self, msg: dict) -> None:
        self._messages.append(msg)

    def receive(self) -> dict | None:
        return self._messages.pop(0) if self._messages else None


class MockTool:
    """A lightweight mock tool for testing."""

    def __init__(self, name: str, category: str = "utility") -> None:
        self._id: str = str(uuid.uuid4())
        self._name = name
        self._category = category

    @property
    def id(self) -> str:
        return self._id

    @property
    def name(self) -> str:
        return self._name

    def execute(self, params: dict) -> dict:
        return {"status": "ok", "params": params}


class MockMemory:
    """A lightweight mock memory store for testing."""

    def __init__(self) -> None:
        self._entries: dict[str, dict] = {}

    def store(self, content: dict) -> str:
        entry_id = str(uuid.uuid4())
        self._entries[entry_id] = content
        return entry_id

    def recall(self, entry_id: str) -> dict | None:
        return self._entries.get(entry_id)

    def search(self, query: str) -> list[dict]:
        q = query.lower()
        return [
            {"id": k, **v}
            for k, v in self._entries.items()
            if any(q in str(val).lower() for val in v.values())
        ]

    def count(self) -> int:
        return len(self._entries)
