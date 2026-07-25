#include <neo/reasoning/logic.hpp>
#include <algorithm>
#include <sstream>
#include <unordered_set>

namespace neo::reasoning {

Rule::Rule(std::string id, std::vector<std::string> premises, std::string conclusion, float confidence)
    : id(std::move(id)), premises(std::move(premises)), conclusion(std::move(conclusion)),
      confidence(confidence) {}

bool Rule::is_satisfied(const std::unordered_map<std::string, bool>& known_facts) const {
    for (const auto& premise : premises) {
        auto it = known_facts.find(premise);
        if (it == known_facts.end() || !it->second) {
            return false;
        }
    }
    return true;
}

std::string Rule::to_string() const {
    std::ostringstream oss;
    oss << "Rule{id=" << id << ", premises=[";
    for (std::size_t i = 0; i < premises.size(); ++i) {
        if (i > 0) oss << ", ";
        oss << premises[i];
    }
    oss << "], conclusion=" << conclusion
        << ", confidence=" << confidence << "}";
    return oss.str();
}

void Reasoner::add_rule(const Rule& rule) {
    rules_.push_back(rule);
}

void Reasoner::add_fact(const std::string& fact, bool value) {
    facts_[fact] = value;
}

void Reasoner::remove_fact(const std::string& fact) {
    facts_.erase(fact);
}

void Reasoner::remove_rule(const std::string& rule_id) {
    rules_.erase(
        std::remove_if(rules_.begin(), rules_.end(),
            [&rule_id](const Rule& r) { return r.id == rule_id; }),
        rules_.end()
    );
}

InferenceResult Reasoner::infer(const std::string& target) {
    InferenceResult result;
    std::vector<std::string> chain;
    float confidence = 0.0f;

    if (chain_inference(target, chain, confidence)) {
        result.conclusion = target;
        result.confidence = confidence;
        result.supporting_rules = std::move(chain);
        result.valid = true;
        facts_[target] = true;
    }
    return result;
}

std::vector<InferenceResult> Reasoner::infer_all() {
    std::vector<InferenceResult> results;
    std::unordered_set<std::string> inferred;

    bool changed = true;
    while (changed) {
        changed = false;
        for (const auto& rule : rules_) {
            if (inferred.contains(rule.id)) continue;
            if (rule.is_satisfied(facts_) && !facts_.contains(rule.conclusion)) {
                facts_[rule.conclusion] = true;
                inferred.insert(rule.id);
                InferenceResult result;
                result.conclusion = rule.conclusion;
                result.confidence = rule.confidence;
                result.supporting_rules.push_back(rule.id);
                result.valid = true;
                results.push_back(std::move(result));
                changed = true;
            }
        }
    }
    return results;
}

bool Reasoner::validate(const std::string& statement) {
    auto it = facts_.find(statement);
    if (it != facts_.end() && it->second) {
        return true;
    }

    for (const auto& rule : rules_) {
        if (rule.conclusion == statement && rule.is_satisfied(facts_)) {
            return true;
        }
    }
    return false;
}

std::size_t Reasoner::rule_count() const noexcept {
    return rules_.size();
}

std::size_t Reasoner::fact_count() const noexcept {
    return facts_.size();
}

bool Reasoner::has_fact(const std::string& fact) const noexcept {
    auto it = facts_.find(fact);
    return it != facts_.end() && it->second;
}

void Reasoner::clear() {
    rules_.clear();
    facts_.clear();
}

float Reasoner::compute_confidence(const Rule& rule) const {
    float min_premise_conf = 1.0f;
    for (const auto& premise : rule.premises) {
        if (facts_.contains(premise)) {
            min_premise_conf = std::min(min_premise_conf, 1.0f);
        } else {
            min_premise_conf = 0.0f;
        }
    }
    return rule.confidence * min_premise_conf;
}

bool Reasoner::chain_inference(const std::string& target, std::vector<std::string>& chain, float& confidence) {
    if (facts_.contains(target)) {
        confidence = 1.0f;
        return true;
    }

    for (auto& rule : rules_) {
        if (rule.conclusion == target) {
            bool all_satisfied = true;
            float min_conf = 1.0f;

            for (const auto& premise : rule.premises) {
                if (!facts_.contains(premise)) {
                    float sub_conf = 0.0f;
                    if (chain_inference(premise, chain, sub_conf)) {
                        min_conf = std::min(min_conf, sub_conf);
                    } else {
                        all_satisfied = false;
                        break;
                    }
                }
            }

            if (all_satisfied) {
                confidence = rule.confidence * min_conf;
                chain.push_back(rule.id);
                facts_[target] = true;
                return true;
            }
        }
    }
    return false;
}

} // namespace neo::reasoning
