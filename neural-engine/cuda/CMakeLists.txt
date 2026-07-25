if(NOT NEO_ENABLE_CUDA)
    return()
endif()

add_library(neo_cuda_ops STATIC
    src/ops.cu
)

target_include_directories(neo_cuda_ops PUBLIC include)
target_compile_features(neo_cuda_ops PUBLIC cxx_std_20)

if(NEO_ENABLE_TESTS)
    add_executable(neo_cuda_ops_tests tests/test_ops.cu)
    target_link_libraries(neo_cuda_ops_tests PRIVATE neo_cuda_ops)
    add_test(NAME neo_cuda_ops_tests COMMAND neo_cuda_ops_tests)
endif()
