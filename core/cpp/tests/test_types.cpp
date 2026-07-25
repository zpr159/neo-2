#include <gtest/gtest.h>
#include <neo/core/types.hpp>

using namespace neo::core;

TEST(VersionTest, DefaultConstruction) {
    Version v{};
    EXPECT_EQ(v.major, 0u);
    EXPECT_EQ(v.minor, 0u);
    EXPECT_EQ(v.patch, 0u);
    EXPECT_TRUE(v.pre_release.empty());
    EXPECT_TRUE(v.build.empty());
}

TEST(VersionTest, Tostring) {
    Version v{1, 2, 3, "", ""};
    EXPECT_EQ(v.to_string(), "1.2.3");
}

TEST(VersionTest, TostringWithPrerelease) {
    Version v{1, 0, 0, "alpha", ""};
    EXPECT_EQ(v.to_string(), "1.0.0-alpha");
}

TEST(VersionTest, TostringWithBuild) {
    Version v{1, 0, 0, "", "build.123"};
    EXPECT_EQ(v.to_string(), "1.0.0+build.123");
}

TEST(VersionTest, Equality) {
    Version a{1, 2, 3, "", ""};
    Version b{1, 2, 3, "", ""};
    Version c{1, 2, 4, "", ""};
    EXPECT_EQ(a, b);
    EXPECT_NE(a, c);
}

TEST(VersionTest, Comparison) {
    Version v1{1, 0, 0, "", ""};
    Version v2{2, 0, 0, "", ""};
    Version v3{1, 1, 0, "", ""};
    EXPECT_LT(v1, v2);
    EXPECT_LT(v1, v3);
    EXPECT_GT(v2, v1);
    EXPECT_LE(v1, v1);
    EXPECT_GE(v1, v1);
}

TEST(VersionTest, PrereleaseOrdering) {
    Version alpha{1, 0, 0, "alpha", ""};
    Version beta{1, 0, 0, "beta", ""};
    Version release{1, 0, 0, "", ""};
    EXPECT_LT(alpha, beta);
    EXPECT_LT(beta, release);
}

TEST(VersionTest, OutputOperator) {
    Version v{3, 4, 5, "rc1", ""};
    std::ostringstream oss;
    oss << v;
    EXPECT_EQ(oss.str(), "3.4.5-rc1");
}

TEST(EnvironmentTest, Tostring) {
    EXPECT_STREQ(to_string(Environment::Development), "Development");
    EXPECT_STREQ(to_string(Environment::Testing), "Testing");
    EXPECT_STREQ(to_string(Environment::Staging), "Staging");
    EXPECT_STREQ(to_string(Environment::Production), "Production");
}

TEST(EnvironmentTest, FromString) {
    EXPECT_EQ(environment_from_string("Development"), Environment::Development);
    EXPECT_EQ(environment_from_string("prod"), Environment::Production);
    EXPECT_EQ(environment_from_string("test"), Environment::Testing);
    EXPECT_THROW(environment_from_string("invalid"), std::invalid_argument);
}

TEST(SeverityTest, Tostring) {
    EXPECT_STREQ(to_string(Severity::Trace), "Trace");
    EXPECT_STREQ(to_string(Severity::Debug), "Debug");
    EXPECT_STREQ(to_string(Severity::Info), "Info");
    EXPECT_STREQ(to_string(Severity::Warn), "Warn");
    EXPECT_STREQ(to_string(Severity::Error), "Error");
    EXPECT_STREQ(to_string(Severity::Fatal), "Fatal");
}

TEST(SeverityTest, FromString) {
    EXPECT_EQ(severity_from_string("info"), Severity::Info);
    EXPECT_EQ(severity_from_string("Error"), Severity::Error);
    EXPECT_EQ(severity_from_string("fatal"), Severity::Fatal);
    EXPECT_THROW(severity_from_string("unknown"), std::invalid_argument);
}
