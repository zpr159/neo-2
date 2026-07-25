"""Neo Capabilities — capability definitions."""
from __future__ import annotations

import uuid
from enum import Enum


class CapabilityType(Enum):
    Tool = "tool"
    Skill = "skill"
    Knowledge = "knowledge"
    Communication = "communication"
    Reasoning = "reasoning"


class CapabilityState(Enum):
    Registered = "registered"
    Active = "active"
    Disabled = "disabled"
    Revoked = "revoked"


class Capability:
    """Represents a capability that can be assigned to an agent."""

    def __init__(self, name: str, cap_type: CapabilityType, description: str = "") -> None:
        self._id: str = str(uuid.uuid4())
        self.name: str = name
        self.cap_type: CapabilityType = cap_type
        self.description: str = description
        self._state: CapabilityState = CapabilityState.Registered

    @property
    def id(self) -> str:
        return self._id

    @property
    def state(self) -> CapabilityState:
        return self._state

    def enable(self) -> None:
        if self._state in (CapabilityState.Registered, CapabilityState.Disabled):
            self._state = CapabilityState.Active

    def disable(self) -> None:
        if self._state == CapabilityState.Active:
            self._state = CapabilityState.Disabled

    def revoke(self) -> None:
        self._state = CapabilityState.Revoked

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "name": self.name,
            "cap_type": self.cap_type.value,
            "description": self.description,
            "state": self._state.value,
        }
