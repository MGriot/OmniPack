include(FetchContent)
FetchContent_Declare(
    doctest
    URL https://github.com/doctest/doctest/releases/download/v2.4.11/doctest.h
    DOWNLOAD_NO_EXTRACT TRUE
)
FetchContent_MakeAvailable(doctest)

add_library(doctest_main OBJECT doctest_main.cpp)
target_include_directories(doctest_main PUBLIC ${doctest_SOURCE_DIR})

add_executable(omnipack_tests
    test_models.cpp
    test_collision.cpp
    test_accessibility.cpp
    test_engine.cpp
    test_multi_container.cpp
    test_catalog.cpp
)
target_link_libraries(omnipack_tests PRIVATE omnipack_core doctest_main)
target_include_directories(omnipack_tests PRIVATE ${doctest_SOURCE_DIR})

include(CTest)
add_test(NAME omnipack_tests COMMAND omnipack_tests)
