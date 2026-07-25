#pragma once

#include <cstdint>
#include <stdexcept>
#include <string>

namespace neo::core {

enum class ErrorCategory : std::uint32_t {
    System = 0,
    Runtime = 1,
    Network = 2,
    Security = 3,
    Neural = 4,
    IO = 5
};

constexpr std::int32_t NEO_OK = 0;
constexpr std::int32_t NEO_ERR_GENERAL = 1000;
constexpr std::int32_t NEO_ERR_TIMEOUT = 1001;
constexpr std::int32_t NEO_ERR_NOT_FOUND = 1002;
constexpr std::int32_t NEO_ERR_PERMISSION = 1003;
constexpr std::int32_t NEO_ERR_RESOURCE = 1004;

class Error final : public std::exception {
public:
    Error(std::int32_t code, std::string message, std::string source);
    ~Error() override = default;

    Error(const Error&) = default;
    Error& operator=(const Error&) = default;
    Error(Error&&) noexcept = default;
    Error& operator=(Error&&) noexcept = default;

    [[nodiscard]] const char* what() const noexcept override;
    [[nodiscard]] std::int32_t code() const noexcept;
    [[nodiscard]] const std::string& message() const noexcept;
    [[nodiscard]] const std::string& source() const noexcept;
    [[nodiscard]] ErrorCategory category() const noexcept;

    void set_category(ErrorCategory category) noexcept;

private:
    std::int32_t code_;
    std::string message_;
    std::string source_;
    ErrorCategory category_{ErrorCategory::System};
    mutable std::string what_;
};

[[nodiscard]] const char* error_category_name(ErrorCategory cat) noexcept;
[[nodiscard]] std::string format_error(const Error& err);

} // namespace neo::core
