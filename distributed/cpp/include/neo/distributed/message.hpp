#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>

namespace neo::distributed {

enum class MessageType : std::uint8_t {
    Heartbeat = 0,
    DataSync = 1,
    Command = 2,
    Response = 3,
    Election = 4,
    Vote = 5,
    Ping = 6,
    Pong = 7
};

[[nodiscard]] const char* to_string(MessageType type) noexcept;

struct Message {
    std::uint64_t id{0};
    std::uint64_t from{0};
    std::uint64_t to{0};
    MessageType type{MessageType::Heartbeat};
    std::string payload;
    std::uint64_t timestamp{0};

    Message() = default;
    Message(std::uint64_t id, std::uint64_t from, std::uint64_t to, MessageType type, std::string payload);

    [[nodiscard]] bool is_valid() const noexcept;
    [[nodiscard]] std::string to_string() const;
};

} // namespace neo::distributed
