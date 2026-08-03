add_executable(omnipack_tests_c
    test_main.c
    test_models.c
    test_collision.c
    test_accessibility.c
    test_engine.c
    test_multi_container.c
    test_catalog.c
)
target_link_libraries(omnipack_tests_c PRIVATE omnipack_core_c)

include(CTest)
add_test(NAME omnipack_tests_c COMMAND omnipack_tests_c)
