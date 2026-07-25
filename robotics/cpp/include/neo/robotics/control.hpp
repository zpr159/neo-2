#pragma once

#include <cstdint>

namespace neo::robotics {

class PIDController {
public:
    PIDController();
    PIDController(double kp, double ki, double kd);
    ~PIDController() = default;

    PIDController(const PIDController&) = default;
    PIDController& operator=(const PIDController&) = default;
    PIDController(PIDController&&) noexcept = default;
    PIDController& operator=(PIDController&&) noexcept = default;

    [[nodiscard]] double compute(double input, double dt);
    void reset() noexcept;

    void set_gains(double kp, double ki, double kd);
    void set_setpoint(double setpoint) noexcept;
    void set_output_limits(double min, double max) noexcept;

    [[nodiscard]] double kp() const noexcept;
    [[nodiscard]] double ki() const noexcept;
    [[nodiscard]] double kd() const noexcept;
    [[nodiscard]] double setpoint() const noexcept;
    [[nodiscard]] double integral() const noexcept;
    [[nodiscard]] double prev_error() const noexcept;

private:
    double kp_{1.0};
    double ki_{0.0};
    double kd_{0.0};
    double setpoint_{0.0};
    double integral_{0.0};
    double prev_error_{0.0};
    double output_min_{-1000.0};
    double output_max_{1000.0};
    bool first_update_{true};
};

} // namespace neo::robotics
