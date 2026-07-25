#include <neo/runtime/scheduler.hpp>
#include <neo/core/error.hpp>

namespace neo::runtime {

Scheduler::Scheduler() = default;

Scheduler::Scheduler(std::size_t max_queue_size)
    : max_queue_size_(max_queue_size) {
}

Scheduler::~Scheduler() {
    stop();
}

void Scheduler::submit(Task task) {
    std::lock_guard lock(mutex_);
    if (queue_.size() >= max_queue_size_) {
        throw neo::core::Error(
            neo::core::NEO_ERR_RESOURCE,
            "Scheduler queue is full (max=" + std::to_string(max_queue_size_) + ")",
            "Scheduler"
        );
    }
    queue_.push(std::move(task));
    condition_.notify_one();
}

Task Scheduler::next() {
    std::lock_guard lock(mutex_);
    if (queue_.empty()) {
        return nullptr;
    }
    Task task = std::move(queue_.front());
    queue_.pop();
    return task;
}

std::size_t Scheduler::pending_count() const noexcept {
    std::lock_guard lock(mutex_);
    return queue_.size();
}

void Scheduler::start() {
    bool expected = false;
    if (running_.compare_exchange_strong(expected, true)) {
        condition_.notify_all();
    }
}

void Scheduler::stop() {
    running_.store(false);
    condition_.notify_all();
}

bool Scheduler::is_running() const noexcept {
    return running_.load();
}

void Scheduler::set_max_queue_size(std::size_t size) noexcept {
    max_queue_size_ = size;
}

std::size_t Scheduler::max_queue_size() const noexcept {
    return max_queue_size_;
}

void Scheduler::process_next() {
    std::unique_lock lock(mutex_);
    condition_.wait(lock, [this] {
        return !queue_.empty() || !running_.load();
    });

    if (!running_.load() && queue_.empty()) {
        return;
    }

    if (!queue_.empty()) {
        Task task = std::move(queue_.front());
        queue_.pop();
        lock.unlock();
        if (task) {
            task();
        }
    }
}

} // namespace neo::runtime
