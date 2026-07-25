add_library(neo_memory STATIC
    src/arena.cpp
)

target_include_directories(neo_memory PUBLIC include)
target_compile_features(neo_memory PUBLIC cxx_std_20)
target_link_libraries(neo_memory PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_memory_tests tests/test_arena.cpp)
    target_link_libraries(neo_memory_tests PRIVATE neo_memory GTest::gtest_main)
    add_test(NAME neo_memory_tests COMMAND neo_memory_tests)
endif()
