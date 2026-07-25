add_library(neo_security STATIC
    src/crypto.cpp
)

target_include_directories(neo_security PUBLIC include)
target_compile_features(neo_security PUBLIC cxx_std_20)
target_link_libraries(neo_security PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_security_tests tests/test_crypto.cpp)
    target_link_libraries(neo_security_tests PRIVATE neo_security GTest::gtest_main)
    add_test(NAME neo_security_tests COMMAND neo_security_tests)
endif()
