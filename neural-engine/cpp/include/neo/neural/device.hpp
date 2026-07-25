#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace neo::neural {

enum class DeviceType : std::uint8_t {
    CPU = 0,
    CUDA = 1,
    Metal = 2,
    Vulkan = 3
};

[[nodiscard]] const char* to_string(DeviceType type) noexcept;

struct Device {
    DeviceType type{DeviceType::CPU};
    std::string name{"cpu"};
    std::uint64_t memory_total{0};
    std::uint64_t memory_available{0};

    [[nodiscard]] static Device cpu();
    [[nodiscard]] static std::vector<Device> detect_all();
    [[nodiscard]] bool is_available() const noexcept;
    [[nodiscard]] bool is_gpu() const noexcept;
    [[nodiscard]] float memory_usage_percent() const noexcept;
};

} // namespace neo::neural
