#include <gtest/gtest.h>
#include <neo/inference/cuda_engine.hpp>

using namespace neo::inference::cuda;

TEST(CudaInferenceEngineTest, LoadModel) {
    CudaInferenceEngine engine;
    ModelConfig config;
    config.name = "test_model";
    config.path = "/tmp/model.bin";
    config.batch_size = 4;
    config.max_sequence_length = 128;

    EXPECT_TRUE(engine.load_model(config));
    EXPECT_TRUE(engine.is_model_loaded("test_model"));
}

TEST(CudaInferenceEngineTest, LoadDuplicateModel) {
    CudaInferenceEngine engine;
    ModelConfig config;
    config.name = "test_model";
    config.path = "/tmp/model.bin";

    EXPECT_TRUE(engine.load_model(config));
    EXPECT_FALSE(engine.load_model(config));
}

TEST(CudaInferenceEngineTest, UnloadModel) {
    CudaInferenceEngine engine;
    ModelConfig config;
    config.name = "test_model";
    config.path = "/tmp/model.bin";

    engine.load_model(config);
    EXPECT_TRUE(engine.unload_model("test_model"));
    EXPECT_FALSE(engine.is_model_loaded("test_model"));
    EXPECT_FALSE(engine.unload_model("nonexistent"));
}

TEST(CudaInferenceEngineTest, Forward) {
    CudaInferenceEngine engine;
    ModelConfig config;
    config.name = "test_model";
    config.path = "/tmp/model.bin";
    config.max_sequence_length = 64;

    engine.load_model(config);

    std::vector<float> input(32, 0.5f);
    auto output = engine.forward("test_model", input);
    EXPECT_FALSE(output.empty());
}

TEST(CudaInferenceEngineTest, InferBatch) {
    CudaInferenceEngine engine;
    ModelConfig config;
    config.name = "test_model";
    config.path = "/tmp/model.bin";
    config.max_sequence_length = 64;

    engine.load_model(config);

    std::vector<std::vector<float>> inputs = {
        std::vector<float>(16, 0.5f),
        std::vector<float>(16, 0.3f)
    };

    auto result = engine.infer_batch("test_model", inputs);
    EXPECT_EQ(result.batch_size, 2u);
    EXPECT_GT(result.inference_time_ms, 0.0);
    EXPECT_FALSE(result.logits.empty());
}

TEST(CudaInferenceEngineTest, InferBatchNotFound) {
    CudaInferenceEngine engine;
    std::vector<std::vector<float>> inputs = {std::vector<float>(8, 0.1f)};
    EXPECT_THROW(engine.infer_batch("nonexistent", inputs), neo::core::Error);
}

TEST(CudaInferenceEngineTest, LoadedModels) {
    CudaInferenceEngine engine;
    ModelConfig c1;
    c1.name = "model_a";
    c1.path = "/tmp/a.bin";
    ModelConfig c2;
    c2.name = "model_b";
    c2.path = "/tmp/b.bin";

    engine.load_model(c1);
    engine.load_model(c2);

    auto names = engine.loaded_models();
    EXPECT_EQ(names.size(), 2u);
}

TEST(CudaInferenceEngineTest, DeviceId) {
    CudaInferenceEngine engine;
    EXPECT_EQ(engine.device_id(), 0);
    engine.set_device(1);
    EXPECT_EQ(engine.device_id(), 1);
}
