#include <neo/core/config.hpp>
#include <algorithm>
#include <fstream>
#include <sstream>
#include <stdexcept>

namespace neo::core {

Config Config::load(const std::string& profile) {
    Config config;
    std::string path = "config/" + profile + ".json";

    std::ifstream file(path);
    if (!file.is_open()) {
        return config;
    }

    std::ostringstream oss;
    oss << file.rdbuf();
    std::string content = oss.str();

    if (content.empty()) {
        return config;
    }

    std::istringstream stream(content);
    std::string line;
    std::string current_section;

    while (std::getline(stream, line)) {
        if (line.empty() || line[0] == '#') {
            continue;
        }

        auto pos = line.find('=');
        if (pos != std::string::npos) {
            std::string key = line.substr(0, pos);
            std::string value = line.substr(pos + 1);

            key.erase(0, key.find_first_not_of(" \t"));
            key.erase(key.find_last_not_of(" \t") + 1);
            value.erase(0, value.find_first_not_of(" \t"));
            value.erase(value.find_last_not_of(" \t") + 1);

            if (!key.empty()) {
                config.set(key, value);
            }
        }
    }

    return config;
}

bool Config::has(const std::string& key) const noexcept {
    return sections_.find(key) != sections_.end();
}

std::vector<std::string> Config::keys() const {
    std::vector<std::string> result;
    result.reserve(sections_.size());
    for (const auto& [k, v] : sections_) {
        result.push_back(k);
    }
    std::sort(result.begin(), result.end());
    return result;
}

void Config::merge(const Config& other) {
    for (const auto& [k, v] : other.sections_) {
        sections_[k] = v;
    }
}

void Config::remove(const std::string& key) {
    sections_.erase(key);
}

std::size_t Config::size() const noexcept {
    return sections_.size();
}

bool Config::empty() const noexcept {
    return sections_.empty();
}

void Config::clear() noexcept {
    sections_.clear();
}

} // namespace neo::core
