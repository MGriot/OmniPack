cmake_minimum_required(VERSION 3.22)
project(omnipack_jni LANGUAGES C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Reuse the same native_c/core sources as the Win32 smoke-test app -
# one engine, no duplication.
set(OMNIPACK_CORE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../native_c/core)

add_library(omnipack_core_c STATIC
    ${OMNIPACK_CORE_DIR}/src/models.c
    ${OMNIPACK_CORE_DIR}/src/prng.c
    ${OMNIPACK_CORE_DIR}/src/extreme_points.c
    ${OMNIPACK_CORE_DIR}/src/collision.c
    ${OMNIPACK_CORE_DIR}/src/engine.c
    ${OMNIPACK_CORE_DIR}/src/multi_container.c
    ${OMNIPACK_CORE_DIR}/src/json_value.c
    ${OMNIPACK_CORE_DIR}/src/catalog.c
)
target_include_directories(omnipack_core_c PUBLIC ${OMNIPACK_CORE_DIR}/include)
target_link_libraries(omnipack_core_c PUBLIC m)

add_library(omnipack_jni SHARED
    jni_bridge.c
)
target_link_libraries(omnipack_jni PRIVATE omnipack_core_c log)
