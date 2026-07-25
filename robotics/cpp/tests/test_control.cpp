#include <gtest/gtest.h>
#include <neo/robotics/kinematics.hpp>
#include <neo/robotics/control.hpp>

using namespace neo::robotics;

TEST(JointTest, Creation) {
    Joint j("shoulder", 0.5, -M_PI, M_PI);
    EXPECT_EQ(j.name, "shoulder");
    EXPECT_DOUBLE_EQ(j.angle, 0.5);
    EXPECT_TRUE(j.is_within_limits());
}

TEST(JointTest, OutOfLimits) {
    Joint j("elbow", 5.0, -1.0, 1.0);
    EXPECT_FALSE(j.is_within_limits());
    j.clamp_to_limits();
    EXPECT_DOUBLE_EQ(j.angle, 1.0);
}

TEST(JointTest, Tostring) {
    Joint j("wrist", 0.0);
    std::string str = j.to_string();
    EXPECT_NE(str.find("wrist"), std::string::npos);
}

TEST(PoseTest, Distance) {
    Pose a(0, 0, 0);
    Pose b(3, 4, 0);
    EXPECT_DOUBLE_EQ(a.distance_to(b), 5.0);
}

TEST(KinematicChainTest, AddRemoveJoint) {
    KinematicChain chain;
    chain.add_joint(Joint("j1", 0.0));
    chain.add_joint(Joint("j2", 0.5));
    EXPECT_EQ(chain.joint_count(), 2u);

    chain.remove_joint("j1");
    EXPECT_EQ(chain.joint_count(), 1u);
}

TEST(KinematicChainTest, GetJoint) {
    KinematicChain chain;
    chain.add_joint(Joint("j1", 1.0));
    EXPECT_DOUBLE_EQ(chain.get_joint(0).angle, 1.0);
    EXPECT_DOUBLE_EQ(chain.get_joint_by_name("j1").angle, 1.0);
}

TEST(KinematicChainTest, SetJointAngle) {
    KinematicChain chain;
    chain.add_joint(Joint("j1", 0.0, -1.0, 1.0));
    chain.set_joint_angle(0, 0.5);
    EXPECT_DOUBLE_EQ(chain.get_joint(0).angle, 0.5);

    chain.set_joint_angle("j1", 5.0);
    EXPECT_DOUBLE_EQ(chain.get_joint(0).angle, 1.0);
}

TEST(KinematicChainTest, ForwardKinematics) {
    KinematicChain chain;
    chain.add_joint(Joint("j1", 0.0));
    chain.add_joint(Joint("j2", 0.0));
    Pose result = chain.forward_kinematics();
    EXPECT_DOUBLE_EQ(result.x, 2.0);
    EXPECT_DOUBLE_EQ(result.y, 0.0);
}

TEST(KinematicChainTest, InverseKinematics) {
    KinematicChain chain;
    chain.add_joint(Joint("j1", 0.0));
    chain.add_joint(Joint("j2", 0.0));
    Pose target(1.0, 1.0, 0.0);
    auto angles = chain.inverse_kinematics(target);
    EXPECT_EQ(angles.size(), 2u);
}

TEST(PIDControllerTest, Creation) {
    PIDController pid;
    EXPECT_DOUBLE_EQ(pid.kp(), 1.0);
    EXPECT_DOUBLE_EQ(pid.ki(), 0.0);
    EXPECT_DOUBLE_EQ(pid.kd(), 0.0);
}

TEST(PIDControllerTest, Compute) {
    PIDController pid(1.0, 0.1, 0.01);
    pid.set_setpoint(10.0);

    double output = pid.compute(0.0, 0.1);
    EXPECT_GT(output, 0.0);

    output = pid.compute(10.0, 0.1);
    EXPECT_NEAR(output, 0.0, 0.1);
}

TEST(PIDControllerTest, Reset) {
    PIDController pid(1.0, 0.1, 0.01);
    pid.set_setpoint(10.0);
    pid.compute(0.0, 0.1);
    pid.reset();
    EXPECT_DOUBLE_EQ(pid.integral(), 0.0);
    EXPECT_DOUBLE_EQ(pid.prev_error(), 0.0);
}

TEST(PIDControllerTest, OutputLimits) {
    PIDController pid(100.0, 0.0, 0.0);
    pid.set_output_limits(-5.0, 5.0);
    pid.set_setpoint(100.0);

    double output = pid.compute(0.0, 0.1);
    EXPECT_LE(output, 5.0);
    EXPECT_GE(output, -5.0);
}

TEST(PIDControllerTest, SetGains) {
    PIDController pid;
    pid.set_gains(2.0, 0.5, 0.1);
    EXPECT_DOUBLE_EQ(pid.kp(), 2.0);
    EXPECT_DOUBLE_EQ(pid.ki(), 0.5);
    EXPECT_DOUBLE_EQ(pid.kd(), 0.1);
}
