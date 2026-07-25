#include <neo/neural/cuda_ops.hpp>
#include <cmath>
#include <cstring>
#include <vector>

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

__global__ void tiled_matmul_kernel(const float* A, const float* B, float* C,
                                     int M, int N, int K, int tile_size) {
    __shared__ float As[32][32];
    __shared__ float Bs[32][32];

    int bx = blockIdx.x;
    int by = blockIdx.y;
    int tx = threadIdx.x;
    int ty = threadIdx.y;

    int row = by * tile_size + ty;
    int col = bx * tile_size + tx;

    float sum = 0.0f;
    for (int t = 0; t < (K + tile_size - 1) / tile_size; ++t) {
        if (row < M && t * tile_size + tx < K) {
            As[ty][tx] = A[row * K + t * tile_size + tx];
        } else {
            As[ty][tx] = 0.0f;
        }

        if (col < N && t * tile_size + ty < K) {
            Bs[ty][tx] = B[(t * tile_size + ty) * N + col];
        } else {
            Bs[ty][tx] = 0.0f;
        }

        __syncthreads();

        for (int k = 0; k < tile_size; ++k) {
            sum += As[ty][k] * Bs[k][tx];
        }
        __syncthreads();
    }

    if (row < M && col < N) {
        C[row * N + col] = sum;
    }
}

} // namespace neo::neural::cuda
