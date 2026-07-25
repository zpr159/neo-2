add_library(neo_neural_engine STATIC
    src/tensor.cpp
    src/device.cpp
)

target_include_directories(neo_neural_engine PUBLIC include)
target_compile_features(neo_neural_engine PUBLIC cxx_std_20)
target_link_libraries(neo_neural_engine PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_neural_engine_tests tests/test_tensor.cpp)
    target_link_libraries(neo_neural_engine_tests PRIVATE neo_neural_engine GTest::gtest_main)
    add_test(NAME neo_neural_engine_tests COMMAND neo_neural_engine_tests)
endif()
