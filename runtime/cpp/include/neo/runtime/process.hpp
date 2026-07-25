#pragma once

#include <cstdint>
#include <string>

namespace neo::runtime {

enum class ProcessState : std::uint8_t {
    Idle = 0,
    Running = 1,
    Blocked = 2,
    Suspended = 3,
    Completed = 4,
    Failed = 5
};

[[nodiscard]] const char* to_string(ProcessState state) noexcept;
[[nodiscard]] ProcessState process_state_from_string(const std::string& str);

struct Process {
    std::uint64_t pid{0};
    std::string name;
    ProcessState state{ProcessState::Idle};

    [[nodiscard]] bool is_running() const noexcept;
    void terminate();
    void suspend();
    void resume();

    [[nodiscard]] std::string to_string() const;
};

} // namespace neo::runtime
