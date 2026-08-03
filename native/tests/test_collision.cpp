// Ports tests/test_shapes_collision.py
#include "doctest.h"
#include "omnipack/engine.hpp"
#include "omnipack/extreme_points.hpp"
#include "omnipack/models.hpp"

using namespace omnipack;

TEST_CASE("sphere-sphere collision via get_valid_ep") {
    // Sphere 1 at (0,0,0) diameter 10. Center (5,5,5).
    Container container("C1", 100, 100, 100);
    Item s1("S1", 10, 10, 10);
    s1.shape_type = ShapeType::SPHERE;
    s1.position = {0, 0, 0};
    container.items.push_back(s1);

    Item s2("S2", 10, 10, 10);
    s2.shape_type = ShapeType::SPHERE;

    // Sphere 2 at (5,0,0) -> Center (10,5,5). Dist 5 < radii sum 10 -> COLLIDE.
    ExtremePointSet overlapping{ExtremePoint(5, 0, 0)};
    auto valid = get_valid_ep(container, s2, overlapping);
    CHECK(valid.empty());

    // Sphere 2 at (10,0,0) -> touching, should not collide.
    ExtremePointSet touching{ExtremePoint(10, 0, 0)};
    valid = get_valid_ep(container, s2, touching);
    CHECK(valid.size() == 1);
}

TEST_CASE("sphere-box collision via get_valid_ep") {
    Container container("C1", 100, 100, 100);
    Item b1("B1", 10, 10, 10);
    b1.position = {0, 0, 0};
    container.items.push_back(b1);

    Item s1("S1", 10, 10, 10);
    s1.shape_type = ShapeType::SPHERE;

    // Sphere at (8,0,0) -> center (13,5,5). Closest box point (10,5,5). Dist 3 < 5 -> COLLIDE.
    ExtremePointSet close{ExtremePoint(8, 0, 0)};
    auto valid = get_valid_ep(container, s1, close);
    CHECK(valid.empty());

    // Sphere at (15,0,0) -> center (20,5,5). Dist 10 > 5 -> no collision.
    ExtremePointSet far{ExtremePoint(15, 0, 0)};
    valid = get_valid_ep(container, s1, far);
    CHECK(valid.size() == 1);
}

TEST_CASE("Level2Engine avoids sphere-box collision when placing") {
    Container container("C1", 100, 100, 100);
    Level2Engine engine(container);

    Item b1("B1", 10, 10, 10);
    b1.position = {0, 0, 0};
    container.items.push_back(b1);

    Item s1("S1", 10, 10, 10);
    s1.shape_type = ShapeType::SPHERE;

    engine.seed_extreme_point(9, 0, 0);
    engine.seed_extreme_point(15, 0, 0);

    auto unpacked = engine.pack({s1});

    REQUIRE(unpacked.empty());
    const auto& placed = container.items.back();
    CHECK(placed.position.x >= 10.0);
}
