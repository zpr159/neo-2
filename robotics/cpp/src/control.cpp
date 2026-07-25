#include <neo/robotics/control.hpp>
#include <algorithm>
#include <cmath>

namespace neo::robotics {

PIDController::PIDController() = default;

PIDController::PIDController(double kp, double ki, double kd)
    : kp_(kp), ki_(ki), kd_(kd) {}

double PIDController::compute(double input, double dt) {
    double error = setpoint_ - input;

    if (first_update_) {
        prev_error_ = error;
        first_update_ = false;
    }

    integral_ += error * dt;
    double derivative = (error - prev_error_) / dt;

    double output = kp_ * error + ki_ * integral_ + kd_ * derivative;
    output = std::max(output_min_, std::min(output_max_, output));

    prev_error_ = error;
    return output;
}

void PIDController::reset() noexcept {
    integral_ = 0.0;
    prev_error_ = 0.0;
    first_update_ = true;
}

void PIDController::set_gains(double kp, double ki, double kd) {
    kp_ = kp;
    ki_ = ki;
    kd_ = kd;
}

void PIDController::set_setpoint(double setpoint) noexcept {
    setpoint_ = setpoint;
}

void PIDController::set_output_limits(double min, double max) noexcept {
    output_min_ = min;
    output_max_ = max;
}

double PIDController::kp() const noexcept { return kp_; }
double PIDController::ki() const noexcept { return ki_; }
double PIDController::kd() const noexcept { return kd_; }
double PIDController::setpoint() const noexcept { return setpoint_; }
double PIDController::integral() const noexcept { return integral_; }
double PIDController::prev_error() const noexcept { return prev_error_; }

} // namespace neo::robotics
