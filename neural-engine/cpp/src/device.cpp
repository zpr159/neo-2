#include <neo/neural/device.hpp>
#include <algorithm>

namespace neo::neural {

const char* to_string(DeviceType type) noexcept {
    switch (type) {
        case DeviceType::CPU: return "CPU";
        case DeviceType::CUDA: return "CUDA";
        case DeviceType::Metal: return "Metal";
        case DeviceType::Vulkan: return "Vulkan";
    }
    return "Unknown";
}

Device Device::cpu() {
    Device dev;
    dev.type = DeviceType::CPU;
    dev.name = "cpu";
    dev.memory_total = 0;
    dev.memory_available = 0;
    return dev;
}

std::vector<Device> Device::detect_all() {
    std::vector<Device> devices;
    devices.push_back(cpu());
    return devices;
}

bool Device::is_available() const noexcept {
    return true;
}

bool Device::is_gpu() const noexcept {
    return type != DeviceType::CPU;
}

float Device::memory_usage_percent() const noexcept {
    if (memory_total == 0) return 0.0f;
    return static_cast<float>(memory_total - memory_available) /
           static_cast<float>(memory_total) * 100.0f;
}

} // namespace neo::neural
