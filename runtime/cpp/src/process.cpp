#include <neo/runtime/process.hpp>
#include <stdexcept>

namespace neo::runtime {

const char* to_string(ProcessState state) noexcept {
    switch (state) {
        case ProcessState::Idle: return "Idle";
        case ProcessState::Running: return "Running";
        case ProcessState::Blocked: return "Blocked";
        case ProcessState::Suspended: return "Suspended";
        case ProcessState::Completed: return "Completed";
        case ProcessState::Failed: return "Failed";
    }
    return "Unknown";
}

ProcessState process_state_from_string(const std::string& str) {
    if (str == "Idle" || str == "idle") return ProcessState::Idle;
    if (str == "Running" || str == "running") return ProcessState::Running;
    if (str == "Blocked" || str == "blocked") return ProcessState::Blocked;
    if (str == "Suspended" || str == "suspended") return ProcessState::Suspended;
    if (str == "Completed" || str == "completed") return ProcessState::Completed;
    if (str == "Failed" || str == "failed") return ProcessState::Failed;
    throw std::invalid_argument("Unknown process state: " + str);
}

bool Process::is_running() const noexcept {
    return state == ProcessState::Running;
}

void Process::terminate() {
    if (state == ProcessState::Completed || state == ProcessState::Failed) {
        return;
    }
    state = ProcessState::Failed;
}

void Process::suspend() {
    if (state == ProcessState::Running) {
        state = ProcessState::Suspended;
    }
}

void Process::resume() {
    if (state == ProcessState::Suspended) {
        state = ProcessState::Running;
    }
}

std::string Process::to_string() const {
    return "Process{pid=" + std::to_string(pid) + ", name=" + name
        + ", state=" + neo::runtime::to_string(state) + "}";
}

} // namespace neo::runtime
