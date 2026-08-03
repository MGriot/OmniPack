add_library(omnipack_core_c STATIC
    src/models.c
    src/prng.c
    src/extreme_points.c
    src/collision.c
    src/engine.c
    src/multi_container.c
    src/json_value.c
    src/catalog.c
)
target_include_directories(omnipack_core_c PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_compile_features(omnipack_core_c PUBLIC c_std_11)

if(NOT MSVC)
    target_link_libraries(omnipack_core_c PUBLIC m)
endif()
