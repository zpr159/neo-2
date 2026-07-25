#pragma once

#include <cmath>
#include <cstdint>
#include <string>
#include <vector>

namespace neo::robotics {

struct Joint {
    std::string name;
    double angle{0.0};
    double velocity{0.0};
    double min_limit{-M_PI};
    double max_limit{M_PI};

    Joint() = default;
    Joint(std::string name, double angle, double min_limit = -M_PI, double max_limit = M_PI);

    [[nodiscard]] bool is_within_limits() const noexcept;
    void clamp_to_limits() noexcept;

    [[nodiscard]] std::string to_string() const;
};

struct Pose {
    double x{0.0};
    double y{0.0};
    double z{0.0};
    double roll{0.0};
    double pitch{0.0};
    double yaw{0.0};

    Pose() = default;
    Pose(double x, double y, double z, double roll = 0, double pitch = 0, double yaw = 0);

    [[nodiscard]] double distance_to(const Pose& other) const noexcept;
};

class KinematicChain {
public:
    KinematicChain() = default;
    ~KinematicChain() = default;

    KinematicChain(const KinematicChain&) = default;
    KinematicChain& operator=(const KinematicChain&) = default;
    KinematicChain(KinematicChain&&) noexcept = default;
    KinematicChain& operator=(KinematicChain&&) noexcept = default;

    void add_joint(const Joint& joint);
    void remove_joint(const std::string& name);

    [[nodiscard]] Pose forward_kinematics() const;
    [[nodiscard]] std::vector<double> inverse_kinematics(const Pose& target, double tolerance = 1e-6, int max_iterations = 100) const;

    [[nodiscard]] std::size_t joint_count() const noexcept;
    [[nodiscard]] const Joint& get_joint(std::size_t index) const;
    [[nodiscard]] const Joint& get_joint_by_name(const std::string& name) const;

    void set_joint_angle(std::size_t index, double angle);
    void set_joint_angle(const std::string& name, double angle);

    [[nodiscard]] std::string to_string() const;

private:
    std::vector<Joint> joints_;

    [[nodiscard]] double compute_x(const std::vector<double>& angles) const;
    [[nodiscard]] double compute_y(const std::vector<double>& angles) const;
};

} // namespace neo::robotics
