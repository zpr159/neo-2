#pragma once

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <memory>
#include <new>

namespace neo::memory {

class Arena {
public:
    explicit Arena(std::size_t capacity);
    ~Arena() = default;

    Arena(const Arena&) = delete;
    Arena& operator=(const Arena&) = delete;
    Arena(Arena&&) noexcept = default;
    Arena& operator=(Arena&&) noexcept = default;

    void* allocate(std::size_t size, std::size_t alignment = alignof(std::max_align_t));
    void reset() noexcept;
    [[nodiscard]] std::size_t used() const noexcept;
    [[nodiscard]] std::size_t free() const noexcept;
    [[nodiscard]] std::size_t capacity() const noexcept;
    [[nodiscard]] bool owns(const void* ptr) const noexcept;

private:
    std::unique_ptr<uint8_t[]> buffer_;
    std::size_t capacity_;
    std::size_t offset_{0};
};

} // namespace neo::memory
