"""Neo Knowledge Graph — graph data structure."""
from __future__ import annotations

from neo_kg.edge import Edge
from neo_kg.node import Node


class KnowledgeGraph:
    """An in-memory knowledge graph with nodes and edges."""

    def __init__(self, name: str) -> None:
        self.name = name
        self._nodes: dict[str, Node] = {}
        self._edges: dict[str, Edge] = {}
        self._adjacency: dict[str, list[str]] = {}

    def add_node(self, node: Node) -> str:
        self._nodes[node.id] = node
        if node.id not in self._adjacency:
            self._adjacency[node.id] = []
        return node.id

    def add_edge(self, edge: Edge) -> str:
        self._edges[edge.id] = edge
        self._adjacency.setdefault(edge.source, []).append(edge.target)
        self._adjacency.setdefault(edge.target, []).append(edge.source)
        return edge.id

    def get_node(self, node_id: str) -> Node | None:
        return self._nodes.get(node_id)

    def get_edge(self, edge_id: str) -> Edge | None:
        return self._edges.get(edge_id)

    def neighbors(self, node_id: str) -> list[str]:
        return list(self._adjacency.get(node_id, []))

    def node_count(self) -> int:
        return len(self._nodes)

    def edge_count(self) -> int:
        return len(self._edges)

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "nodes": [n.to_dict() for n in self._nodes.values()],
            "edges": [e.to_dict() for e in self._edges.values()],
        }
