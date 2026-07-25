#include <neo/knowledge/graph.hpp>
#include <neo/core/error.hpp>
#include <algorithm>
#include <queue>
#include <stdexcept>

namespace neo::knowledge {

template <typename NodeId>
void Graph<NodeId>::add_node(const Node& node) {
    nodes_[node.id] = node;
    adjacency_.try_emplace(node.id);
    reverse_adjacency_.try_emplace(node.id);
}

template <typename NodeId>
void Graph<NodeId>::add_node(NodeId id, const std::string& label) {
    add_node(Node(id, label));
}

template <typename NodeId>
bool Graph<NodeId>::remove_node(NodeId id) {
    auto it = nodes_.find(id);
    if (it == nodes_.end()) return false;

    if (adjacency_.contains(id)) {
        edge_count_ -= adjacency_[id].size();
    }

    for (auto& [_, neighbors] : adjacency_) {
        neighbors.erase(id);
    }
    for (auto& [_, incoming] : reverse_adjacency_) {
        incoming.erase(id);
    }

    adjacency_.erase(id);
    reverse_adjacency_.erase(id);
    nodes_.erase(it);
    return true;
}

template <typename NodeId>
std::optional<Node> Graph<NodeId>::get_node(NodeId id) const {
    auto it = nodes_.find(id);
    if (it == nodes_.end()) return std::nullopt;
    return it->second;
}

template <typename NodeId>
bool Graph<NodeId>::has_node(NodeId id) const noexcept {
    return nodes_.contains(id);
}

template <typename NodeId>
void Graph<NodeId>::add_edge(NodeId from, NodeId to) {
    if (!nodes_.contains(from) || !nodes_.contains(to)) {
        throw neo::core::Error(
            neo::core::NEO_ERR_NOT_FOUND,
            "Cannot add edge: node not found",
            "Graph::add_edge"
        );
    }
    if (adjacency_[from].insert(to).second) {
        reverse_adjacency_[to].insert(from);
        ++edge_count_;
    }
}

template <typename NodeId>
void Graph<NodeId>::add_edge(NodeId from, NodeId to, const std::string& /*label*/) {
    add_edge(from, to);
}

template <typename NodeId>
bool Graph<NodeId>::remove_edge(NodeId from, NodeId to) {
    auto it = adjacency_.find(from);
    if (it == adjacency_.end()) return false;
    if (it->second.erase(to) == 0) return false;

    reverse_adjacency_[to].erase(from);
    --edge_count_;
    return true;
}

template <typename NodeId>
bool Graph<NodeId>::has_edge(NodeId from, NodeId to) const noexcept {
    auto it = adjacency_.find(from);
    return it != adjacency_.end() && it->second.contains(to);
}

template <typename NodeId>
std::vector<NodeId> Graph<NodeId>::neighbors(NodeId id) const {
    auto it = adjacency_.find(id);
    if (it == adjacency_.end()) return {};
    return {it->second.begin(), it->second.end()};
}

template <typename NodeId>
std::vector<NodeId> Graph<NodeId>::incoming(NodeId id) const {
    auto it = reverse_adjacency_.find(id);
    if (it == reverse_adjacency_.end()) return {};
    return {it->second.begin(), it->second.end()};
}

template <typename NodeId>
std::vector<Node> Graph<NodeId>::find_nodes(const NodePredicate& predicate) const {
    std::vector<Node> result;
    for (const auto& [id, node] : nodes_) {
        if (predicate(node)) {
            result.push_back(node);
        }
    }
    return result;
}

template <typename NodeId>
std::optional<NodeId> Graph<NodeId>::find_node_by_label(const std::string& label) const {
    for (const auto& [id, node] : nodes_) {
        if (node.label == label) {
            return id;
        }
    }
    return std::nullopt;
}

template <typename NodeId>
std::size_t Graph<NodeId>::node_count() const noexcept {
    return nodes_.size();
}

template <typename NodeId>
std::size_t Graph<NodeId>::edge_count() const noexcept {
    return edge_count_;
}

template <typename NodeId>
bool Graph<NodeId>::empty() const noexcept {
    return nodes_.empty();
}

template <typename NodeId>
void Graph<NodeId>::clear() noexcept {
    nodes_.clear();
    adjacency_.clear();
    reverse_adjacency_.clear();
    edge_count_ = 0;
}

template <typename NodeId>
std::vector<NodeId> Graph<NodeId>::bfs(NodeId start) const {
    std::vector<NodeId> result;
    std::unordered_set<NodeId> visited;
    std::queue<NodeId> queue;

    if (!nodes_.contains(start)) return result;

    visited.insert(start);
    queue.push(start);

    while (!queue.empty()) {
        NodeId current = queue.front();
        queue.pop();
        result.push_back(current);

        auto it = adjacency_.find(current);
        if (it != adjacency_.end()) {
            for (const auto& neighbor : it->second) {
                if (visited.insert(neighbor).second) {
                    queue.push(neighbor);
                }
            }
        }
    }
    return result;
}

template <typename NodeId>
std::vector<NodeId> Graph<NodeId>::dfs(NodeId start) const {
    std::vector<NodeId> result;
    std::unordered_set<NodeId> visited;
    if (nodes_.contains(start)) {
        dfs_visit(start, visited, result);
    }
    return result;
}

template <typename NodeId>
bool Graph<NodeId>::path_exists(NodeId from, NodeId to) const {
    auto reachable = bfs(from);
    return std::find(reachable.begin(), reachable.end(), to) != reachable.end();
}

template <typename NodeId>
void Graph<NodeId>::dfs_visit(NodeId id, std::unordered_set<NodeId>& visited, std::vector<NodeId>& result) const {
    visited.insert(id);
    result.push_back(id);

    auto it = adjacency_.find(id);
    if (it != adjacency_.end()) {
        for (const auto& neighbor : it->second) {
            if (!visited.contains(neighbor)) {
                dfs_visit(neighbor, visited, result);
            }
        }
    }
}

template class Graph<std::uint64_t>;
template class Graph<std::string>;

} // namespace neo::knowledge
