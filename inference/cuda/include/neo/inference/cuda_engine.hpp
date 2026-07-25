#pragma once

#include <cstddef>
#include <cstdint>
#include <memory>
#include <string>
#include <unordered_map>
#include <vector>

namespace neo::inference::cuda {

struct ModelConfig {
    std::string name;
    std::string path;
    std::uint32_t batch_size{1};
    std::uint32_t max_sequence_length{512};
    bool use_fp16{false};
};

struct InferenceResult {
    std::vector<float> logits;
    float loss{0.0f};
    std::uint32_t batch_size{0};
    double inference_time_ms{0.0};
};

class CudaInferenceEngine {
public:
    CudaInferenceEngine();
    ~CudaInferenceEngine();

    CudaInferenceEngine(const CudaInferenceEngine&) = delete;
    CudaInferenceEngine& operator=(const CudaInferenceEngine&) = delete;
    CudaInferenceEngine(CudaInferenceEngine&&) noexcept;
    CudaInferenceEngine& operator=(CudaInferenceEngine&&) noexcept;

    bool load_model(const ModelConfig& config);
    bool unload_model(const std::string& name);

    InferenceResult infer_batch(const std::string& model_name,
                                 const std::vector<std::vector<float>>& inputs);

    std::vector<float> forward(const std::string& model_name,
                                const std::vector<float>& input);

    bool is_model_loaded(const std::string& name) const noexcept;
    std::vector<std::string> loaded_models() const;

    void set_device(int device_id);
    [[nodiscard]] int device_id() const noexcept;

private:
    struct ModelState {
        ModelConfig config;
        std::vector<float> weights;
        bool loaded{false};
    };

    std::unordered_map<std::string, std::unique_ptr<ModelState>> models_;
    int device_id_{0};

    bool validate_input(const std::vector<float>& input, const ModelState& state) const;
};

} // namespace neo::inference::cuda
