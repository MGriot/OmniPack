/* Ports native/tests/test_collision.cpp (from tests/test_shapes_collision.py) */
#include "omnipack/engine.h"
#include "omnipack/extreme_points.h"
#include "omnipack/models.h"
#include "test.h"

TEST_CASE(test_sphere_sphere_collision_via_get_valid_ep) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);
    Item s1 = item_make("S1", 10, 10, 10);
    s1.shape_type = SHAPE_SPHERE;
    s1.position.x = 0; s1.position.y = 0; s1.position.z = 0;
    item_array_push(&c.items, s1);

    Item s2 = item_make("S2", 10, 10, 10);
    s2.shape_type = SHAPE_SPHERE;

    ExtremePoint overlapping = {5, 0, 0};
    ExtremePointList out1; eps_init(&out1);
    get_valid_ep(&c, &s2, &overlapping, 1, &out1);
    CHECK_TRUE(out1.len == 0);
    eps_free(&out1);

    ExtremePoint touching = {10, 0, 0};
    ExtremePointList out2; eps_init(&out2);
    get_valid_ep(&c, &s2, &touching, 1, &out2);
    CHECK_TRUE(out2.len == 1);
    eps_free(&out2);

    container_free(&c);
}

TEST_CASE(test_sphere_box_collision_via_get_valid_ep) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);
    Item b1 = item_make("B1", 10, 10, 10);
    b1.position.x = 0; b1.position.y = 0; b1.position.z = 0;
    item_array_push(&c.items, b1);

    Item s1 = item_make("S1", 10, 10, 10);
    s1.shape_type = SHAPE_SPHERE;

    ExtremePoint close = {8, 0, 0};
    ExtremePointList out1; eps_init(&out1);
    get_valid_ep(&c, &s1, &close, 1, &out1);
    CHECK_TRUE(out1.len == 0);
    eps_free(&out1);

    ExtremePoint far = {15, 0, 0};
    ExtremePointList out2; eps_init(&out2);
    get_valid_ep(&c, &s1, &far, 1, &out2);
    CHECK_TRUE(out2.len == 1);
    eps_free(&out2);

    container_free(&c);
}

TEST_CASE(test_level2_engine_avoids_sphere_box_collision) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);

    Level2Engine engine;
    level2_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL, 0.0);

    Item b1 = item_make("B1", 10, 10, 10);
    b1.position.x = 0; b1.position.y = 0; b1.position.z = 0;
    item_array_push(&c.items, b1);

    Item s1 = item_make("S1", 10, 10, 10);
    s1.shape_type = SHAPE_SPHERE;

    level2_engine_seed_extreme_point(&engine, 9, 0, 0);
    level2_engine_seed_extreme_point(&engine, 15, 0, 0);

    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, &s1, 1, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    const Item* placed = &c.items.data[c.items.len - 1];
    CHECK_TRUE(placed->position.x >= 10.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

void run_collision_tests(void) {
    RUN_TEST(test_sphere_sphere_collision_via_get_valid_ep);
    RUN_TEST(test_sphere_box_collision_via_get_valid_ep);
    RUN_TEST(test_level2_engine_avoids_sphere_box_collision);
}
