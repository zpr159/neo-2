#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>

namespace neo::knowledge {

struct Node {
    std::uint64_t id{0};
    std::string label;
    std::unordered_map<std::string, std::string> properties;

    Node() = default;
    Node(std::uint64_t id, std::string label);

    void set_property(const std::string& key, const std::string& value);
    [[nodiscard]] std::string get_property(const std::string& key, const std::string& default_value = "") const;
    [[nodiscard]] bool has_property(const std::string& key) const noexcept;
    void remove_property(const std::string& key);

    [[nodiscard]] std::string to_string() const;
};

} // namespace neo::knowledge
