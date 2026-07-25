#include <gtest/gtest.h>
#include <neo/distributed/node.hpp>
#include <neo/distributed/message.hpp>

using namespace neo::distributed;

TEST(ClusterNodeTest, DefaultConstruction) {
    ClusterNode node;
    EXPECT_EQ(node.id, 0u);
    EXPECT_TRUE(node.address.empty());
    EXPECT_EQ(node.state, NodeState::Unknown);
}

TEST(ClusterNodeTest, Creation) {
    ClusterNode node(1, "192.168.1.1:8080");
    EXPECT_EQ(node.id, 1u);
    EXPECT_EQ(node.address, "192.168.1.1:8080");
    EXPECT_TRUE(node.is_reachable());
}

TEST(ClusterNodeTest, HealthScore) {
    ClusterNode node(1, "localhost");
    node.state = NodeState::Healthy;
    EXPECT_FLOAT_EQ(node.health_score(), 1.0f);

    node.state = NodeState::Degraded;
    EXPECT_FLOAT_EQ(node.health_score(), 0.5f);

    node.state = NodeState::Unreachable;
    EXPECT_FLOAT_EQ(node.health_score(), 0.0f);
}

TEST(ClusterNodeTest, RecordFailure) {
    ClusterNode node(1, "localhost");
    node.state = NodeState::Healthy;

    node.record_failure();
    EXPECT_EQ(node.state, NodeState::Degraded);
    EXPECT_EQ(node.failure_count, 1u);

    node.record_failure();
    node.record_failure();
    EXPECT_EQ(node.state, NodeState::Unreachable);
}

TEST(ClusterNodeTest, ResetHealth) {
    ClusterNode node(1, "localhost");
    node.record_failure();
    node.record_failure();
    node.reset_health();
    EXPECT_EQ(node.state, NodeState::Healthy);
    EXPECT_EQ(node.failure_count, 0u);
}

TEST(ClusterNodeTest, Tostring) {
    ClusterNode node(42, "10.0.0.1:9090");
    std::string str = node.to_string();
    EXPECT_NE(str.find("42"), std::string::npos);
    EXPECT_NE(str.find("10.0.0.1:9090"), std::string::npos);
}

TEST(MessageTest, Creation) {
    Message msg(1, 10, 20, MessageType::Heartbeat, "ping");
    EXPECT_EQ(msg.id, 1u);
    EXPECT_EQ(msg.from, 10u);
    EXPECT_EQ(msg.to, 20u);
    EXPECT_EQ(msg.type, MessageType::Heartbeat);
    EXPECT_EQ(msg.payload, "ping");
}

TEST(MessageTest, IsValid) {
    Message valid(1, 10, 20, MessageType::Command, "do_something");
    EXPECT_TRUE(valid.is_valid());

    Message self_msg(1, 10, 10, MessageType::Ping, "ping");
    EXPECT_FALSE(self_msg.is_valid());

    Message empty_payload(1, 10, 20, MessageType::DataSync, "");
    EXPECT_FALSE(empty_payload.is_valid());
}

TEST(MessageTest, MessageTypeString) {
    EXPECT_STREQ(to_string(MessageType::Heartbeat), "Heartbeat");
    EXPECT_STREQ(to_string(MessageType::Command), "Command");
    EXPECT_STREQ(to_string(MessageType::Vote), "Vote");
}

TEST(MessageTest, Tostring) {
    Message msg(1, 10, 20, MessageType::DataSync, "hello");
    std::string str = msg.to_string();
    EXPECT_NE(str.find("10"), std::string::npos);
    EXPECT_NE(str.find("DataSync"), std::string::npos);
}
