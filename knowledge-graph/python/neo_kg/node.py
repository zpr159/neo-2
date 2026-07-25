"""Neo Knowledge Graph — node definitions."""
from __future__ import annotations

import uuid
from enum import Enum


class NodeType(Enum):
    Entity = "entity"
    Concept = "concept"
    Event = "event"
    Relation = "relation"
    Attribute = "attribute"


class Node:
    """A node in the knowledge graph."""

    def __init__(self, node_type: NodeType, label: str, properties: dict | None = None) -> None:
        self._id: str = str(uuid.uuid4())
        self.node_type: NodeType = node_type
        self.label: str = label
        self.properties: dict = properties or {}

    @property
    def id(self) -> str:
        return self._id

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "node_type": self.node_type.value,
            "label": self.label,
            "properties": self.properties,
        }
