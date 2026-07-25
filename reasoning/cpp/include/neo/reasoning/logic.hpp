#pragma once

#include <cstdint>
#include <string>
#include <unordered_map>
#include <vector>

namespace neo::reasoning {

struct Rule {
    std::string id;
    std::vector<std::string> premises;
    std::string conclusion;
    float confidence{1.0f};

    Rule() = default;
    Rule(std::string id, std::vector<std::string> premises, std::string conclusion, float confidence = 1.0f);

    [[nodiscard]] bool is_satisfied(const std::unordered_map<std::string, bool>& facts) const;
    [[nodiscard]] std::string to_string() const;
};

struct InferenceResult {
    std::string conclusion;
    float confidence{0.0f};
    std::vector<std::string> supporting_rules;
    bool valid{false};
};

class Reasoner {
public:
    Reasoner() = default;
    ~Reasoner() = default;

    Reasoner(const Reasoner&) = default;
    Reasoner& operator=(const Reasoner&) = default;
    Reasoner(Reasoner&&) noexcept = default;
    Reasoner& operator=(Reasoner&&) noexcept = default;

    void add_rule(const Rule& rule);
    void add_fact(const std::string& fact, bool value = true);
    void remove_fact(const std::string& fact);
    void remove_rule(const std::string& rule_id);

    [[nodiscard]] InferenceResult infer(const std::string& target);
    [[nodiscard]] std::vector<InferenceResult> infer_all();
    [[nodiscard]] bool validate(const std::string& statement);

    [[nodiscard]] std::size_t rule_count() const noexcept;
    [[nodiscard]] std::size_t fact_count() const noexcept;
    [[nodiscard]] bool has_fact(const std::string& fact) const noexcept;

    void clear();

private:
    std::vector<Rule> rules_;
    std::unordered_map<std::string, bool> facts_;

    [[nodiscard]] float compute_confidence(const Rule& rule) const;
    [[nodiscard]] bool chain_inference(const std::string& target, std::vector<std::string>& chain, float& confidence);
};

} // namespace neo::reasoning
