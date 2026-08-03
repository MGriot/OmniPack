#include <stdio.h>

int g_omnipack_tests_run = 0;
int g_omnipack_tests_failed = 0;
int g_omnipack_current_test_failed = 0;

void run_models_tests(void);
void run_collision_tests(void);
void run_accessibility_tests(void);
void run_engine_tests(void);
void run_multi_container_tests(void);
void run_catalog_tests(void);

int main(void) {
    run_models_tests();
    run_collision_tests();
    run_accessibility_tests();
    run_engine_tests();
    run_multi_container_tests();
    run_catalog_tests();

    printf("\n%d tests run, %d failed\n", g_omnipack_tests_run, g_omnipack_tests_failed);
    return g_omnipack_tests_failed == 0 ? 0 : 1;
}
