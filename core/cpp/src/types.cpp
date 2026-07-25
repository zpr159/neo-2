#include <neo/core/types.hpp>
#include <algorithm>
#include <sstream>
#include <stdexcept>

namespace neo::core {

std::string Version::to_string() const {
    std::ostringstream oss;
    oss << major << "." << minor << "." << patch;
    if (!pre_release.empty()) {
        oss << "-" << pre_release;
    }
    if (!build.empty()) {
        oss << "+" << build;
    }
    return oss.str();
}

Version Version::parse(const std::string& str) {
    Version v{};
    std::istringstream iss(str);
    char dot;
    iss >> v.major >> dot >> v.minor >> dot >> v.patch;

    if (iss.peek() == '-') {
        iss.ignore();
        std::getline(iss, v.build, '+');
        std::string remaining;
        std::getline(iss, remaining);
        if (!remaining.empty()) {
            v.pre_release = std::move(v.build);
            v.build = std::move(remaining);
        }
    } else if (iss.peek() == '+') {
        iss.ignore();
        std::getline(iss, v.build);
    }
    return v;
}

bool Version::operator==(const Version& other) const noexcept {
    return major == other.major && minor == other.minor && patch == other.patch
        && pre_release == other.pre_release;
}

bool Version::operator!=(const Version& other) const noexcept {
    return !(*this == other);
}

bool Version::operator<(const Version& other) const noexcept {
    if (major != other.major) return major < other.major;
    if (minor != other.minor) return minor < other.minor;
    if (patch != other.patch) return patch < other.patch;
    if (pre_release.empty() && !other.pre_release.empty()) return false;
    if (!pre_release.empty() && other.pre_release.empty()) return true;
    return pre_release < other.pre_release;
}

bool Version::operator<=(const Version& other) const noexcept {
    return *this == other || *this < other;
}

bool Version::operator>(const Version& other) const noexcept {
    return !(*this <= other);
}

bool Version::operator>=(const Version& other) const noexcept {
    return !(*this < other);
}

std::ostream& operator<<(std::ostream& os, const Version& v) {
    return os << v.to_string();
}

const char* to_string(Environment env) noexcept {
    switch (env) {
        case Environment::Development: return "Development";
        case Environment::Testing: return "Testing";
        case Environment::Staging: return "Staging";
        case Environment::Production: return "Production";
    }
    return "Unknown";
}

Environment environment_from_string(const std::string& str) {
    if (str == "Development" || str == "development" || str == "dev") {
        return Environment::Development;
    }
    if (str == "Testing" || str == "testing" || str == "test") {
        return Environment::Testing;
    }
    if (str == "Staging" || str == "staging" || str == "stage") {
        return Environment::Staging;
    }
    if (str == "Production" || str == "production" || str == "prod") {
        return Environment::Production;
    }
    throw std::invalid_argument("Unknown environment: " + str);
}

const char* to_string(Severity sev) noexcept {
    switch (sev) {
        case Severity::Trace: return "Trace";
        case Severity::Debug: return "Debug";
        case Severity::Info: return "Info";
        case Severity::Warn: return "Warn";
        case Severity::Error: return "Error";
        case Severity::Fatal: return "Fatal";
    }
    return "Unknown";
}

Severity severity_from_string(const std::string& str) {
    if (str == "Trace" || str == "trace") return Severity::Trace;
    if (str == "Debug" || str == "debug") return Severity::Debug;
    if (str == "Info" || str == "info") return Severity::Info;
    if (str == "Warn" || str == "warn") return Severity::Warn;
    if (str == "Error" || str == "error") return Severity::Error;
    if (str == "Fatal" || str == "fatal") return Severity::Fatal;
    throw std::invalid_argument("Unknown severity: " + str);
}

} // namespace neo::core
