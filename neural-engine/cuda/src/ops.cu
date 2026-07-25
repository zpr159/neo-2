#include <neo/neural/cuda_ops.hpp>
#include <cmath>
#include <cstring>
#include <algorithm>
#include <vector>
#include <numeric>

namespace neo::neural::cuda {

__global__ void matmul_kernel(const float* A, const float* B, float* C,
                               int M, int N, int K) {
    int row = blockIdx.y * blockDim.y + threadIdx.y;
    int col = blockIdx.x * blockDim.x + threadIdx.x;

    if (row < M && col < N) {
        float sum = 0.0f;
        for (int k = 0; k < K; ++k) {
            sum += A[row * K + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
}

__global__ void add_kernel(const float* A, const float* B, float* C, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        C[idx] = A[idx] + B[idx];
    }
}

__global__ void relu_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = fmaxf(0.0f, input[idx]);
    }
}

__global__ void softmax_kernel(const float* input, float* output,
                                int rows, int cols) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row < rows) {
        float max_val = -1e30f;
        for (int c = 0; c < cols; ++c) {
            max_val = fmaxf(max_val, input[row * cols + c]);
        }

        float sum = 0.0f;
        for (int c = 0; c < cols; ++c) {
            output[row * cols + c] = expf(input[row * cols + c] - max_val);
            sum += output[row * cols + c];
        }

        for (int c = 0; c < cols; ++c) {
            output[row * cols + c] /= sum;
        }
    }
}

__global__ void layer_norm_kernel(const float* input, float* output,
                                   int rows, int cols, float eps) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row < rows) {
        float mean = 0.0f;
        for (int c = 0; c < cols; ++c) {
            mean += input[row * cols + c];
        }
        mean /= cols;

        float variance = 0.0f;
        for (int c = 0; c < cols; ++c) {
            float diff = input[row * cols + c] - mean;
            variance += diff * diff;
        }
        variance /= cols;

        float inv_std = rsqrtf(variance + eps);
        for (int c = 0; c < cols; ++c) {
            output[row * cols + c] = (input[row * cols + c] - mean) * inv_std;
        }
    }
}

void cuda_matmul(const float* A, const float* B, float* C,
                 int M, int N, int K) {
    std::vector<float> h_A(A, A + M * K);
    std::vector<float> h_B(B, B + K * N);
    std::vector<float> h_C(M * N, 0.0f);

    for (int i = 0; i < M; ++i) {
        for (int j = 0; j < N; ++j) {
            float sum = 0.0f;
            for (int k = 0; k < K; ++k) {
                sum += h_A[i * K + k] * h_B[k * N + j];
            }
            h_C[i * N + j] = sum;
        }
    }

    std::memcpy(C, h_C.data(), M * N * sizeof(float));
}

void cuda_add(const float* A, const float* B, float* C, int size) {
    for (int i = 0; i < size; ++i) {
        C[i] = A[i] + B[i];
    }
}

void cuda_relu(const float* input, float* output, int size) {
    for (int i = 0; i < size; ++i) {
        output[i] = std::max(0.0f, input[i]);
    }
}

void cuda_softmax(const float* input, float* output, int rows, int cols) {
    for (int r = 0; r < rows; ++r) {
        float max_val = -1e30f;
        for (int c = 0; c < cols; ++c) {
            max_val = std::max(max_val, input[r * cols + c]);
        }

        float sum = 0.0f;
        for (int c = 0; c < cols; ++c) {
            output[r * cols + c] = std::exp(input[r * cols + c] - max_val);
            sum += output[r * cols + c];
        }

        for (int c = 0; c < cols; ++c) {
            output[r * cols + c] /= sum;
        }
    }
}

void cuda_layer_norm(const float* input, float* output, int rows, int cols, float eps) {
    for (int r = 0; r < rows; ++r) {
        float mean = 0.0f;
        for (int c = 0; c < cols; ++c) {
            mean += input[r * cols + c];
        }
        mean /= cols;

        float variance = 0.0f;
        for (int c = 0; c < cols; ++c) {
            float diff = input[r * cols + c] - mean;
            variance += diff * diff;
        }
        variance /= cols;

        float inv_std = 1.0f / std::sqrt(variance + eps);
        for (int c = 0; c < cols; ++c) {
            output[r * cols + c] = (input[r * cols + c] - mean) * inv_std;
        }
    }
}

void cuda_matmul_transpose_a(const float* A, const float* B, float* C,
                              int M, int N, int K) {
    std::vector<float> h_C(M * N, 0.0f);
    for (int i = 0; i < M; ++i) {
        for (int j = 0; j < N; ++j) {
            float sum = 0.0f;
            for (int k = 0; k < K; ++k) {
                sum += A[k * M + i] * B[k * N + j];
            }
            h_C[i * N + j] = sum;
        }
    }
    std::memcpy(C, h_C.data(), M * N * sizeof(float));
}

void cuda_matmul_transpose_b(const float* A, const float* B, float* C,
                              int M, int N, int K) {
    std::vector<float> h_C(M * N, 0.0f);
    for (int i = 0; i < M; ++i) {
        for (int j = 0; j < N; ++j) {
            float sum = 0.0f;
            for (int k = 0; k < K; ++k) {
                sum += A[i * K + k] * B[j * K + k];
            }
            h_C[i * N + j] = sum;
        }
    }
    std::memcpy(C, h_C.data(), M * N * sizeof(float));
}

void cuda_scale(const float* input, float* output, float scale, int size) {
    for (int i = 0; i < size; ++i) {
        output[i] = input[i] * scale;
    }
}

void cuda_bias_add(const float* input, const float* bias, float* output,
                   int rows, int cols) {
    for (int r = 0; r < rows; ++r) {
        for (int c = 0; c < cols; ++c) {
            output[r * cols + c] = input[r * cols + c] + bias[c];
        }
    }
}

} // namespace neo::neural::cuda
