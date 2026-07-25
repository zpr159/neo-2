#include <gtest/gtest.h>
#include <neo/core/error.hpp>

using namespace neo::core;

TEST(ErrorTest, Creation) {
    Error err(NEO_ERR_GENERAL, "something failed", "TestModule");
    EXPECT_EQ(err.code(), NEO_ERR_GENERAL);
    EXPECT_EQ(err.message(), "something failed");
    EXPECT_EQ(err.source(), "TestModule");
}

TEST(ErrorTest, WhatMessage) {
    Error err(NEO_ERR_TIMEOUT, "timed out", "Network");
    std::string what_str = err.what();
    EXPECT_NE(what_str.find("Network"), std::string::npos);
    EXPECT_NE(what_str.find("timed out"), std::string::npos);
    EXPECT_NE(what_str.find("1001"), std::string::npos);
}

TEST(ErrorTest, Category) {
    Error err(NEO_ERR_PERMISSION, "denied", "Security");
    err.set_category(ErrorCategory::Security);
    EXPECT_EQ(err.category(), ErrorCategory::Security);

    EXPECT_STREQ(error_category_name(ErrorCategory::System), "System");
    EXPECT_STREQ(error_category_name(ErrorCategory::Neural), "Neural");
    EXPECT_STREQ(error_category_name(ErrorCategory::IO), "IO");
}

TEST(ErrorTest, FormatError) {
    Error err(NEO_ERR_RESOURCE, "out of memory", "Allocator");
    err.set_category(ErrorCategory::Runtime);
    std::string formatted = format_error(err);
    EXPECT_NE(formatted.find("Runtime"), std::string::npos);
    EXPECT_NE(formatted.find("Allocator"), std::string::npos);
    EXPECT_NE(formatted.find("out of memory"), std::string::npos);
}

TEST(ErrorTest, OkCode) {
    EXPECT_EQ(NEO_OK, 0);
    EXPECT_EQ(NEO_ERR_GENERAL, 1000);
    EXPECT_EQ(NEO_ERR_TIMEOUT, 1001);
    EXPECT_EQ(NEO_ERR_NOT_FOUND, 1002);
    EXPECT_EQ(NEO_ERR_PERMISSION, 1003);
    EXPECT_EQ(NEO_ERR_RESOURCE, 1004);
}

TEST(ErrorTest, CopySemantics) {
    Error original(42, "original", "Src");
    Error copy = original;
    EXPECT_EQ(copy.code(), 42);
    EXPECT_EQ(copy.message(), "original");
    EXPECT_EQ(copy.source(), "Src");
}

TEST(ErrorTest, MoveSemantics) {
    Error original(99, "movable", "Src");
    Error moved = std::move(original);
    EXPECT_EQ(moved.code(), 99);
    EXPECT_EQ(moved.message(), "movable");
}
