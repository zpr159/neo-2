#include <gtest/gtest.h>
#include <neo/runtime/process.hpp>

using namespace neo::runtime;

TEST(ProcessTest, DefaultConstruction) {
    Process p;
    EXPECT_EQ(p.pid, 0u);
    EXPECT_TRUE(p.name.empty());
    EXPECT_EQ(p.state, ProcessState::Idle);
}

TEST(ProcessTest, IsRunning) {
    Process p;
    p.state = ProcessState::Running;
    EXPECT_TRUE(p.is_running());

    p.state = ProcessState::Idle;
    EXPECT_FALSE(p.is_running());
}

TEST(ProcessTest, Terminate) {
    Process p;
    p.state = ProcessState::Running;
    p.terminate();
    EXPECT_EQ(p.state, ProcessState::Failed);
}

TEST(ProcessTest, TerminateCompleted) {
    Process p;
    p.state = ProcessState::Completed;
    p.terminate();
    EXPECT_EQ(p.state, ProcessState::Completed);
}

TEST(ProcessTest, SuspendResume) {
    Process p;
    p.state = ProcessState::Running;
    p.suspend();
    EXPECT_EQ(p.state, ProcessState::Suspended);

    p.resume();
    EXPECT_EQ(p.state, ProcessState::Running);
}

TEST(ProcessTest, SuspendFromNonRunning) {
    Process p;
    p.state = ProcessState::Idle;
    p.suspend();
    EXPECT_EQ(p.state, ProcessState::Idle);
}

TEST(ProcessTest, ResumeFromNonSuspended) {
    Process p;
    p.state = ProcessState::Running;
    p.resume();
    EXPECT_EQ(p.state, ProcessState::Running);
}

TEST(ProcessTest, ToString) {
    Process p;
    p.pid = 42;
    p.name = "worker";
    p.state = ProcessState::Running;
    std::string str = p.to_string();
    EXPECT_NE(str.find("42"), std::string::npos);
    EXPECT_NE(str.find("worker"), std::string::npos);
    EXPECT_NE(str.find("Running"), std::string::npos);
}

TEST(ProcessStateTest, Tostring) {
    EXPECT_STREQ(to_string(ProcessState::Idle), "Idle");
    EXPECT_STREQ(to_string(ProcessState::Running), "Running");
    EXPECT_STREQ(to_string(ProcessState::Blocked), "Blocked");
    EXPECT_STREQ(to_string(ProcessState::Suspended), "Suspended");
    EXPECT_STREQ(to_string(ProcessState::Completed), "Completed");
    EXPECT_STREQ(to_string(ProcessState::Failed), "Failed");
}

TEST(ProcessStateTest, FromString) {
    EXPECT_EQ(process_state_from_string("Running"), ProcessState::Running);
    EXPECT_EQ(process_state_from_string("blocked"), ProcessState::Blocked);
    EXPECT_THROW(process_state_from_string("invalid"), std::invalid_argument);
}
