add_library(neo_reasoning STATIC
    src/logic.cpp
)

target_include_directories(neo_reasoning PUBLIC include)
target_compile_features(neo_reasoning PUBLIC cxx_std_20)
target_link_libraries(neo_reasoning PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_reasoning_tests tests/test_logic.cpp)
    target_link_libraries(neo_reasoning_tests PRIVATE neo_reasoning GTest::gtest_main)
    add_test(NAME neo_reasoning_tests COMMAND neo_reasoning_tests)
endif()
