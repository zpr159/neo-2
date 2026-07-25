add_library(neo_robotics STATIC
    src/kinematics.cpp
    src/control.cpp
)

target_include_directories(neo_robotics PUBLIC include)
target_compile_features(neo_robotics PUBLIC cxx_std_20)
target_link_libraries(neo_robotics PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_robotics_tests tests/test_control.cpp)
    target_link_libraries(neo_robotics_tests PRIVATE neo_robotics GTest::gtest_main)
    add_test(NAME neo_robotics_tests COMMAND neo_robotics_tests)
endif()
