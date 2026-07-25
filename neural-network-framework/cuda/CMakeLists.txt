if(NOT NEO_ENABLE_CUDA)
    return()
endif()

add_library(neo_nn_layers STATIC
    src/layers.cu
)

target_include_directories(neo_nn_layers PUBLIC include)
target_compile_features(neo_nn_layers PUBLIC cxx_std_20)
target_link_libraries(neo_nn_layers PRIVATE neo_neural_engine)

if(NEO_ENABLE_TESTS)
    add_executable(neo_nn_layers_tests tests/test_layers.cu)
    target_link_libraries(neo_nn_layers_tests PRIVATE neo_nn_layers GTest::gtest_main)
    add_test(NAME neo_nn_layers_tests COMMAND neo_nn_layers_tests)
endif()
