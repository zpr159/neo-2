#pragma once

#include <cstdint>
#include <string>

namespace neo::distributed {

enum class NodeState : std::uint8_t {
    Unknown = 0,
    Healthy = 1,
    Degraded = 2,
    Unreachable = 3,
    Decommissioned = 4
};

[[nodiscard]] const char* to_string(NodeState state) noexcept;

struct ClusterNode {
    std::uint64_t id{0};
    std::string address;
    NodeState state{NodeState::Unknown};
    std::uint64_t last_heartbeat{0};
    std::uint32_t failure_count{0};

    ClusterNode() = default;
    ClusterNode(std::uint64_t id, std::string address);

    [[nodiscard]] bool is_reachable() const noexcept;
    [[nodiscard]] float health_score() const noexcept;
    void record_failure() noexcept;
    void reset_health() noexcept;

    [[nodiscard]] std::string to_string() const;
};

} // namespace neo::distributed
