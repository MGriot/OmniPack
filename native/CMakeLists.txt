cmake_minimum_required(VERSION 3.20)
project(omnipack_native LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

option(OMNIPACK_BUILD_TESTS "Build the omnipack_core unit tests" ON)
option(OMNIPACK_BUILD_APP "Build the Qt Quick smoke-test app (requires Qt6 via CMAKE_PREFIX_PATH)" OFF)

add_subdirectory(core)

if(OMNIPACK_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(OMNIPACK_BUILD_APP)
    find_package(Qt6 REQUIRED COMPONENTS Quick)
    qt_standard_project_setup()
    add_subdirectory(app)
endif()
