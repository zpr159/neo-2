#include <neo/distributed/node.hpp>
#include <neo/distributed/message.hpp>
#include <sstream>

namespace neo::distributed {

const char* to_string(NodeState state) noexcept {
    switch (state) {
        case NodeState::Unknown: return "Unknown";
        case NodeState::Healthy: return "Healthy";
        case NodeState::Degraded: return "Degraded";
        case NodeState::Unreachable: return "Unreachable";
        case NodeState::Decommissioned: return "Decommissioned";
    }
    return "Unknown";
}

ClusterNode::ClusterNode(std::uint64_t id, std::string address)
    : id(id), address(std::move(address)) {}

bool ClusterNode::is_reachable() const noexcept {
    return state == NodeState::Healthy || state == NodeState::Degraded;
}

float ClusterNode::health_score() const noexcept {
    switch (state) {
        case NodeState::Healthy: return 1.0f;
        case NodeState::Degraded: return 0.5f;
        case NodeState::Unreachable: return 0.0f;
        case NodeState::Decommissioned: return 0.0f;
        case NodeState::Unknown: return 0.25f;
    }
    return 0.0f;
}

void ClusterNode::record_failure() noexcept {
    ++failure_count;
    if (failure_count >= 3) {
        state = NodeState::Unreachable;
    } else if (failure_count >= 1) {
        state = NodeState::Degraded;
    }
}

void ClusterNode::reset_health() noexcept {
    failure_count = 0;
    state = NodeState::Healthy;
}

std::string ClusterNode::to_string() const {
    std::ostringstream oss;
    oss << "ClusterNode{id=" << id << ", addr=" << address
        << ", state=" << neo::distributed::to_string(state)
        << ", failures=" << failure_count << "}";
    return oss.str();
}

const char* to_string(MessageType type) noexcept {
    switch (type) {
        case MessageType::Heartbeat: return "Heartbeat";
        case MessageType::DataSync: return "DataSync";
        case MessageType::Command: return "Command";
        case MessageType::Response: return "Response";
        case MessageType::Election: return "Election";
        case MessageType::Vote: return "Vote";
        case MessageType::Ping: return "Ping";
        case MessageType::Pong: return "Pong";
    }
    return "Unknown";
}

Message::Message(std::uint64_t id, std::uint64_t from, std::uint64_t to, MessageType type, std::string payload)
    : id(id), from(from), to(to), type(type), payload(std::move(payload)) {}

bool Message::is_valid() const noexcept {
    return from != to && !payload.empty();
}

std::string Message::to_string() const {
    std::ostringstream oss;
    oss << "Message{id=" << id << ", from=" << from << ", to=" << to
        << ", type=" << neo::distributed::to_string(type)
        << ", payload_len=" << payload.size() << "}";
    return oss.str();
}

} // namespace neo::distributed
