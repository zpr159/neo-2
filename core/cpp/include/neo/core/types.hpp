#pragma once

#include <cstdint>
#include <ostream>
#include <string>

namespace neo::core {

struct Version {
    std::uint32_t major{0};
    std::uint32_t minor{0};
    std::uint32_t patch{0};
    std::string pre_release;
    std::string build;

    [[nodiscard]] std::string to_string() const;
    static Version parse(const std::string& str);

    [[nodiscard]] bool operator==(const Version& other) const noexcept;
    [[nodiscard]] bool operator!=(const Version& other) const noexcept;
    [[nodiscard]] bool operator<(const Version& other) const noexcept;
    [[nodiscard]] bool operator<=(const Version& other) const noexcept;
    [[nodiscard]] bool operator>(const Version& other) const noexcept;
    [[nodiscard]] bool operator>=(const Version& other) const noexcept;

    friend std::ostream& operator<<(std::ostream& os, const Version& v);
};

enum class Environment : std::uint8_t {
    Development = 0,
    Testing = 1,
    Staging = 2,
    Production = 3
};

[[nodiscard]] const char* to_string(Environment env) noexcept;
[[nodiscard]] Environment environment_from_string(const std::string& str);

enum class Severity : std::uint8_t {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5
};

[[nodiscard]] const char* to_string(Severity sev) noexcept;
[[nodiscard]] Severity severity_from_string(const std::string& str);

} // namespace neo::core
