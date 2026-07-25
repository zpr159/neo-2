#pragma once

#include <cstddef>
#include <cstdint>

namespace neo::neural::cuda {

void cuda_matmul(const float* A, const float* B, float* C,
                 int M, int N, int K);

void cuda_add(const float* A, const float* B, float* C,
              int size);

void cuda_relu(const float* input, float* output, int size);

void cuda_softmax(const float* input, float* output, int rows, int cols);

void cuda_layer_norm(const float* input, float* output,
                     int rows, int cols, float eps = 1e-5f);

void cuda_matmul_transpose_a(const float* A, const float* B, float* C,
                              int M, int N, int K);

void cuda_matmul_transpose_b(const float* A, const float* B, float* C,
                              int M, int N, int K);

void cuda_scale(const float* input, float* output, float scale, int size);

void cuda_bias_add(const float* input, const float* bias, float* output,
                   int rows, int cols);

} // namespace neo::neural::cuda
