cmake_minimum_required(VERSION 3.20)
project(omnipack_native_c LANGUAGES C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

option(OMNIPACK_C_BUILD_TESTS "Build the omnipack_core_c unit tests" ON)
option(OMNIPACK_C_BUILD_WIN32_APP "Build the Win32 smoke-test app (Windows only)" ${WIN32})

add_subdirectory(core)

if(OMNIPACK_C_BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(OMNIPACK_C_BUILD_WIN32_APP AND WIN32)
    add_subdirectory(win32_app)
endif()
