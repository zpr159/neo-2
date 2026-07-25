#pragma once

#include <cstddef>
#include <memory>
#include <mutex>
#include <optional>
#include <vector>

namespace neo::memory {

template <typename T>
class RingBuffer {
public:
    explicit RingBuffer(std::size_t capacity)
        : buffer_(std::make_unique<T[]>(capacity)), capacity_(capacity) {}

    ~RingBuffer() = default;

    RingBuffer(const RingBuffer&) = delete;
    RingBuffer& operator=(const RingBuffer&) = delete;
    RingBuffer(RingBuffer&&) noexcept = default;
    RingBuffer& operator=(RingBuffer&&) noexcept = default;

    bool push(const T& item) {
        std::lock_guard lock(mutex_);
        if (count_ == capacity_) {
            return false;
        }
        buffer_[head_] = item;
        head_ = (head_ + 1) % capacity_;
        ++count_;
        return true;
    }

    bool push(T&& item) {
        std::lock_guard lock(mutex_);
        if (count_ == capacity_) {
            return false;
        }
        buffer_[head_] = std::move(item);
        head_ = (head_ + 1) % capacity_;
        ++count_;
        return true;
    }

    std::optional<T> pop() {
        std::lock_guard lock(mutex_);
        if (count_ == 0) {
            return std::nullopt;
        }
        T item = std::move(buffer_[tail_]);
        tail_ = (tail_ + 1) % capacity_;
        --count_;
        return item;
    }

    [[nodiscard]] bool is_full() const noexcept {
        std::lock_guard lock(mutex_);
        return count_ == capacity_;
    }

    [[nodiscard]] bool is_empty() const noexcept {
        std::lock_guard lock(mutex_);
        return count_ == 0;
    }

    [[nodiscard]] std::size_t size() const noexcept {
        std::lock_guard lock(mutex_);
        return count_;
    }

    [[nodiscard]] std::size_t capacity() const noexcept {
        return capacity_;
    }

    void clear() noexcept {
        std::lock_guard lock(mutex_);
        head_ = 0;
        tail_ = 0;
        count_ = 0;
    }

private:
    std::unique_ptr<T[]> buffer_;
    std::size_t capacity_;
    std::size_t head_{0};
    std::size_t tail_{0};
    std::size_t count_{0};
    mutable std::mutex mutex_;
};

} // namespace neo::memory
