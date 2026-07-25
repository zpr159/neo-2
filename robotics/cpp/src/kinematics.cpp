#include <neo/robotics/kinematics.hpp>
#include <neo/core/error.hpp>
#include <algorithm>
#include <cmath>
#include <sstream>
#include <stdexcept>

namespace neo::robotics {

Joint::Joint(std::string name, double angle, double min_limit, double max_limit)
    : name(std::move(name)), angle(angle), min_limit(min_limit), max_limit(max_limit) {}

bool Joint::is_within_limits() const noexcept {
    return angle >= min_limit && angle <= max_limit;
}

void Joint::clamp_to_limits() noexcept {
    angle = std::max(min_limit, std::min(max_limit, angle));
}

std::string Joint::to_string() const {
    std::ostringstream oss;
    oss << "Joint{" << name << ", angle=" << angle
        << ", vel=" << velocity
        << ", limits=[" << min_limit << ", " << max_limit << "]}";
    return oss.str();
}

Pose::Pose(double x, double y, double z, double roll, double pitch, double yaw)
    : x(x), y(y), z(z), roll(roll), pitch(pitch), yaw(yaw) {}

double Pose::distance_to(const Pose& other) const noexcept {
    double dx = x - other.x;
    double dy = y - other.y;
    double dz = z - other.z;
    return std::sqrt(dx * dx + dy * dy + dz * dz);
}

void KinematicChain::add_joint(const Joint& joint) {
    joints_.push_back(joint);
}

void KinematicChain::remove_joint(const std::string& name) {
    joints_.erase(
        std::remove_if(joints_.begin(), joints_.end(),
            [&name](const Joint& j) { return j.name == name; }),
        joints_.end()
    );
}

Pose KinematicChain::forward_kinematics() const {
    std::vector<double> angles;
    angles.reserve(joints_.size());
    for (const auto& joint : joints_) {
        angles.push_back(joint.angle);
    }

    Pose result;
    result.x = compute_x(angles);
    result.y = compute_y(angles);
    result.z = 0.0;
    if (!angles.empty()) {
        result.yaw = angles.back();
    }
    return result;
}

std::vector<double> KinematicChain::inverse_kinematics(const Pose& target, double tolerance, int max_iterations) const {
    std::vector<double> angles(joints_.size(), 0.0);

    for (int iter = 0; iter < max_iterations; ++iter) {
        std::vector<double> current_angles = angles;

        double current_x = 0.0;
        double current_y = 0.0;
        double link_length = 1.0;

        for (std::size_t i = 0; i < current_angles.size(); ++i) {
            double sum = 0.0;
            for (std::size_t j = 0; j <= i; ++j) {
                sum += current_angles[j];
            }
            current_x += link_length * std::cos(sum);
            current_y += link_length * std::sin(sum);
        }

        double error_x = target.x - current_x;
        double error_y = target.y - current_y;

        if (std::sqrt(error_x * error_x + error_y * error_y) < tolerance) {
            break;
        }

        double jacobian_inv = 0.001;
        for (std::size_t i = 0; i < angles.size(); ++i) {
            angles[i] += jacobian_inv * (error_x + error_y);
            angles[i] = std::max(-M_PI, std::min(M_PI, angles[i]));
        }
    }

    return angles;
}

std::size_t KinematicChain::joint_count() const noexcept {
    return joints_.size();
}

const Joint& KinematicChain::get_joint(std::size_t index) const {
    if (index >= joints_.size()) {
        throw neo::core::Error(
            neo::core::NEO_ERR_NOT_FOUND,
            "Joint index out of range: " + std::to_string(index),
            "KinematicChain::get_joint"
        );
    }
    return joints_[index];
}

const Joint& KinematicChain::get_joint_by_name(const std::string& name) const {
    for (const auto& joint : joints_) {
        if (joint.name == name) {
            return joint;
        }
    }
    throw neo::core::Error(
        neo::core::NEO_ERR_NOT_FOUND,
        "Joint not found: " + name,
        "KinematicChain::get_joint_by_name"
    );
}

void KinematicChain::set_joint_angle(std::size_t index, double angle) {
    if (index >= joints_.size()) {
        throw neo::core::Error(
            neo::core::NEO_ERR_NOT_FOUND,
            "Joint index out of range: " + std::to_string(index),
            "KinematicChain::set_joint_angle"
        );
    }
    joints_[index].angle = angle;
    joints_[index].clamp_to_limits();
}

void KinematicChain::set_joint_angle(const std::string& name, double angle) {
    for (auto& joint : joints_) {
        if (joint.name == name) {
            joint.angle = angle;
            joint.clamp_to_limits();
            return;
        }
    }
    throw neo::core::Error(
        neo::core::NEO_ERR_NOT_FOUND,
        "Joint not found: " + name,
        "KinematicChain::set_joint_angle"
    );
}

std::string KinematicChain::to_string() const {
    std::ostringstream oss;
    oss << "KinematicChain{joints=" << joints_.size() << "}";
    return oss.str();
}

double KinematicChain::compute_x(const std::vector<double>& angles) const {
    double x = 0.0;
    double link_length = 1.0;
    double sum = 0.0;
    for (const auto& angle : angles) {
        sum += angle;
        x += link_length * std::cos(sum);
    }
    return x;
}

double KinematicChain::compute_y(const std::vector<double>& angles) const {
    double y = 0.0;
    double link_length = 1.0;
    double sum = 0.0;
    for (const auto& angle : angles) {
        sum += angle;
        y += link_length * std::sin(sum);
    }
    return y;
}

} // namespace neo::robotics
