/* Ports native/tests/test_models.cpp (itself ported from
   tests/test_models.py and tests/test_rotation_constraints.py) */
#include <string.h>

#include "omnipack/engine.h"
#include "omnipack/models.h"
#include "test.h"

TEST_CASE(test_item_volume) {
    Item item = item_make("box1", 10, 20, 30);
    CHECK_DOUBLE_EQ(item_volume(&item), 6000.0);
}

TEST_CASE(test_item_rotation_changes_dimensions) {
    Item item = item_make("box1", 10, 20, 30);
    item.rotation = ROT_H_W_D;
    double dim[3];
    item_get_dimension(&item, dim);
    CHECK_DOUBLE_EQ(dim[0], 20.0);
    CHECK_DOUBLE_EQ(dim[1], 10.0);
    CHECK_DOUBLE_EQ(dim[2], 30.0);
}

TEST_CASE(test_container_volume) {
    Container c;
    container_init(&c, "bin1", 100, 100, 100);
    CHECK_DOUBLE_EQ(container_volume(&c), 1000000.0);
    container_free(&c);
}

TEST_CASE(test_container_remaining_volume) {
    Container c;
    container_init(&c, "bin1", 100, 100, 100);
    Item item = item_make("box1", 10, 20, 30);
    item_array_push(&c.items, item);
    CHECK_DOUBLE_EQ(container_remaining_volume(&c), 1000000.0 - 6000.0);
    container_free(&c);
}

TEST_CASE(test_shape_type_propagation) {
    Item item = item_make("sphere1", 10, 10, 10);
    item.shape_type = SHAPE_SPHERE;
    CHECK_TRUE(item.shape_type == SHAPE_SPHERE);

    Container c;
    container_init_full(&c, "cont1", 100, 100, 100, 1000000.0, SHAPE_CYLINDER);
    CHECK_TRUE(c.shape_type == SHAPE_CYLINDER);
    container_free(&c);
}

TEST_CASE(test_restricted_rotations_free_item_rotates) {
    Container c;
    container_init(&c, "SHORT_BIN", 100, 20, 100);
    Item item_free = item_make("FREE_1", 10, 50, 10);

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, &item_free, 1, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    CHECK_TRUE(c.items.data[0].rotation != ROT_W_H_D);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_restricted_rotations_only_w_h_d_fails) {
    Container c;
    container_init(&c, "SHORT_BIN_2", 100, 20, 100);
    Item item = item_make("RESTRICTED_1", 10, 50, 10);
    item.allowed_rotations[0] = ROT_W_H_D;
    item.allowed_rotations_count = 1;

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, &item, 1, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 1);
    CHECK_TRUE(c.items.len == 0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_restricted_rotations_specific_rotation_succeeds) {
    Container c;
    container_init(&c, "SHORT_BIN_3", 100, 20, 100);
    Item item = item_make("SPECIFIC_1", 10, 50, 10);
    item.allowed_rotations[0] = ROT_H_W_D;
    item.allowed_rotations_count = 1;

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, &item, 1, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    CHECK_TRUE(c.items.data[0].rotation == ROT_H_W_D);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

void run_models_tests(void) {
    RUN_TEST(test_item_volume);
    RUN_TEST(test_item_rotation_changes_dimensions);
    RUN_TEST(test_container_volume);
    RUN_TEST(test_container_remaining_volume);
    RUN_TEST(test_shape_type_propagation);
    RUN_TEST(test_restricted_rotations_free_item_rotates);
    RUN_TEST(test_restricted_rotations_only_w_h_d_fails);
    RUN_TEST(test_restricted_rotations_specific_rotation_succeeds);
}
