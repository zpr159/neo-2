if(NOT NEO_ENABLE_CUDA)
    return()
endif()

add_library(neo_cuda_embeddings STATIC
    src/embeddings.cu
)

target_include_directories(neo_cuda_embeddings PUBLIC include)
target_compile_features(neo_cuda_embeddings PUBLIC cxx_std_20)
target_link_libraries(neo_cuda_embeddings PRIVATE neo_neural_engine)

if(NEO_ENABLE_TESTS)
    add_executable(neo_cuda_embeddings_tests tests/test_embeddings.cu)
    target_link_libraries(neo_cuda_embeddings_tests PRIVATE neo_cuda_embeddings GTest::gtest_main)
    add_test(NAME neo_cuda_embeddings_tests COMMAND neo_cuda_embeddings_tests)
endif()
