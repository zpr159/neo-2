#pragma once

#include <any>
#include <string>
#include <unordered_map>
#include <vector>

namespace neo::core {

class Config {
public:
    Config() = default;
    ~Config() = default;

    Config(const Config&) = default;
    Config& operator=(const Config&) = default;
    Config(Config&&) noexcept = default;
    Config& operator=(Config&&) noexcept = default;

    [[nodiscard]] static Config load(const std::string& profile);

    template <typename T>
    [[nodiscard]] T get(const std::string& key) const {
        auto it = sections_.find(key);
        if (it == sections_.end()) {
            throw std::out_of_range("Config key not found: " + key);
        }
        return std::any_cast<T>(it->second);
    }

    template <typename T>
    void set(const std::string& key, T value) {
        sections_[key] = std::move(value);
    }

    [[nodiscard]] bool has(const std::string& key) const noexcept;
    [[nodiscard]] std::vector<std::string> keys() const;
    void merge(const Config& other);
    void remove(const std::string& key);

    [[nodiscard]] std::size_t size() const noexcept;
    [[nodiscard]] bool empty() const noexcept;
    void clear() noexcept;

private:
    std::unordered_map<std::string, std::any> sections_;
};

} // namespace neo::core
