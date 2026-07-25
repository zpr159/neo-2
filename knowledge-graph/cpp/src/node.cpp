#include <neo/knowledge/graph.hpp>
#include <neo/knowledge/node.hpp>
#include <algorithm>
#include <queue>
#include <sstream>

namespace neo::knowledge {

Node::Node(std::uint64_t id, std::string label)
    : id(id), label(std::move(label)) {}

void Node::set_property(const std::string& key, const std::string& value) {
    properties[key] = value;
}

std::string Node::get_property(const std::string& key, const std::string& default_value) const {
    auto it = properties.find(key);
    return it != properties.end() ? it->second : default_value;
}

bool Node::has_property(const std::string& key) const noexcept {
    return properties.find(key) != properties.end();
}

void Node::remove_property(const std::string& key) {
    properties.erase(key);
}

std::string Node::to_string() const {
    std::ostringstream oss;
    oss << "Node{id=" << id << ", label=" << label << ", props=" << properties.size() << "}";
    return oss.str();
}

} // namespace neo::knowledge
