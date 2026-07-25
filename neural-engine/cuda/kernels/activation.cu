#include <cmath>

namespace neo::neural::cuda {

__global__ void relu_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = fmaxf(0.0f, input[idx]);
    }
}

__global__ void leaky_relu_kernel(const float* input, float* output,
                                   int size, float alpha) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = input[idx] >= 0.0f ? input[idx] : alpha * input[idx];
    }
}

__global__ void sigmoid_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = 1.0f / (1.0f + expf(-input[idx]));
    }
}

__global__ void tanh_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        output[idx] = tanhf(input[idx]);
    }
}

__global__ void gelu_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        float x = input[idx];
        float c = 0.7978845608f;
        float k = 0.044715f;
        output[idx] = 0.5f * x * (1.0f + tanhf(c * x * (1.0f + k * x * x)));
    }
}

__global__ void silu_kernel(const float* input, float* output, int size) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < size) {
        float x = input[idx];
        output[idx] = x / (1.0f + expf(-x));
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

__global__ void log_softmax_kernel(const float* input, float* output,
                                    int rows, int cols) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row < rows) {
        float max_val = -1e30f;
        for (int c = 0; c < cols; ++c) {
            max_val = fmaxf(max_val, input[row * cols + c]);
        }

        float sum = 0.0f;
        for (int c = 0; c < cols; ++c) {
            sum += expf(input[row * cols + c] - max_val);
        }

        float log_sum = logf(sum);
        for (int c = 0; c < cols; ++c) {
            output[row * cols + c] = input[row * cols + c] - max_val - log_sum;
        }
    }
}

} // namespace neo::neural::cuda
