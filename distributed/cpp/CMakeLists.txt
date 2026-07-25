add_library(neo_distributed STATIC
    src/node.cpp
)

target_include_directories(neo_distributed PUBLIC include)
target_compile_features(neo_distributed PUBLIC cxx_std_20)
target_link_libraries(neo_distributed PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_distributed_tests tests/test_node.cpp)
    target_link_libraries(neo_distributed_tests PRIVATE neo_distributed GTest::gtest_main)
    add_test(NAME neo_distributed_tests COMMAND neo_distributed_tests)
endif()
