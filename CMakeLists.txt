# Neo — Root CMake Configuration
#
# This CMakeLists.txt builds all C/C++ components in the Neo monorepo.
# It integrates with CUDA for GPU-accelerated components and produces
# static libraries that are linked into the Rust crates via FFI.

cmake_minimum_required(VERSION 3.28)

project(neo-cpp
    VERSION 0.1.0
    DESCRIPTION "Neo AGI Operating System — C++ Components"
    LANGUAGES C CXX
)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_C_STANDARD 17)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE Release CACHE STRING "Build type" FORCE)
    set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS Debug Release RelWithDebInfo MinSizeRel)
endif()

option(NEO_ENABLE_CUDA "Enable CUDA support for GPU acceleration" ON)
option(NEO_ENABLE_TESTS "Enable building tests" ON)
option(NEO_ENABLE_BENCHMARKS "Enable building benchmarks" ON)
option(NEO_ENABLE_SANITIZERS "Enable address and undefined behavior sanitizers" OFF)

if(NEO_ENABLE_SANITIZERS)
    add_compile_options(-fsanitize=address,undefined -fno-omit-frame-pointer)
    add_link_options(-fsanitize=address,undefined)
endif()

if(NEO_ENABLE_CUDA)
    include(CheckLanguage)
    check_language(CUDA)
    if(CMAKE_CUDA_COMPILER)
        enable_language(CUDA)
        set(CMAKE_CUDA_STANDARD 20)
        set(CMAKE_CUDA_STANDARD_REQUIRED ON)
        set(CMAKE_CUDA_ARCHITECTURES "70;75;80;86;89;90")
        message(STATUS "CUDA support enabled: ${CMAKE_CUDA_COMPILER}")
    else()
        message(WARNING "CUDA compiler not found — GPU support disabled")
        set(NEO_ENABLE_CUDA OFF)
    endif()
endif()

set(NEO_CPP_ROOT ${CMAKE_CURRENT_SOURCE_DIR})
set(NEO_CPP_INCLUDE ${NEO_CPP_ROOT}/core/cpp/include)
set(NEO_CPP_SRC ${NEO_CPP_ROOT}/core/cpp/src)

add_subdirectory(core/cpp)
add_subdirectory(runtime/cpp)
add_subdirectory(neural-engine/cpp)
add_subdirectory(memory/cpp)
add_subdirectory(knowledge-graph/cpp)
add_subdirectory(reasoning/cpp)
add_subdirectory(distributed/cpp)
add_subdirectory(security/cpp)
add_subdirectory(robotics/cpp)

if(NEO_ENABLE_CUDA)
    add_subdirectory(neural-engine/cuda)
    add_subdirectory(neural-network-framework/cuda)
    add_subdirectory(inference/cuda)
    add_subdirectory(memory/embeddings)
endif()

if(NEO_ENABLE_TESTS)
    enable_testing()
    find_package(GTest QUIET)
    if(NOT GTest_FOUND)
        include(FetchContent)
        FetchContent_Declare(
            googletest
            GIT_REPOSITORY https://github.com/google/googletest.git
            GIT_TAG v1.14.0
        )
        FetchContent_MakeAvailable(googletest)
    endif()
endif()
