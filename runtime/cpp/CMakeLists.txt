add_library(neo_runtime STATIC
    src/process.cpp
    src/scheduler.cpp
)

target_include_directories(neo_runtime PUBLIC include)
target_compile_features(neo_runtime PUBLIC cxx_std_20)
target_link_libraries(neo_runtime PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_runtime_tests tests/test_process.cpp)
    target_link_libraries(neo_runtime_tests PRIVATE neo_runtime GTest::gtest_main)
    add_test(NAME neo_runtime_tests COMMAND neo_runtime_tests)
endif()
