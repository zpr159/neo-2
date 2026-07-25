add_library(neo_knowledge STATIC
    src/graph.cpp
)

target_include_directories(neo_knowledge PUBLIC include)
target_compile_features(neo_knowledge PUBLIC cxx_std_20)
target_link_libraries(neo_knowledge PRIVATE neo_core)

if(NEO_ENABLE_TESTS)
    add_executable(neo_knowledge_tests tests/test_graph.cpp)
    target_link_libraries(neo_knowledge_tests PRIVATE neo_knowledge GTest::gtest_main)
    add_test(NAME neo_knowledge_tests COMMAND neo_knowledge_tests)
endif()
