"""Neo Knowledge Graph — edge definitions."""
from __future__ import annotations

import uuid
from enum import Enum


class EdgeType(Enum):
    IsA = "is_a"
    HasProperty = "has_property"
    RelatedTo = "related_to"
    Causes = "causes"
    PartOf = "part_of"
    DependsOn = "depends_on"


class Edge:
    """An edge in the knowledge graph."""

    def __init__(
        self,
        source: str,
        target: str,
        edge_type: EdgeType,
        weight: float = 1.0,
        properties: dict | None = None,
    ) -> None:
        self._id: str = str(uuid.uuid4())
        self.source: str = source
        self.target: str = target
        self.edge_type: EdgeType = edge_type
        self.weight: float = weight
        self.properties: dict = properties or {}

    @property
    def id(self) -> str:
        return self._id

    def to_dict(self) -> dict:
        return {
            "id": self._id,
            "source": self.source,
            "target": self.target,
            "edge_type": self.edge_type.value,
            "weight": self.weight,
            "properties": self.properties,
        }
