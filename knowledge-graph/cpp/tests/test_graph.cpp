#include <gtest/gtest.h>
#include <neo/knowledge/graph.hpp>

using namespace neo::knowledge;

TEST(NodeTest, Creation) {
    Node n(1, "Person");
    EXPECT_EQ(n.id, 1u);
    EXPECT_EQ(n.label, "Person");
}

TEST(NodeTest, Properties) {
    Node n(1, "Person");
    n.set_property("name", "Alice");
    n.set_property("age", "30");
    EXPECT_EQ(n.get_property("name"), "Alice");
    EXPECT_EQ(n.get_property("age"), "30");
    EXPECT_TRUE(n.has_property("name"));
    EXPECT_FALSE(n.has_property("email"));
    EXPECT_EQ(n.get_property("missing", "default"), "default");
}

TEST(NodeTest, RemoveProperty) {
    Node n(1, "Person");
    n.set_property("key", "val");
    n.remove_property("key");
    EXPECT_FALSE(n.has_property("key"));
}

TEST(NodeTest, Tostring) {
    Node n(42, "Widget");
    std::string str = n.to_string();
    EXPECT_NE(str.find("42"), std::string::npos);
    EXPECT_NE(str.find("Widget"), std::string::npos);
}

TEST(GraphTest, AddRemoveNode) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    EXPECT_EQ(g.node_count(), 2u);
    EXPECT_TRUE(g.has_node(1));
    EXPECT_TRUE(g.has_node(2));

    g.remove_node(1);
    EXPECT_EQ(g.node_count(), 1u);
    EXPECT_FALSE(g.has_node(1));
}

TEST(GraphTest, AddRemoveEdge) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_edge(1, 2);

    EXPECT_TRUE(g.has_edge(1, 2));
    EXPECT_FALSE(g.has_edge(2, 1));
    EXPECT_EQ(g.edge_count(), 1u);

    g.remove_edge(1, 2);
    EXPECT_FALSE(g.has_edge(1, 2));
    EXPECT_EQ(g.edge_count(), 0u);
}

TEST(GraphTest, Neighbors) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_node(3, "C");
    g.add_edge(1, 2);
    g.add_edge(1, 3);

    auto nb = g.neighbors(1);
    EXPECT_EQ(nb.size(), 2u);
}

TEST(GraphTest, Incoming) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_edge(1, 2);

    auto inc = g.incoming(2);
    EXPECT_EQ(inc.size(), 1u);
    EXPECT_EQ(inc[0], 1u);
}

TEST(GraphTest, FindNodes) {
    Graph<> g;
    g.add_node(1, "Person");
    g.add_node(2, "Person");
    g.add_node(3, "Place");

    auto people = g.find_nodes([](const Node& n) {
        return n.label == "Person";
    });
    EXPECT_EQ(people.size(), 2u);
}

TEST(GraphTest, FindNodeByLabel) {
    Graph<> g;
    g.add_node(1, "Person");
    auto found = g.find_node_by_label("Person");
    EXPECT_TRUE(found.has_value());
    EXPECT_EQ(*found, 1u);

    auto not_found = g.find_node_by_label("Place");
    EXPECT_FALSE(not_found.has_value());
}

TEST(GraphTest, BFS) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_node(3, "C");
    g.add_node(4, "D");
    g.add_edge(1, 2);
    g.add_edge(1, 3);
    g.add_edge(2, 4);

    auto order = g.bfs(1);
    EXPECT_EQ(order.size(), 4u);
    EXPECT_EQ(order[0], 1u);
}

TEST(GraphTest, DFS) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_node(3, "C");
    g.add_edge(1, 2);
    g.add_edge(1, 3);

    auto order = g.dfs(1);
    EXPECT_EQ(order.size(), 3u);
    EXPECT_EQ(order[0], 1u);
}

TEST(GraphTest, PathExists) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_node(2, "B");
    g.add_node(3, "C");
    g.add_edge(1, 2);
    g.add_edge(2, 3);

    EXPECT_TRUE(g.path_exists(1, 3));
    EXPECT_FALSE(g.path_exists(3, 1));
}

TEST(GraphTest, Clear) {
    Graph<> g;
    g.add_node(1, "A");
    g.add_edge(1, 1);
    g.clear();
    EXPECT_TRUE(g.empty());
    EXPECT_EQ(g.edge_count(), 0u);
}
