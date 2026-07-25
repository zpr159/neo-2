#pragma once

#include <neo/knowledge/node.hpp>
#include <cstdint>
#include <functional>
#include <optional>
#include <unordered_map>
#include <unordered_set>
#include <vector>

namespace neo::knowledge {

template <typename NodeId = std::uint64_t>
class Graph {
public:
    using NodePredicate = std::function<bool(const Node&)>;

    Graph() = default;
    ~Graph() = default;

    Graph(const Graph&) = default;
    Graph& operator=(const Graph&) = default;
    Graph(Graph&&) noexcept = default;
    Graph& operator=(Graph&&) noexcept = default;

    void add_node(const Node& node);
    void add_node(NodeId id, const std::string& label);
    bool remove_node(NodeId id);
    [[nodiscard]] std::optional<Node> get_node(NodeId id) const;
    [[nodiscard]] bool has_node(NodeId id) const noexcept;

    void add_edge(NodeId from, NodeId to);
    void add_edge(NodeId from, NodeId to, const std::string& label);
    bool remove_edge(NodeId from, NodeId to);
    [[nodiscard]] bool has_edge(NodeId from, NodeId to) const noexcept;

    [[nodiscard]] std::vector<NodeId> neighbors(NodeId id) const;
    [[nodiscard]] std::vector<NodeId> incoming(NodeId id) const;
    [[nodiscard]] std::vector<Node> find_nodes(const NodePredicate& predicate) const;
    [[nodiscard]] std::optional<NodeId> find_node_by_label(const std::string& label) const;

    [[nodiscard]] std::size_t node_count() const noexcept;
    [[nodiscard]] std::size_t edge_count() const noexcept;
    [[nodiscard]] bool empty() const noexcept;

    void clear() noexcept;

    [[nodiscard]] std::vector<NodeId> bfs(NodeId start) const;
    [[nodiscard]] std::vector<NodeId> dfs(NodeId start) const;
    [[nodiscard]] bool path_exists(NodeId from, NodeId to) const;

private:
    std::unordered_map<NodeId, Node> nodes_;
    std::unordered_map<NodeId, std::unordered_set<NodeId>> adjacency_;
    std::unordered_map<NodeId, std::unordered_set<NodeId>> reverse_adjacency_;
    std::size_t edge_count_{0};

    void dfs_visit(NodeId id, std::unordered_set<NodeId>& visited, std::vector<NodeId>& result) const;
};

} // namespace neo::knowledge
