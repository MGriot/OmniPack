// Ports tests/test_models.py and tests/test_rotation_constraints.py
#include "doctest.h"
#include "omnipack/engine.hpp"
#include "omnipack/models.hpp"

using namespace omnipack;

TEST_CASE("item volume") {
    Item item("box1", 10, 20, 30);
    CHECK(item.volume() == doctest::Approx(6000));
}

TEST_CASE("item rotation changes bounding dimensions") {
    Item item("box1", 10, 20, 30);
    item.rotation = Rotation::H_W_D;
    auto dim = item.get_dimension();
    CHECK(dim[0] == doctest::Approx(20));
    CHECK(dim[1] == doctest::Approx(10));
    CHECK(dim[2] == doctest::Approx(30));
}

TEST_CASE("container volume") {
    Container container("bin1", 100, 100, 100);
    CHECK(container.volume() == doctest::Approx(1'000'000));
}

TEST_CASE("container remaining volume") {
    Container container("bin1", 100, 100, 100);
    Item item("box1", 10, 20, 30);
    container.items.push_back(item);
    CHECK(container.remaining_volume() == doctest::Approx(1'000'000 - 6000));
}

TEST_CASE("shape type propagation") {
    Item item("sphere1", 10, 10, 10);
    item.shape_type = ShapeType::SPHERE;
    CHECK(item.shape_type == ShapeType::SPHERE);

    Container container("cont1", 100, 100, 100, 1'000'000.0, ShapeType::CYLINDER);
    CHECK(container.shape_type == ShapeType::CYLINDER);
}

TEST_CASE("restricted rotations - free item rotates to fit a short container") {
    // Container is SHORT (height 20). Item is TALL (10,50,10) if not rotated.
    Container container("SHORT_BIN", 100, 20, 100);
    Item item_free("FREE_1", 10, 50, 10);

    Level2Engine engine(container, 0.5);
    auto unpacked = engine.pack({item_free});

    REQUIRE(unpacked.empty());
    CHECK(container.items[0].rotation != Rotation::W_H_D);
}

TEST_CASE("restricted rotations - only W_H_D allowed fails in a short container") {
    Container container("SHORT_BIN_2", 100, 20, 100);
    Item item("RESTRICTED_1", 10, 50, 10);
    item.allowed_rotations = {Rotation::W_H_D};

    Level2Engine engine(container, 0.5);
    auto unpacked = engine.pack({item});

    CHECK(unpacked.size() == 1);
    CHECK(container.items.empty());
}

TEST_CASE("restricted rotations - specific valid rotation succeeds") {
    Container container("SHORT_BIN_3", 100, 20, 100);
    Item item("SPECIFIC_1", 10, 50, 10);
    item.allowed_rotations = {Rotation::H_W_D};

    Level2Engine engine(container, 0.5);
    auto unpacked = engine.pack({item});

    REQUIRE(unpacked.empty());
    CHECK(container.items[0].rotation == Rotation::H_W_D);
}
