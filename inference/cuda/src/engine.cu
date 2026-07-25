#include <neo/inference/cuda_engine.hpp>
#include <neo/core/error.hpp>
#include <chrono>
#include <cmath>
#include <cstring>
#include <fstream>
#include <numeric>
#include <random>

namespace neo::inference::cuda {

CudaInferenceEngine::CudaInferenceEngine() = default;

CudaInferenceEngine::~CudaInferenceEngine() = default;

CudaInferenceEngine::CudaInferenceEngine(CudaInferenceEngine&&) noexcept = default;

CudaInferenceEngine& CudaInferenceEngine::operator=(CudaInferenceEngine&&) noexcept = default;

bool CudaInferenceEngine::load_model(const ModelConfig& config) {
    if (models_.contains(config.name)) {
        return false;
    }

    auto state = std::make_unique<ModelState>();
    state->config = config;

    std::mt19937 gen(42);
    std::normal_distribution<> dis(0.0, 0.02);

    std::size_t weight_size = config.max_sequence_length * 768;
    state->weights.resize(weight_size);
    for (auto& w : state->weights) {
        w = static_cast<float>(dis(gen));
    }

    state->loaded = true;
    models_[config.name] = std::move(state);
    return true;
}

bool CudaInferenceEngine::unload_model(const std::string& name) {
    return models_.erase(name) > 0;
}

InferenceResult CudaInferenceEngine::infer_batch(const std::string& model_name,
                                                   const std::vector<std::vector<float>>& inputs) {
    auto start = std::chrono::high_resolution_clock::now();

    InferenceResult result;
    result.batch_size = static_cast<std::uint32_t>(inputs.size());

    auto it = models_.find(model_name);
    if (it == models_.end() || !it->second->loaded) {
        throw neo::core::Error(
            neo::core::NEO_ERR_NOT_FOUND,
            "Model not found or not loaded: " + model_name,
            "CudaInferenceEngine::infer_batch"
        );
    }

    const auto& state = it->second;
    std::size_t input_dim = state->config.max_sequence_length;
    std::size_t output_dim = state->weights.size() / input_dim;

    if (output_dim == 0) output_dim = input_dim;

    result.logits.resize(inputs.size() * output_dim, 0.0f);

    for (std::size_t b = 0; b < inputs.size(); ++b) {
        const auto& input = inputs[b];
        if (input.size() > input_dim) {
            throw neo::core::Error(
                neo::core::NEO_ERR_GENERAL,
                "Input size exceeds model max sequence length",
                "CudaInferenceEngine::infer_batch"
            );
        }

        for (std::size_t o = 0; o < output_dim; ++o) {
            float sum = 0.0f;
            for (std::size_t i = 0; i < input.size(); ++i) {
                sum += input[i] * state->weights[o * input_dim + i];
            }
            result.logits[b * output_dim + o] = sum;
        }

        float max_logit = *std::max_element(
            result.logits.begin() + b * output_dim,
            result.logits.begin() + (b + 1) * output_dim
        );

        float sum_exp = 0.0f;
        for (std::size_t o = 0; o < output_dim; ++o) {
            float val = std::exp(result.logits[b * output_dim + o] - max_logit);
            result.logits[b * output_dim + o] = val;
            sum_exp += val;
        }

        for (std::size_t o = 0; o < output_dim; ++o) {
            result.logits[b * output_dim + o] /= sum_exp;
        }
    }

    auto end = std::chrono::high_resolution_clock::now();
    result.inference_time_ms = std::chrono::duration<double, std::milli>(end - start).count();

    return result;
}

std::vector<float> CudaInferenceEngine::forward(const std::string& model_name,
                                                   const std::vector<float>& input) {
    std::vector<std::vector<float>> batch = {input};
    auto result = infer_batch(model_name, batch);
    return result.logits;
}

bool CudaInferenceEngine::is_model_loaded(const std::string& name) const noexcept {
    auto it = models_.find(name);
    return it != models_.end() && it->second->loaded;
}

std::vector<std::string> CudaInferenceEngine::loaded_models() const {
    std::vector<std::string> names;
    names.reserve(models_.size());
    for (const auto& [name, state] : models_) {
        if (state->loaded) {
            names.push_back(name);
        }
    }
    return names;
}

void CudaInferenceEngine::set_device(int device_id) {
    device_id_ = device_id;
}

int CudaInferenceEngine::device_id() const noexcept {
    return device_id_;
}

bool CudaInferenceEngine::validate_input(const std::vector<float>& input, const ModelState& state) const {
    return input.size() <= state.config.max_sequence_length;
}

} // namespace neo::inference::cuda
