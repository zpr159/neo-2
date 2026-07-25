#pragma once

#include <atomic>
#include <condition_variable>
#include <cstdint>
#include <functional>
#include <mutex>
#include <queue>
#include <vector>

namespace neo::runtime {

using Task = std::function<void()>;

class Scheduler {
public:
    Scheduler();
    explicit Scheduler(std::size_t max_queue_size);
    ~Scheduler();

    Scheduler(const Scheduler&) = delete;
    Scheduler& operator=(const Scheduler&) = delete;
    Scheduler(Scheduler&&) noexcept = delete;
    Scheduler& operator=(Scheduler&&) noexcept = delete;

    void submit(Task task);
    [[nodiscard]] Task next();
    [[nodiscard]] std::size_t pending_count() const noexcept;

    void start();
    void stop();
    [[nodiscard]] bool is_running() const noexcept;

    void set_max_queue_size(std::size_t size) noexcept;
    [[nodiscard]] std::size_t max_queue_size() const noexcept;

private:
    std::queue<Task> queue_;
    std::size_t max_queue_size_{1024};
    std::atomic<bool> running_{false};
    mutable std::mutex mutex_;
    std::condition_variable condition_;

    void process_next();
};

} // namespace neo::runtime
