// Ports tests/test_engine.py, tests/test_ep.py, tests/test_packing_versus.py, tests/test_grasp.py
#include <set>

#include "doctest.h"
#include "omnipack/engine.hpp"
#include "omnipack/extreme_points.hpp"
#include "omnipack/models.hpp"

using namespace omnipack;

TEST_CASE("Level1Engine simple pack") {
    Container container("c1", 100, 100, 100);
    std::vector<Item> items = {Item("i1", 50, 50, 50), Item("i2", 50, 50, 50)};

    Level1Engine engine(container);
    auto unpacked = engine.pack(items);

    REQUIRE(unpacked.empty());
    REQUIRE(container.items.size() == 2);
    CHECK(container.items[0].position.x == doctest::Approx(0.0));
    CHECK(container.items[0].position.y == doctest::Approx(0.0));
    CHECK(container.items[0].position.z == doctest::Approx(0.0));
}

TEST_CASE("Level1Engine rotates to fit") {
    Container container("c1", 10, 50, 10);
    Item item("i1", 50, 10, 10); // must rotate to fit

    Level1Engine engine(container);
    auto unpacked = engine.pack({item});

    REQUIRE(unpacked.empty());
    REQUIRE(container.items.size() == 1);
    auto dim = container.items[0].get_dimension();
    CHECK(dim[0] <= container.width);
    CHECK(dim[1] <= container.height);
    CHECK(dim[2] <= container.depth);
}

TEST_CASE("Level1Engine reports overflow") {
    Container container("c1", 10, 10, 10);
    std::vector<Item> items = {Item("i1", 10, 10, 10), Item("i2", 10, 10, 10)};

    Level1Engine engine(container);
    auto unpacked = engine.pack(items);

    CHECK(unpacked.size() == 1);
    CHECK(container.items.size() == 1);
}

TEST_CASE("generate_extreme_points produces the basic forward + backward points") {
    Container container("c1", 100, 100, 100);
    Item item("i1", 10, 20, 30);
    item.position = {0, 0, 0};

    auto eps = generate_extreme_points(container, item);
    std::set<std::tuple<double, double, double>> tuples;
    for (const auto& p : eps) tuples.insert({p.x, p.y, p.z});

    CHECK(tuples.count({10.0, 0.0, 0.0}) == 1);
    CHECK(tuples.count({0.0, 20.0, 0.0}) == 1);
    CHECK(tuples.count({0.0, 0.0, 30.0}) == 1);
    CHECK(tuples.count({0.0, 0.0, 0.0}) == 1); // backward point
    CHECK(eps.size() >= 4);
}

TEST_CASE("get_valid_ep respects container boundary") {
    Container container("c1", 20, 20, 20);
    Item item("i1", 15, 15, 15);

    ExtremePointSet eps{ExtremePoint(0, 0, 0), ExtremePoint(10, 0, 0)};
    auto valid = get_valid_ep(container, item, eps);

    REQUIRE(valid.size() == 1);
    CHECK(valid[0] == ExtremePoint(0, 0, 0));
}

TEST_CASE("get_valid_ep filters out colliding points") {
    Container container("c1", 100, 100, 100);
    Item existing("e1", 10, 10, 10);
    existing.position = {0, 0, 0};
    container.items.push_back(existing);

    Item new_item("n1", 10, 10, 10);

    ExtremePointSet eps{ExtremePoint(0, 0, 0), ExtremePoint(10, 0, 0)};
    auto valid = get_valid_ep(container, new_item, eps);

    REQUIRE(valid.size() == 1);
    CHECK(valid[0] == ExtremePoint(10, 0, 0));
}

TEST_CASE("PackingVersus::LATERAL fills width before depth") {
    Container container("C1", 100, 100, 100);
    Level2Engine engine(container, 1.0, PackingVersus::LATERAL);

    std::vector<Item> items = {Item("B1", 10, 10, 10), Item("B2", 10, 10, 10), Item("B3", 10, 10, 10)};
    engine.pack(items);

    REQUIRE(container.items.size() == 3);
    CHECK(container.items[0].position.x == doctest::Approx(0.0));
    CHECK(container.items[1].position.x == doctest::Approx(10.0));
    CHECK(container.items[2].position.x == doctest::Approx(20.0));
}

TEST_CASE("PackingVersus::LONGITUDINAL fills depth before width") {
    Container container("C1", 100, 100, 100);
    Level2Engine engine(container, 1.0, PackingVersus::LONGITUDINAL);

    std::vector<Item> items = {Item("B1", 10, 10, 10), Item("B2", 10, 10, 10), Item("B3", 10, 10, 10)};
    engine.pack(items);

    REQUIRE(container.items.size() == 3);
    CHECK(container.items[0].position.z == doctest::Approx(0.0));
    CHECK(container.items[1].position.z == doctest::Approx(10.0));
    CHECK(container.items[2].position.z == doctest::Approx(20.0));
}

TEST_CASE("GRASP k=1 is deterministic, k=10 explores different layouts") {
    auto run_engine = [](int k) {
        Container container("C", 100, 100, 100);
        std::vector<Item> items;
        for (int i = 0; i < 5; ++i) items.push_back(Item("I_" + std::to_string(i), 30, 30, 30));
        Level2Engine engine(container, 0.5);
        engine.pack(items, k);

        std::vector<std::tuple<double, double, double>> positions;
        for (const auto& it : container.items) positions.push_back({it.position.x, it.position.y, it.position.z});
        return positions;
    };

    auto pos1_k1 = run_engine(1);
    auto pos2_k1 = run_engine(1);
    CHECK(pos1_k1 == pos2_k1);

    auto first_run_k10 = run_engine(10);
    bool different = false;
    for (int i = 0; i < 10; ++i) {
        if (run_engine(10) != first_run_k10) { different = true; break; }
    }
    CHECK(different);
}
