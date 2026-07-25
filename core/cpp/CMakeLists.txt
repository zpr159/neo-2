add_library(neo_core STATIC
    src/error.cpp
    src/types.cpp
    src/config.cpp
)

target_include_directories(neo_core PUBLIC include)
target_compile_features(neo_core PUBLIC cxx_std_20)
target_link_libraries(neo_core PRIVATE fmt::fmt nlohmann_json::nlohmann_json)

if(NEO_ENABLE_TESTS)
    add_executable(neo_core_tests tests/test_error.cpp tests/test_types.cpp)
    target_link_libraries(neo_core_tests PRIVATE neo_core GTest::gtest_main)
    add_test(NAME neo_core_tests COMMAND neo_core_tests)
endif()
