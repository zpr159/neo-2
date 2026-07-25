#include <neo/embeddings/cuda_embeddings.hpp>
#include <cmath>
#include <cstring>
#include <algorithm>
#include <vector>
#include <numeric>

namespace neo::embeddings::cuda {

void cuda_embedding_lookup(const float* embedding_table, const int* indices,
                             float* output, int num_indices, int embedding_dim) {
    for (int i = 0; i < num_indices; ++i) {
        int idx = indices[i];
        std::memcpy(output + i * embedding_dim,
                     embedding_table + idx * embedding_dim,
                     embedding_dim * sizeof(float));
    }
}

void cuda_cosine_similarity(const float* A, const float* B, float* output,
                              int num_vectors, int dim) {
    for (int i = 0; i < num_vectors; ++i) {
        float dot = 0.0f;
        float norm_a = 0.0f;
        float norm_b = 0.0f;

        for (int d = 0; d < dim; ++d) {
            dot += A[i * dim + d] * B[i * dim + d];
            norm_a += A[i * dim + d] * A[i * dim + d];
            norm_b += B[i * dim + d] * B[i * dim + d];
        }

        float denom = std::sqrt(norm_a) * std::sqrt(norm_b);
        output[i] = denom > 1e-8f ? dot / denom : 0.0f;
    }
}

void cuda_batch_cosine_similarity(const float* query, const float* keys,
                                    float* output, int num_queries, int num_keys, int dim) {
    for (int q = 0; q < num_queries; ++q) {
        for (int k = 0; k < num_keys; ++k) {
            float dot = 0.0f;
            float norm_q = 0.0f;
            float norm_k = 0.0f;

            for (int d = 0; d < dim; ++d) {
                float qv = query[q * dim + d];
                float kv = keys[k * dim + d];
                dot += qv * kv;
                norm_q += qv * qv;
                norm_k += kv * kv;
            }

            float denom = std::sqrt(norm_q) * std::sqrt(norm_k);
            output[q * num_keys + k] = denom > 1e-8f ? dot / denom : 0.0f;
        }
    }
}

void cuda_l2_normalize(float* vectors, int num_vectors, int dim) {
    for (int i = 0; i < num_vectors; ++i) {
        float norm = 0.0f;
        for (int d = 0; d < dim; ++d) {
            norm += vectors[i * dim + d] * vectors[i * dim + d];
        }
        norm = std::sqrt(norm);

        if (norm > 1e-8f) {
            for (int d = 0; d < dim; ++d) {
                vectors[i * dim + d] /= norm;
            }
        }
    }
}

void cuda_add_positional_encoding(const float* input, float* output,
                                    int seq_len, int dim, float scale) {
    for (int pos = 0; pos < seq_len; ++pos) {
        for (int d = 0; d < dim; d += 2) {
            float angle = static_cast<float>(pos) / std::pow(10000.0f, static_cast<float>(d) / dim);
            float sin_val = std::sin(angle) * scale;
            float cos_val = std::cos(angle) * scale;

            output[pos * dim + d] = input[pos * dim + d] + sin_val;
            if (d + 1 < dim) {
                output[pos * dim + d + 1] = input[pos * dim + d + 1] + cos_val;
            }
        }
    }
}

void cuda_tokenize_embeddings(const float* token_embeddings, const int* attention_mask,
                                float* output, int batch_size, int seq_len, int dim) {
    for (int b = 0; b < batch_size; ++b) {
        for (int s = 0; s < seq_len; ++s) {
            int mask_val = attention_mask[b * seq_len + s];
            for (int d = 0; d < dim; ++d) {
                output[b * seq_len * dim + s * dim + d] =
                    token_embeddings[b * seq_len * dim + s * dim + d] * static_cast<float>(mask_val);
            }
        }
    }
}

void cuda_topk(const float* input, float* values, int* indices,
                int batch_size, int num_elements, int k) {
    for (int b = 0; b < batch_size; ++b) {
        std::vector<std::pair<float, int>> indexed(k);
        for (int i = 0; i < k; ++i) {
            indexed[i] = {input[b * num_elements + i], i};
        }

        std::partial_sort(indexed.begin(), indexed.begin() + k, indexed.end(),
            [](const auto& a, const auto& b) { return a.first > b.first; });

        for (int i = k; i < num_elements; ++i) {
            if (input[b * num_elements + i] > indexed[k - 1].first) {
                indexed[k - 1] = {input[b * num_elements + i], i};
                std::partial_sort(indexed.begin(), indexed.begin() + k, indexed.end(),
                    [](const auto& a, const auto& b) { return a.first > b.first; });
            }
        }

        for (int i = 0; i < k; ++i) {
            values[b * k + i] = indexed[i].first;
            indices[b * k + i] = indexed[i].second;
        }
    }
}

} // namespace neo::embeddings::cuda
