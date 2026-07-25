"""Neo Test — test fixtures and context manager."""
from __future__ import annotations

import tempfile
from typing import Any

from neo_test.mocks import MockAgent, MockMemory


class NeoTestFixture:
    """A test fixture providing setup/teardown and helper factories."""

    def __init__(self, config: dict | None = None) -> None:
        self.config = config or {}
        self._temp_dir: str | None = None
        self._agents: list[MockAgent] = []
        self._memories: list[MockMemory] = []

    def __enter__(self) -> NeoTestFixture:
        self.setup()
        return self

    def __exit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        self.teardown()

    def setup(self) -> None:
        self._temp_dir = tempfile.mkdtemp(prefix="neo_test_")

    def teardown(self) -> None:
        self._agents.clear()
        self._memories.clear()

    def create_test_agent(self, name: str) -> MockAgent:
        agent = MockAgent(name)
        self._agents.append(agent)
        return agent

    def create_test_memory(self) -> MockMemory:
        mem = MockMemory()
        self._memories.append(mem)
        return mem

    def temp_dir(self) -> str:
        if self._temp_dir is None:
            self.setup()
        return self._temp_dir  # type: ignore
