"""Neo Tools — tool definitions."""
from __future__ import annotations

import uuid
from enum import Enum


class ToolCategory(Enum):
    Search = "search"
    Code = "code"
    Data = "data"
    Communication = "communication"
    Utility = "utility"


class ToolStatus(Enum):
    Enabled = "enabled"
    Disabled = "disabled"


class Tool:
    """A callable tool that can be registered and used by agents."""

    def __init__(self, name: str, description: str, category: ToolCategory) -> None:
        self._id: str = str(uuid.uuid4())
        self.name: str = name
        self.description: str = description
        self.category: ToolCategory = category
        self._status: ToolStatus = ToolStatus.Enabled

    @property
    def id(self) -> str:
        return self._id

    @property
    def status(self) -> ToolStatus:
        return self._status

    def enable(self) -> None:
        self._status = ToolStatus.Enabled

    def disable(self) -> None:
        self._status = ToolStatus.Disabled

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "name": self.name,
            "description": self.description,
            "category": self.category.value,
            "status": self._status.value,
        }
