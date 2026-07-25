#include <neo/nn/cuda_layers.hpp>
#include <cmath>
#include <cstring>
#include <vector>
#include <algorithm>

namespace neo::nn::cuda {

void cuda_dense_forward(const float* input, const float* weight, const float* bias,
                         float* output, int batch_size, int in_features, int out_features) {
    for (int b = 0; b < batch_size; ++b) {
        for (int o = 0; o < out_features; ++o) {
            float sum = bias ? bias[o] : 0.0f;
            for (int i = 0; i < in_features; ++i) {
                sum += input[b * in_features + i] * weight[o * in_features + i];
            }
            output[b * out_features + o] = sum;
        }
    }
}

void cuda_conv2d_forward(const float* input, const float* kernel, float* output,
                          int batch_size, int in_channels, int out_channels,
                          int height, int width, int kernel_size, int stride, int padding) {
    int out_h = (height + 2 * padding - kernel_size) / stride + 1;
    int out_w = (width + 2 * padding - kernel_size) / stride + 1;

    for (int b = 0; b < batch_size; ++b) {
        for (int oc = 0; oc < out_channels; ++oc) {
            for (int oh = 0; oh < out_h; ++oh) {
                for (int ow = 0; ow < out_w; ++ow) {
                    float sum = 0.0f;
                    for (int ic = 0; ic < in_channels; ++ic) {
                        for (int kh = 0; kh < kernel_size; ++kh) {
                            for (int kw = 0; kw < kernel_size; ++kw) {
                                int ih = oh * stride + kh - padding;
                                int iw = ow * stride + kw - padding;
                                if (ih >= 0 && ih < height && iw >= 0 && iw < width) {
                                    float in_val = input[b * in_channels * height * width + ic * height * width + ih * width + iw];
                                    float k_val = kernel[oc * in_channels * kernel_size * kernel_size + ic * kernel_size * kernel_size + kh * kernel_size + kw];
                                    sum += in_val * k_val;
                                }
                            }
                        }
                    }
                    output[b * out_channels * out_h * out_w + oc * out_h * out_w + oh * out_w + ow] = sum;
                }
            }
        }
    }
}

void cuda_batch_norm(const float* input, const float* gamma, const float* beta,
                      const float* running_mean, const float* running_var,
                      float* output, int batch_size, int features, float eps) {
    for (int b = 0; b < batch_size; ++b) {
        for (int f = 0; f < features; ++f) {
            float normalized = (input[b * features + f] - running_mean[f]) /
                               std::sqrt(running_var[f] + eps);
            output[b * features + f] = gamma[f] * normalized + beta[f];
        }
    }
}

void cuda_max_pool2d(const float* input, float* output,
                      int batch_size, int channels, int height, int width,
                      int pool_size, int stride) {
    int out_h = (height - pool_size) / stride + 1;
    int out_w = (width - pool_size) / stride + 1;

    for (int b = 0; b < batch_size; ++b) {
        for (int c = 0; c < channels; ++c) {
            for (int oh = 0; oh < out_h; ++oh) {
                for (int ow = 0; ow < out_w; ++ow) {
                    float max_val = -1e30f;
                    for (int ph = 0; ph < pool_size; ++ph) {
                        for (int pw = 0; pw < pool_size; ++pw) {
                            int ih = oh * stride + ph;
                            int iw = ow * stride + pw;
                            float val = input[b * channels * height * width + c * height * width + ih * width + iw];
                            max_val = std::max(max_val, val);
                        }
                    }
                    output[b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow] = max_val;
                }
            }
        }
    }
}

void cuda_avg_pool2d(const float* input, float* output,
                      int batch_size, int channels, int height, int width,
                      int pool_size, int stride) {
    int out_h = (height - pool_size) / stride + 1;
    int out_w = (width - pool_size) / stride + 1;
    float inv_area = 1.0f / (pool_size * pool_size);

    for (int b = 0; b < batch_size; ++b) {
        for (int c = 0; c < channels; ++c) {
            for (int oh = 0; oh < out_h; ++oh) {
                for (int ow = 0; ow < out_w; ++ow) {
                    float sum = 0.0f;
                    for (int ph = 0; ph < pool_size; ++ph) {
                        for (int pw = 0; pw < pool_size; ++pw) {
                            int ih = oh * stride + ph;
                            int iw = ow * stride + pw;
                            sum += input[b * channels * height * width + c * height * width + ih * width + iw];
                        }
                    }
                    output[b * channels * out_h * out_w + c * out_h * out_w + oh * out_w + ow] = sum * inv_area;
                }
            }
        }
    }
}

void cuda_dropout(const float* input, float* output, float* mask,
                   int size, float probability, unsigned int seed) {
    float scale = 1.0f / (1.0f - probability);
    unsigned int state = seed;

    for (int i = 0; i < size; ++i) {
        state = state * 1103515245 + 12345;
        float rand_val = static_cast<float>((state >> 16) & 0x7FFF) / 32768.0f;

        if (rand_val < probability) {
            mask[i] = 0.0f;
            output[i] = 0.0f;
        } else {
            mask[i] = scale;
            output[i] = input[i] * scale;
        }
    }
}

void cuda_softmax_cross_entropy(const float* logits, const float* targets,
                                 float* output, int batch_size, int num_classes) {
    for (int b = 0; b < batch_size; ++b) {
        float max_val = -1e30f;
        for (int c = 0; c < num_classes; ++c) {
            max_val = std::max(max_val, logits[b * num_classes + c]);
        }

        float sum = 0.0f;
        for (int c = 0; c < num_classes; ++c) {
            sum += std::exp(logits[b * num_classes + c] - max_val);
        }

        float loss = 0.0f;
        for (int c = 0; c < num_classes; ++c) {
            float prob = std::exp(logits[b * num_classes + c] - max_val) / sum;
            if (targets[b * num_classes + c] > 0.0f) {
                loss -= targets[b * num_classes + c] * std::log(prob + 1e-7f);
            }
        }
        output[b] = loss;
    }
}

} // namespace neo::nn::cuda
