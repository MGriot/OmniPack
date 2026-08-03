add_executable(omnipack_win32_app WIN32 main.c)
target_link_libraries(omnipack_win32_app PRIVATE omnipack_core_c user32 gdi32)
