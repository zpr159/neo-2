#include <gtest/gtest.h>
#include <neo/nn/cuda_layers.hpp>
#include <cmath>
#include <vector>

using namespace neo::nn::cuda;

TEST(CudaDenseTest, Forward) {
    int batch_size = 2, in_features = 3, out_features = 2;
    float input[] = {1, 2, 3, 4, 5, 6};
    float weight[] = {1, 0, 1, 0, 1, 1};
    float bias[] = {0.1f, 0.2f};
    float output[4] = {0};

    cuda_dense_forward(input, weight, bias, output, batch_size, in_features, out_features);

    EXPECT_FLOAT_EQ(output[0], 4.1f);
    EXPECT_FLOAT_EQ(output[1], 5.2f);
    EXPECT_FLOAT_EQ(output[2], 10.1f);
    EXPECT_FLOAT_EQ(output[3], 15.2f);
}

TEST(CudaBatchNormTest, Normalize) {
    int batch_size = 2, features = 3;
    float input[] = {1, 2, 3, 4, 5, 6};
    float gamma[] = {1, 1, 1};
    float beta[] = {0, 0, 0};
    float mean[] = {2.5f, 3.5f, 4.5f};
    float var[] = {2.25f, 2.25f, 2.25f};
    float output[6] = {0};

    cuda_batch_norm(input, gamma, beta, mean, var, output, batch_size, features);

    for (int i = 0; i < 6; ++i) {
        EXPECT_NEAR(output[i], 0.0f, 1e-5f);
    }
}

TEST(CudaMaxPoolTest, Pool2x2) {
    int batch = 1, channels = 1, h = 4, w = 4;
    float input[] = {
        1, 2, 3, 4,
        5, 6, 7, 8,
        9, 10, 11, 12,
        13, 14, 15, 16
    };
    float output[4] = {0};

    cuda_max_pool2d(input, output, batch, channels, h, w, 2, 2);

    EXPECT_FLOAT_EQ(output[0], 6.0f);
    EXPECT_FLOAT_EQ(output[1], 8.0f);
    EXPECT_FLOAT_EQ(output[2], 14.0f);
    EXPECT_FLOAT_EQ(output[3], 16.0f);
}

TEST(CudaAvgPoolTest, Pool2x2) {
    int batch = 1, channels = 1, h = 2, w = 2;
    float input[] = {1, 2, 3, 4};
    float output[1] = {0};

    cuda_avg_pool2d(input, output, batch, channels, h, w, 2, 2);

    EXPECT_FLOAT_EQ(output[0], 2.5f);
}

TEST(CudaSoftmaxCrossEntropyTest, Basic) {
    int batch = 1, classes = 3;
    float logits[] = {1.0f, 2.0f, 3.0f};
    float targets[] = {0, 0, 1.0f};
    float output[1] = {0};

    cuda_softmax_cross_entropy(logits, targets, output, batch, classes);

    EXPECT_GT(output[0], 0.0f);
    EXPECT_LT(output[0], 1.0f);
}
