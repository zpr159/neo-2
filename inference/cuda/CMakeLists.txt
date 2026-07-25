if(NOT NEO_ENABLE_CUDA)
    return()
endif()

add_library(neo_inference_engine STATIC
    src/engine.cu
)

target_include_directories(neo_inference_engine PUBLIC include)
target_compile_features(neo_inference_engine PUBLIC cxx_std_20)
target_link_libraries(neo_inference_engine PRIVATE neo_neural_engine neo_nn_layers)

if(NEO_ENABLE_TESTS)
    add_executable(neo_inference_engine_tests tests/test_engine.cu)
    target_link_libraries(neo_inference_engine_tests PRIVATE neo_inference_engine GTest::gtest_main)
    add_test(NAME neo_inference_engine_tests COMMAND neo_inference_engine_tests)
endif()
