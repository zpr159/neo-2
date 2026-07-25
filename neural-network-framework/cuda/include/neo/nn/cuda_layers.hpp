#pragma once

#include <cstddef>
#include <cstdint>

namespace neo::nn::cuda {

void cuda_dense_forward(const float* input, const float* weight, const float* bias,
                         float* output, int batch_size, int in_features, int out_features);

void cuda_conv2d_forward(const float* input, const float* kernel, float* output,
                          int batch_size, int in_channels, int out_channels,
                          int height, int width, int kernel_size, int stride, int padding);

void cuda_batch_norm(const float* input, const float* gamma, const float* beta,
                      const float* running_mean, const float* running_var,
                      float* output, int batch_size, int features,
                      float eps = 1e-5f);

void cuda_max_pool2d(const float* input, float* output,
                      int batch_size, int channels, int height, int width,
                      int pool_size, int stride);

void cuda_avg_pool2d(const float* input, float* output,
                      int batch_size, int channels, int height, int width,
                      int pool_size, int stride);

void cuda_dropout(const float* input, float* output, float* mask,
                   int size, float probability, unsigned int seed);

void cuda_softmax_cross_entropy(const float* logits, const float* targets,
                                 float* output, int batch_size, int num_classes);

} // namespace neo::nn::cuda
