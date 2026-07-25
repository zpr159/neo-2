"""Neo Tools — tool registry."""
from __future__ import annotations

from neo_tools.tool import Tool


class ToolRegistry:
    """Registry for managing tools."""

    def __init__(self) -> None:
        self._tools: dict[str, Tool] = {}
        self._name_index: dict[str, str] = {}

    def register(self, tool: Tool) -> str:
        self._tools[tool.id] = tool
        self._name_index[tool.name] = tool.id
        return tool.id

    def unregister(self, tool_id: str) -> bool:
        if tool_id in self._tools:
            tool = self._tools.pop(tool_id)
            self._name_index.pop(tool.name, None)
            return True
        return False

    def get(self, tool_id: str) -> Tool | None:
        return self._tools.get(tool_id)

    def list_all(self) -> list[Tool]:
        return list(self._tools.values())

    def search(self, query: str) -> list[Tool]:
        q = query.lower()
        return [t for t in self._tools.values() if q in t.name.lower() or q in t.description.lower()]

    def count(self) -> int:
        return len(self._tools)
