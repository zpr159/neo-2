#include <neo/neural/cuda_ops.hpp>
#include <cmath>
#include <cstring>
#include <vector>
#include <gtest/gtest.h>

using namespace neo::neural::cuda;

TEST(CudaOpsTest, Matmul) {
    const int M = 2, N = 2, K = 2;
    float A[] = {1, 2, 3, 4};
    float B[] = {5, 6, 7, 8};
    float C[4] = {0};

    cuda_matmul(A, B, C, M, N, K);

    EXPECT_FLOAT_EQ(C[0], 19.0f);
    EXPECT_FLOAT_EQ(C[1], 22.0f);
    EXPECT_FLOAT_EQ(C[2], 43.0f);
    EXPECT_FLOAT_EQ(C[3], 50.0f);
}

TEST(CudaOpsTest, Matmul3x3) {
    const int M = 3, N = 3, K = 3;
    float A[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
    float B[] = {9, 8, 7, 6, 5, 4, 3, 2, 1};
    float C[9] = {0};

    cuda_matmul(A, B, C, M, N, K);

    EXPECT_FLOAT_EQ(C[0], 30.0f);
    EXPECT_FLOAT_EQ(C[1], 24.0f);
    EXPECT_FLOAT_EQ(C[2], 18.0f);
    EXPECT_FLOAT_EQ(C[3], 84.0f);
    EXPECT_FLOAT_EQ(C[4], 69.0f);
    EXPECT_FLOAT_EQ(C[5], 54.0f);
    EXPECT_FLOAT_EQ(C[6], 138.0f);
    EXPECT_FLOAT_EQ(C[7], 114.0f);
    EXPECT_FLOAT_EQ(C[8], 90.0f);
}

TEST(CudaOpsTest, Add) {
    float A[] = {1, 2, 3, 4};
    float B[] = {5, 6, 7, 8};
    float C[4] = {0};

    cuda_add(A, B, C, 4);

    EXPECT_FLOAT_EQ(C[0], 6.0f);
    EXPECT_FLOAT_EQ(C[1], 8.0f);
    EXPECT_FLOAT_EQ(C[2], 10.0f);
    EXPECT_FLOAT_EQ(C[3], 12.0f);
}

TEST(CudaOpsTest, ReLU) {
    float input[] = {-3, -1, 0, 1, 3};
    float output[5] = {0};

    cuda_relu(input, output, 5);

    EXPECT_FLOAT_EQ(output[0], 0.0f);
    EXPECT_FLOAT_EQ(output[1], 0.0f);
    EXPECT_FLOAT_EQ(output[2], 0.0f);
    EXPECT_FLOAT_EQ(output[3], 1.0f);
    EXPECT_FLOAT_EQ(output[4], 3.0f);
}

TEST(CudaOpsTest, Softmax) {
    float input[] = {1, 2, 3};
    float output[3] = {0};

    cuda_softmax(input, output, 1, 3);

    float sum = output[0] + output[1] + output[2];
    EXPECT_NEAR(sum, 1.0f, 1e-5f);
    EXPECT_GT(output[2], output[1]);
    EXPECT_GT(output[1], output[0]);
}

TEST(CudaOpsTest, SoftmaxMultipleRows) {
    float input[] = {1, 2, 3, 10, 20, 30};
    float output[6] = {0};

    cuda_softmax(input, output, 2, 3);

    float sum1 = output[0] + output[1] + output[2];
    float sum2 = output[3] + output[4] + output[5];
    EXPECT_NEAR(sum1, 1.0f, 1e-5f);
    EXPECT_NEAR(sum2, 1.0f, 1e-5f);
}

TEST(CudaOpsTest, LayerNorm) {
    float input[] = {1, 2, 3, 4, 5, 6};
    float output[6] = {0};

    cuda_layer_norm(input, output, 2, 3);

    float mean1 = (output[0] + output[1] + output[2]) / 3.0f;
    float mean2 = (output[3] + output[4] + output[5]) / 3.0f;
    EXPECT_NEAR(mean1, 0.0f, 1e-4f);
    EXPECT_NEAR(mean2, 0.0f, 1e-4f);
}

TEST(CudaOpsTest, Scale) {
    float input[] = {1, 2, 3};
    float output[3] = {0};

    cuda_scale(input, output, 2.5f, 3);

    EXPECT_FLOAT_EQ(output[0], 2.5f);
    EXPECT_FLOAT_EQ(output[1], 5.0f);
    EXPECT_FLOAT_EQ(output[2], 7.5f);
}

TEST(CudaOpsTest, BiasAdd) {
    float input[] = {1, 2, 3, 4};
    float bias[] = {10, 20};
    float output[4] = {0};

    cuda_bias_add(input, bias, output, 2, 2);

    EXPECT_FLOAT_EQ(output[0], 11.0f);
    EXPECT_FLOAT_EQ(output[1], 22.0f);
    EXPECT_FLOAT_EQ(output[2], 13.0f);
    EXPECT_FLOAT_EQ(output[3], 24.0f);
}
