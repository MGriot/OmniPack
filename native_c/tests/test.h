#ifndef OMNIPACK_TEST_H
#define OMNIPACK_TEST_H
/* Minimal self-contained C test harness - no external dependency, to avoid
   the doctest/MinGW-toolchain-path friction hit while building native/. */

#include <math.h>
#include <stdio.h>
#include <stdlib.h>

extern int g_omnipack_tests_run;
extern int g_omnipack_tests_failed;
extern int g_omnipack_current_test_failed;

#define TEST_CASE(fn_name) static void fn_name(void)

#define RUN_TEST(fn_name)                                          \
    do {                                                           \
        g_omnipack_current_test_failed = 0;                        \
        g_omnipack_tests_run++;                                    \
        fn_name();                                                 \
        if (g_omnipack_current_test_failed) {                      \
            g_omnipack_tests_failed++;                             \
            printf("[FAIL] %s\n", #fn_name);                       \
        } else {                                                   \
            printf("[ OK ] %s\n", #fn_name);                       \
        }                                                          \
    } while (0)

#define CHECK_TRUE(cond)                                                          \
    do {                                                                          \
        if (!(cond)) {                                                           \
            printf("       %s:%d: CHECK_TRUE(%s) failed\n", __FILE__, __LINE__, #cond); \
            g_omnipack_current_test_failed = 1;                                   \
        }                                                                         \
    } while (0)

#define CHECK_DOUBLE_EQ(a, b)                                                                        \
    do {                                                                                             \
        double _a = (a), _b = (b);                                                                  \
        if (fabs(_a - _b) > 1e-6) {                                                                  \
            printf("       %s:%d: CHECK_DOUBLE_EQ(%s, %s) failed: %f != %f\n", __FILE__, __LINE__,   \
                   #a, #b, _a, _b);                                                                  \
            g_omnipack_current_test_failed = 1;                                                       \
        }                                                                                             \
    } while (0)

#define CHECK_STR_EQ(a, b)                                                                    \
    do {                                                                                      \
        if (strcmp((a), (b)) != 0) {                                                          \
            printf("       %s:%d: CHECK_STR_EQ(%s, %s) failed: \"%s\" != \"%s\"\n", __FILE__, \
                   __LINE__, #a, #b, (a), (b));                                               \
            g_omnipack_current_test_failed = 1;                                               \
        }                                                                                      \
    } while (0)

#endif /* OMNIPACK_TEST_H */
