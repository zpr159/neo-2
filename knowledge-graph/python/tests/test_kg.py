"""Tests for neo_kg."""
from neo_kg.graph import KnowledgeGraph
from neo_kg.node import Node, NodeType
from neo_kg.edge import Edge, EdgeType


class TestKnowledgeGraph:
    def test_create_graph(self):
        g = KnowledgeGraph("test")
        assert g.name == "test"
        assert g.node_count() == 0
        assert g.edge_count() == 0

    def test_add_node(self):
        g = KnowledgeGraph("g")
        n = Node(NodeType.Entity, "Person")
        nid = g.add_node(n)
        assert nid == n.id
        assert g.node_count() == 1

    def test_get_node(self):
        g = KnowledgeGraph("g")
        n = Node(NodeType.Concept, "AI")
        g.add_node(n)
        assert g.get_node(n.id) is n
        assert g.get_node("nonexistent") is None

    def test_add_edge(self):
        g = KnowledgeGraph("g")
        n1 = Node(NodeType.Entity, "A")
        n2 = Node(NodeType.Entity, "B")
        g.add_node(n1)
        g.add_node(n2)
        e = Edge(n1.id, n2.id, EdgeType.RelatedTo)
        eid = g.add_edge(e)
        assert eid == e.id
        assert g.edge_count() == 1

    def test_get_edge(self):
        g = KnowledgeGraph("g")
        e = Edge("a", "b", EdgeType.IsA)
        g.add_edge(e)
        assert g.get_edge(e.id) is e

    def test_neighbors(self):
        g = KnowledgeGraph("g")
        n1 = Node(NodeType.Entity, "A")
        n2 = Node(NodeType.Entity, "B")
        n3 = Node(NodeType.Entity, "C")
        g.add_node(n1)
        g.add_node(n2)
        g.add_node(n3)
        g.add_edge(Edge(n1.id, n2.id, EdgeType.RelatedTo))
        g.add_edge(Edge(n1.id, n3.id, EdgeType.Causes))
        neighbors = g.neighbors(n1.id)
        assert n2.id in neighbors
        assert n3.id in neighbors

    def test_to_dict(self):
        g = KnowledgeGraph("g")
        g.add_node(Node(NodeType.Entity, "X"))
        d = g.to_dict()
        assert d["name"] == "g"
        assert len(d["nodes"]) == 1
