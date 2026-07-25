#include <neo/memory/arena.hpp>
#include <neo/core/error.hpp>
#include <algorithm>
#include <stdexcept>

namespace neo::memory {

Arena::Arena(std::size_t capacity)
    : capacity_(capacity) {
    buffer_ = std::make_unique<uint8_t[]>(capacity);
}

void* Arena::allocate(std::size_t size, std::size_t alignment) {
    const std::size_t aligned_offset = (offset_ + alignment - 1) & ~(alignment - 1);

    if (aligned_offset + size > capacity_) {
        throw neo::core::Error(
            neo::core::NEO_ERR_RESOURCE,
            "Arena allocation failed: requested " + std::to_string(size) +
            " bytes with alignment " + std::to_string(alignment) +
            ", only " + std::to_string(capacity_ - aligned_offset) + " bytes remaining",
            "Arena::allocate"
        );
    }

    void* ptr = buffer_.get() + aligned_offset;
    offset_ = aligned_offset + size;
    return ptr;
}

void Arena::reset() noexcept {
    offset_ = 0;
}

std::size_t Arena::used() const noexcept {
    return offset_;
}

std::size_t Arena::free() const noexcept {
    return capacity_ - offset_;
}

std::size_t Arena::capacity() const noexcept {
    return capacity_;
}

bool Arena::owns(const void* ptr) const noexcept {
    auto addr = reinterpret_cast<const uint8_t*>(ptr);
    return addr >= buffer_.get() && addr < buffer_.get() + capacity_;
}

} // namespace neo::memory
