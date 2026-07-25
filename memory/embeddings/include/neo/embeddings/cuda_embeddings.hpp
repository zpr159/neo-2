#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace neo::embeddings::cuda {

void cuda_embedding_lookup(const float* embedding_table, const int* indices,
                             float* output, int num_indices, int embedding_dim);

void cuda_cosine_similarity(const float* A, const float* B, float* output,
                              int num_vectors, int dim);

void cuda_batch_cosine_similarity(const float* query, const float* keys,
                                    float* output, int num_queries, int num_keys, int dim);

void cuda_l2_normalize(float* vectors, int num_vectors, int dim);

void cuda_add_positional_encoding(const float* input, float* output,
                                    int seq_len, int dim, float scale = 1.0f);

void cuda_tokenize_embeddings(const float* token_embeddings, const int* attention_mask,
                                float* output, int batch_size, int seq_len, int dim);

void cuda_topk(const float* input, float* values, int* indices,
                int batch_size, int num_elements, int k);

} // namespace neo::embeddings::cuda
