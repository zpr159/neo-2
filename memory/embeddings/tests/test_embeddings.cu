#include <gtest/gtest.h>
#include <neo/embeddings/cuda_embeddings.hpp>
#include <cmath>
#include <vector>

using namespace neo::embeddings::cuda;

TEST(EmbeddingLookupTest, Basic) {
    int num_embeddings = 4;
    int dim = 3;
    float table[] = {
        1, 0, 0,
        0, 1, 0,
        0, 0, 1,
        1, 1, 1
    };
    int indices[] = {0, 2, 3};
    float output[9] = {0};

    cuda_embedding_lookup(table, indices, output, 3, dim);

    EXPECT_FLOAT_EQ(output[0], 1.0f);
    EXPECT_FLOAT_EQ(output[1], 0.0f);
    EXPECT_FLOAT_EQ(output[2], 0.0f);
    EXPECT_FLOAT_EQ(output[3], 0.0f);
    EXPECT_FLOAT_EQ(output[4], 0.0f);
    EXPECT_FLOAT_EQ(output[5], 1.0f);
    EXPECT_FLOAT_EQ(output[6], 1.0f);
    EXPECT_FLOAT_EQ(output[7], 1.0f);
    EXPECT_FLOAT_EQ(output[8], 1.0f);
}

TEST(CosineSimilarityTest, Identical) {
    float A[] = {1, 0, 0};
    float B[] = {1, 0, 0};
    float output[1] = {0};

    cuda_cosine_similarity(A, B, output, 1, 3);
    EXPECT_NEAR(output[0], 1.0f, 1e-5f);
}

TEST(CosineSimilarityTest, Orthogonal) {
    float A[] = {1, 0, 0};
    float B[] = {0, 1, 0};
    float output[1] = {0};

    cuda_cosine_similarity(A, B, output, 1, 3);
    EXPECT_NEAR(output[0], 0.0f, 1e-5f);
}

TEST(CosineSimilarityTest, Multiple) {
    float A[] = {1, 0, 0, 1, 1, 0};
    float B[] = {0, 1, 0, 1, 0, 0};
    float output[2] = {0};

    cuda_cosine_similarity(A, B, output, 2, 3);
    EXPECT_NEAR(output[0], 0.0f, 1e-5f);
    EXPECT_NEAR(output[1], 1.0f / std::sqrt(2.0f), 1e-5f);
}

TEST(BatchCosineSimilarityTest, Basic) {
    float query[] = {1, 0};
    float keys[] = {1, 0, 0, 1, -1, 0};
    float output[3] = {0};

    cuda_batch_cosine_similarity(query, keys, output, 1, 3, 2);
    EXPECT_NEAR(output[0], 1.0f, 1e-5f);
    EXPECT_NEAR(output[1], 0.0f, 1e-5f);
    EXPECT_NEAR(output[2], -1.0f, 1e-5f);
}

TEST(L2NormalizeTest, Normalize) {
    float vectors[] = {3, 4};
    cuda_l2_normalize(vectors, 1, 2);

    float norm = std::sqrt(vectors[0] * vectors[0] + vectors[1] * vectors[1]);
    EXPECT_NEAR(norm, 1.0f, 1e-5f);
}

TEST(AddPositionalEncodingTest, Basic) {
    int seq_len = 2, dim = 4;
    float input[] = {0, 0, 0, 0, 0, 0, 0, 0};
    float output[8] = {0};

    cuda_add_positional_encoding(input, output, seq_len, dim);

    for (int i = 0; i < 8; ++i) {
        EXPECT_NE(output[i], 0.0f);
    }
}

TEST(TopKTest, Basic) {
    float input[] = {5, 1, 3, 2, 4};
    float values[3] = {0};
    int indices[3] = {0};

    cuda_topk(input, values, indices, 1, 5, 3);

    EXPECT_FLOAT_EQ(values[0], 5.0f);
    EXPECT_FLOAT_EQ(values[1], 4.0f);
    EXPECT_FLOAT_EQ(values[2], 3.0f);
}
