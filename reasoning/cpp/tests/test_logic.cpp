#include <gtest/gtest.h>
#include <neo/reasoning/logic.hpp>

using namespace neo::reasoning;

TEST(RuleTest, Creation) {
    Rule r("r1", {"a", "b"}, "c", 0.9f);
    EXPECT_EQ(r.id, "r1");
    EXPECT_EQ(r.premises.size(), 2u);
    EXPECT_EQ(r.conclusion, "c");
    EXPECT_FLOAT_EQ(r.confidence, 0.9f);
}

TEST(RuleTest, IsSatisfied) {
    Rule r("r1", {"a", "b"}, "c");
    std::unordered_map<std::string, bool> facts = {{"a", true}, {"b", true}};
    EXPECT_TRUE(r.is_satisfied(facts));

    std::unordered_map<std::string, bool> partial = {{"a", true}};
    EXPECT_FALSE(r.is_satisfied(partial));

    std::unordered_map<std::string, bool> negated = {{"a", true}, {"b", false}};
    EXPECT_FALSE(r.is_satisfied(negated));
}

TEST(RuleTest, Tostring) {
    Rule r("r1", {"a", "b"}, "c", 0.8f);
    std::string str = r.to_string();
    EXPECT_NE(str.find("r1"), std::string::npos);
    EXPECT_NE(str.find("c"), std::string::npos);
}

TEST(ReasonerTest, AddFacts) {
    Reasoner reasoner;
    reasoner.add_fact("sky_blue");
    EXPECT_TRUE(reasoner.has_fact("sky_blue"));
    EXPECT_EQ(reasoner.fact_count(), 1u);
}

TEST(ReasonerTest, AddRemoveRule) {
    Reasoner reasoner;
    reasoner.add_rule(Rule("r1", {"a"}, "b"));
    EXPECT_EQ(reasoner.rule_count(), 1u);

    reasoner.remove_rule("r1");
    EXPECT_EQ(reasoner.rule_count(), 0u);
}

TEST(ReasonerTest, SimpleInference) {
    Reasoner reasoner;
    reasoner.add_fact("has_wings");
    reasoner.add_rule(Rule("r1", {"has_wings"}, "can_fly"));

    auto result = reasoner.infer("can_fly");
    EXPECT_TRUE(result.valid);
    EXPECT_EQ(result.conclusion, "can_fly");
    EXPECT_TRUE(reasoner.has_fact("can_fly"));
}

TEST(ReasonerTest, ChainInference) {
    Reasoner reasoner;
    reasoner.add_fact("feathers");
    reasoner.add_rule(Rule("r1", {"feathers"}, "has_wings"));
    reasoner.add_rule(Rule("r2", {"has_wings"}, "can_fly"));

    auto result = reasoner.infer("can_fly");
    EXPECT_TRUE(result.valid);
    EXPECT_EQ(result.conclusion, "can_fly");
}

TEST(ReasonerTest, InferenceFailure) {
    Reasoner reasoner;
    reasoner.add_rule(Rule("r1", {"a"}, "b"));

    auto result = reasoner.infer("b");
    EXPECT_FALSE(result.valid);
}

TEST(ReasonerTest, InferAll) {
    Reasoner reasoner;
    reasoner.add_fact("a");
    reasoner.add_rule(Rule("r1", {"a"}, "b"));
    reasoner.add_rule(Rule("r2", {"b"}, "c"));

    auto results = reasoner.infer_all();
    EXPECT_GE(results.size(), 2u);
}

TEST(ReasonerTest, Validate) {
    Reasoner reasoner;
    reasoner.add_fact("x");
    reasoner.add_rule(Rule("r1", {"x"}, "y"));

    EXPECT_TRUE(reasoner.validate("x"));
    EXPECT_TRUE(reasoner.validate("y"));
    EXPECT_FALSE(reasoner.validate("z"));
}

TEST(ReasonerTest, Clear) {
    Reasoner reasoner;
    reasoner.add_fact("a");
    reasoner.add_rule(Rule("r1", {"a"}, "b"));
    reasoner.clear();
    EXPECT_EQ(reasoner.rule_count(), 0u);
    EXPECT_EQ(reasoner.fact_count(), 0u);
}
