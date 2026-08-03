/* Ports native/tests/test_engine.cpp
   (from tests/test_engine.py, tests/test_ep.py, tests/test_packing_versus.py,
   tests/test_grasp.py) */
#include <math.h>
#include <string.h>

#include "omnipack/engine.h"
#include "omnipack/extreme_points.h"
#include "omnipack/models.h"
#include "test.h"

static int ep_list_has_point(const ExtremePointList* l, double x, double y, double z) {
    for (size_t i = 0; i < l->len; ++i) {
        if (fabs(l->data[i].x - x) < 0.001 && fabs(l->data[i].y - y) < 0.001 && fabs(l->data[i].z - z) < 0.001) return 1;
    }
    return 0;
}

TEST_CASE(test_level1_simple_pack) {
    Container c;
    container_init(&c, "c1", 100, 100, 100);
    Item items[2] = {item_make("i1", 50, 50, 50), item_make("i2", 50, 50, 50)};

    Level1Engine engine;
    level1_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level1_engine_pack(&engine, items, 2, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    CHECK_TRUE(c.items.len == 2);
    CHECK_DOUBLE_EQ(c.items.data[0].position.x, 0.0);
    CHECK_DOUBLE_EQ(c.items.data[0].position.y, 0.0);
    CHECK_DOUBLE_EQ(c.items.data[0].position.z, 0.0);

    item_array_free(&unpacked);
    level1_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_level1_rotates_to_fit) {
    Container c;
    container_init(&c, "c1", 10, 50, 10);
    Item item = item_make("i1", 50, 10, 10);

    Level1Engine engine;
    level1_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level1_engine_pack(&engine, &item, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    CHECK_TRUE(c.items.len == 1);
    double dim[3];
    item_get_dimension(&c.items.data[0], dim);
    CHECK_TRUE(dim[0] <= c.width);
    CHECK_TRUE(dim[1] <= c.height);
    CHECK_TRUE(dim[2] <= c.depth);

    item_array_free(&unpacked);
    level1_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_level1_reports_overflow) {
    Container c;
    container_init(&c, "c1", 10, 10, 10);
    Item items[2] = {item_make("i1", 10, 10, 10), item_make("i2", 10, 10, 10)};

    Level1Engine engine;
    level1_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level1_engine_pack(&engine, items, 2, &unpacked);

    CHECK_TRUE(unpacked.len == 1);
    CHECK_TRUE(c.items.len == 1);

    item_array_free(&unpacked);
    level1_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_generate_extreme_points_basic_and_backward) {
    Container c;
    container_init(&c, "c1", 100, 100, 100);
    Item item = item_make("i1", 10, 20, 30);
    item.position.x = 0; item.position.y = 0; item.position.z = 0;

    ExtremePointList eps;
    eps_init(&eps);
    generate_extreme_points(&c, &item, &eps);

    CHECK_TRUE(ep_list_has_point(&eps, 10.0, 0.0, 0.0));
    CHECK_TRUE(ep_list_has_point(&eps, 0.0, 20.0, 0.0));
    CHECK_TRUE(ep_list_has_point(&eps, 0.0, 0.0, 30.0));
    CHECK_TRUE(ep_list_has_point(&eps, 0.0, 0.0, 0.0)); /* backward point */
    CHECK_TRUE(eps.len >= 4);

    eps_free(&eps);
    container_free(&c);
}

TEST_CASE(test_get_valid_ep_respects_boundary) {
    Container c;
    container_init(&c, "c1", 20, 20, 20);
    Item item = item_make("i1", 15, 15, 15);

    ExtremePoint eps[2] = {{0, 0, 0}, {10, 0, 0}};
    ExtremePointList out;
    eps_init(&out);
    get_valid_ep(&c, &item, eps, 2, &out);

    CHECK_TRUE(out.len == 1);
    CHECK_TRUE(extreme_point_eq(out.data[0], eps[0]));

    eps_free(&out);
    container_free(&c);
}

TEST_CASE(test_get_valid_ep_filters_collisions) {
    Container c;
    container_init(&c, "c1", 100, 100, 100);
    Item existing = item_make("e1", 10, 10, 10);
    existing.position.x = 0; existing.position.y = 0; existing.position.z = 0;
    item_array_push(&c.items, existing);

    Item new_item = item_make("n1", 10, 10, 10);

    ExtremePoint eps[2] = {{0, 0, 0}, {10, 0, 0}};
    ExtremePointList out;
    eps_init(&out);
    get_valid_ep(&c, &new_item, eps, 2, &out);

    CHECK_TRUE(out.len == 1);
    CHECK_TRUE(extreme_point_eq(out.data[0], eps[1]));

    eps_free(&out);
    container_free(&c);
}

TEST_CASE(test_lateral_versus_fills_width_before_depth) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);
    Level2Engine engine;
    level2_engine_init(&engine, &c, 1.0, VERSUS_LATERAL, 0.0);

    Item items[3] = {item_make("B1", 10, 10, 10), item_make("B2", 10, 10, 10), item_make("B3", 10, 10, 10)};
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 3, 1, &unpacked);

    CHECK_TRUE(c.items.len == 3);
    CHECK_DOUBLE_EQ(c.items.data[0].position.x, 0.0);
    CHECK_DOUBLE_EQ(c.items.data[1].position.x, 10.0);
    CHECK_DOUBLE_EQ(c.items.data[2].position.x, 20.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_longitudinal_versus_fills_depth_before_width) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);
    Level2Engine engine;
    level2_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL, 0.0);

    Item items[3] = {item_make("B1", 10, 10, 10), item_make("B2", 10, 10, 10), item_make("B3", 10, 10, 10)};
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 3, 1, &unpacked);

    CHECK_TRUE(c.items.len == 3);
    CHECK_DOUBLE_EQ(c.items.data[0].position.z, 0.0);
    CHECK_DOUBLE_EQ(c.items.data[1].position.z, 10.0);
    CHECK_DOUBLE_EQ(c.items.data[2].position.z, 20.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

static void run_grasp(int k, double out_positions[5][3]) {
    Container c;
    container_init(&c, "C", 100, 100, 100);
    Item items[5];
    for (int i = 0; i < 5; ++i) {
        char id[16];
        snprintf(id, sizeof(id), "I_%d", i);
        items[i] = item_make(id, 30, 30, 30);
    }

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 5, k, &unpacked);

    for (size_t i = 0; i < c.items.len && i < 5; ++i) {
        out_positions[i][0] = c.items.data[i].position.x;
        out_positions[i][1] = c.items.data[i].position.y;
        out_positions[i][2] = c.items.data[i].position.z;
    }

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

static int positions_equal(double a[5][3], double b[5][3]) {
    for (int i = 0; i < 5; ++i)
        for (int j = 0; j < 3; ++j)
            if (fabs(a[i][j] - b[i][j]) > 0.001) return 0;
    return 1;
}

TEST_CASE(test_grasp_k1_deterministic_k10_diverse) {
    double pos1_k1[5][3], pos2_k1[5][3];
    run_grasp(1, pos1_k1);
    run_grasp(1, pos2_k1);
    CHECK_TRUE(positions_equal(pos1_k1, pos2_k1));

    double first_run_k10[5][3];
    run_grasp(10, first_run_k10);
    int different = 0;
    for (int i = 0; i < 10; ++i) {
        double next_run[5][3];
        run_grasp(10, next_run);
        if (!positions_equal(first_run_k10, next_run)) { different = 1; break; }
    }
    CHECK_TRUE(different);
}

void run_engine_tests(void) {
    RUN_TEST(test_level1_simple_pack);
    RUN_TEST(test_level1_rotates_to_fit);
    RUN_TEST(test_level1_reports_overflow);
    RUN_TEST(test_generate_extreme_points_basic_and_backward);
    RUN_TEST(test_get_valid_ep_respects_boundary);
    RUN_TEST(test_get_valid_ep_filters_collisions);
    RUN_TEST(test_lateral_versus_fills_width_before_depth);
    RUN_TEST(test_longitudinal_versus_fills_depth_before_width);
    RUN_TEST(test_grasp_k1_deterministic_k10_diverse);
}
