# omnipack_core is header-only (all functions are inline / class templates),
# so this is an INTERFACE target: nothing to compile here, just include paths.
add_library(omnipack_core INTERFACE)
target_include_directories(omnipack_core INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/include)
target_compile_features(omnipack_core INTERFACE cxx_std_20)
