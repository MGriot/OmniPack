/* Ports native/tests/test_accessibility.cpp
   (from tests/test_accessibility.py and tests/test_fifo_lifo.py) */
#include <string.h>

#include "omnipack/engine.h"
#include "omnipack/models.h"
#include "test.h"

static const Item* find_item(const Container* c, const char* id) {
    for (size_t i = 0; i < c->items.len; ++i) {
        if (strcmp(c->items.data[i].id, id) == 0) return &c->items.data[i];
    }
    return NULL;
}

TEST_CASE(test_stop_id_grouping_fully_accessible) {
    Container c;
    container_init(&c, "TRUCK_1", 100, 100, 100);
    Item stop1 = item_make("STOP_1", 50, 50, 20); stop1.stop_id = 1;
    Item stop2 = item_make("STOP_2", 50, 50, 20); stop2.stop_id = 2;
    Item stop3 = item_make("STOP_3", 50, 50, 20); stop3.stop_id = 3;
    Item items[3] = {stop1, stop2, stop3};

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 3, 1, &unpacked);

    const Item* p3 = find_item(&c, "STOP_3");
    CHECK_TRUE(p3 != NULL);
    CHECK_DOUBLE_EQ(p3->position.z, 0.0);
    CHECK_DOUBLE_EQ(container_calculate_accessibility(&c), 100.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_stop_id_saturation_correct_order) {
    Container c;
    container_init(&c, "TRUCK_SMALL", 60, 60, 100);
    Item stop1 = item_make("S_STOP_1", 50, 50, 20); stop1.stop_id = 1;
    Item stop2 = item_make("S_STOP_2", 50, 50, 20); stop2.stop_id = 2;
    Item stop3 = item_make("S_STOP_3", 50, 50, 20); stop3.stop_id = 3;
    Item items[3] = {stop1, stop2, stop3};

    Level2Engine engine;
    level2_engine_init(&engine, &c, 1.0, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 3, 1, &unpacked);

    CHECK_DOUBLE_EQ(find_item(&c, "S_STOP_3")->position.z, 0.0);
    CHECK_DOUBLE_EQ(find_item(&c, "S_STOP_2")->position.z, 20.0);
    CHECK_DOUBLE_EQ(find_item(&c, "S_STOP_1")->position.z, 40.0);
    CHECK_DOUBLE_EQ(container_calculate_accessibility(&c), 100.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

TEST_CASE(test_wrong_order_blocking_detected) {
    Container c;
    container_init(&c, "TRUCK_WRONG", 60, 60, 100);

    Item wrong1 = item_make("WRONG_1", 50, 50, 20); wrong1.stop_id = 1;
    wrong1.position.x = 0; wrong1.position.y = 0; wrong1.position.z = 0;
    Item wrong2 = item_make("WRONG_2", 50, 50, 20); wrong2.stop_id = 2;
    wrong2.position.x = 0; wrong2.position.y = 0; wrong2.position.z = 20;

    item_array_push(&c.items, wrong1);
    item_array_push(&c.items, wrong2);

    /* WRONG_1 (needed first) sits behind WRONG_2 (needed later) -> blocked.
       Accessible count = 1 (WRONG_2) of 2 total -> 50%. */
    CHECK_DOUBLE_EQ(container_calculate_accessibility(&c), 50.0);

    container_free(&c);
}

TEST_CASE(test_fifo_lifo_none_placement_order) {
    Container c;
    container_init(&c, "C1", 100, 100, 100);
    Item item_fifo = item_make("FIFO_1", 20, 20, 20); item_fifo.strategy = STRAT_FIFO;
    Item item_lifo = item_make("LIFO_1", 20, 20, 20); item_lifo.strategy = STRAT_LIFO;
    Item item_none = item_make("NONE_1", 20, 20, 20); item_none.strategy = STRAT_NONE;
    Item items[3] = {item_fifo, item_lifo, item_none};

    Level2Engine engine;
    level2_engine_init(&engine, &c, 0.5, VERSUS_LONGITUDINAL, 0.0);
    ItemArray unpacked;
    item_array_init(&unpacked);
    level2_engine_pack(&engine, items, 3, 1, &unpacked);

    CHECK_TRUE(unpacked.len == 0);
    CHECK_DOUBLE_EQ(find_item(&c, "FIFO_1")->position.z, 0.0);
    CHECK_TRUE(find_item(&c, "LIFO_1")->position.z > 0.0);
    CHECK_TRUE(find_item(&c, "LIFO_1")->position.z >= 50.0);

    item_array_free(&unpacked);
    level2_engine_free(&engine);
    container_free(&c);
}

void run_accessibility_tests(void) {
    RUN_TEST(test_stop_id_grouping_fully_accessible);
    RUN_TEST(test_stop_id_saturation_correct_order);
    RUN_TEST(test_wrong_order_blocking_detected);
    RUN_TEST(test_fifo_lifo_none_placement_order);
}
