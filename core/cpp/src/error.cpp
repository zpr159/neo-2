#include <neo/core/error.hpp>
#include <sstream>

namespace neo::core {

Error::Error(std::int32_t code, std::string message, std::string source)
    : code_(code), message_(std::move(message)), source_(std::move(source)) {
    std::ostringstream oss;
    oss << "[" << source_ << "] (code=" << code_ << ") " << message_;
    what_ = oss.str();
}

const char* Error::what() const noexcept {
    return what_.c_str();
}

std::int32_t Error::code() const noexcept {
    return code_;
}

const std::string& Error::message() const noexcept {
    return message_;
}

const std::string& Error::source() const noexcept {
    return source_;
}

ErrorCategory Error::category() const noexcept {
    return category_;
}

void Error::set_category(ErrorCategory category) noexcept {
    category_ = category;
    std::ostringstream oss;
    oss << "[" << source_ << "] (" << error_category_name(category_) << ") (code=" << code_ << ") " << message_;
    what_ = oss.str();
}

const char* error_category_name(ErrorCategory cat) noexcept {
    switch (cat) {
        case ErrorCategory::System: return "System";
        case ErrorCategory::Runtime: return "Runtime";
        case ErrorCategory::Network: return "Network";
        case ErrorCategory::Security: return "Security";
        case ErrorCategory::Neural: return "Neural";
        case ErrorCategory::IO: return "IO";
    }
    return "Unknown";
}

std::string format_error(const Error& err) {
    std::ostringstream oss;
    oss << "[" << error_category_name(err.category()) << "] "
        << err.source() << ": " << err.message()
        << " (code=" << err.code() << ")";
    return oss.str();
}

} // namespace neo::core
