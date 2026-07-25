"""Neo Capabilities — capability registry."""
from __future__ import annotations

from neo_capabilities.capability import Capability


class CapabilityRegistry:
    """Registry for managing capabilities."""

    def __init__(self) -> None:
        self._capabilities: dict[str, Capability] = {}
        self._name_index: dict[str, str] = {}

    def register(self, capability: Capability) -> str:
        self._capabilities[capability.id] = capability
        self._name_index[capability.name] = capability.id
        return capability.id

    def unregister(self, cap_id: str) -> bool:
        if cap_id in self._capabilities:
            cap = self._capabilities.pop(cap_id)
            self._name_index.pop(cap.name, None)
            return True
        return False

    def get(self, cap_id: str) -> Capability | None:
        return self._capabilities.get(cap_id)

    def list_all(self) -> list[Capability]:
        return list(self._capabilities.values())

    def has(self, name: str) -> bool:
        return name in self._name_index
