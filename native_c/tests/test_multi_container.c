/* Ports native/tests/test_multi_container.cpp */
#include <string.h>

#include "omnipack/models.h"
#include "omnipack/multi_container.h"
#include "test.h"

TEST_CASE(test_overflow_into_second_container) {
    MultiContainerEngine e;
    multi_container_engine_init(&e, "C", 10, 10, 10, 1000000.0, SHAPE_BOX, VERSUS_LONGITUDINAL);
    Item items[2] = {item_make("i1", 10, 10, 10), item_make("i2", 10, 10, 10)};

    ContainerArray out;
    container_array_init(&out);
    multi_container_engine_pack_all(&e, items, 2, "level2", 1.0, 1, 0.0, &out);

    CHECK_TRUE(out.len == 2);
    CHECK_TRUE(out.data[0].items.len == 1);
    CHECK_TRUE(out.data[1].items.len == 1);
    CHECK_TRUE(strcmp(out.data[0].id, "C_1") == 0);
    CHECK_TRUE(strcmp(out.data[1].id, "C_2") == 0);

    container_array_free(&out);
}

TEST_CASE(test_all_items_fit_single_container) {
    MultiContainerEngine e;
    multi_container_engine_init(&e, "C", 100, 100, 100, 1000000.0, SHAPE_BOX, VERSUS_LONGITUDINAL);
    Item items[2] = {item_make("i1", 10, 10, 10), item_make("i2", 10, 10, 10)};

    ContainerArray out;
    container_array_init(&out);
    multi_container_engine_pack_all(&e, items, 2, "level2", 1.0, 1, 0.0, &out);

    CHECK_TRUE(out.len == 1);
    CHECK_TRUE(out.data[0].items.len == 2);

    container_array_free(&out);
}

void run_multi_container_tests(void) {
    RUN_TEST(test_overflow_into_second_container);
    RUN_TEST(test_all_items_fit_single_container);
}
