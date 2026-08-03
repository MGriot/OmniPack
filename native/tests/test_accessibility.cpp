// Ports tests/test_accessibility.py and tests/test_fifo_lifo.py
#include <stdexcept>

#include "doctest.h"
#include "omnipack/engine.hpp"
#include "omnipack/models.hpp"

using namespace omnipack;

TEST_CASE("stop_id grouping - correct discharge order is fully accessible") {
    Container container("TRUCK_1", 100, 100, 100);
    Item stop1("STOP_1", 50, 50, 20); stop1.stop_id = 1;
    Item stop2("STOP_2", 50, 50, 20); stop2.stop_id = 2;
    Item stop3("STOP_3", 50, 50, 20); stop3.stop_id = 3;

    Level2Engine engine(container, 0.5);
    engine.pack({stop1, stop2, stop3});

    const Item* p3 = nullptr;
    for (const auto& it : container.items) if (it.id == "STOP_3") p3 = &it;
    REQUIRE(p3 != nullptr);
    CHECK(p3->position.z == doctest::Approx(0.0));

    CHECK(container.calculate_accessibility() == doctest::Approx(100.0));
}

TEST_CASE("stop_id saturation - stacked in correct discharge order") {
    Container container("TRUCK_SMALL", 60, 60, 100);
    Item stop1("S_STOP_1", 50, 50, 20); stop1.stop_id = 1;
    Item stop2("S_STOP_2", 50, 50, 20); stop2.stop_id = 2;
    Item stop3("S_STOP_3", 50, 50, 20); stop3.stop_id = 3;

    Level2Engine engine(container, 1.0);
    engine.pack({stop1, stop2, stop3});

    auto find = [&](const std::string& id) -> const Item& {
        for (const auto& it : container.items) if (it.id == id) return it;
        throw std::runtime_error("not found");
    };

    CHECK(find("S_STOP_3").position.z == doctest::Approx(0.0));
    CHECK(find("S_STOP_2").position.z == doctest::Approx(20.0));
    CHECK(find("S_STOP_1").position.z == doctest::Approx(40.0));

    CHECK(container.calculate_accessibility() == doctest::Approx(100.0));
}

TEST_CASE("wrong order blocking is detected") {
    Container container("TRUCK_WRONG", 60, 60, 100);

    Item wrong1("WRONG_1", 50, 50, 20); wrong1.stop_id = 1; wrong1.position = {0, 0, 0};
    Item wrong2("WRONG_2", 50, 50, 20); wrong2.stop_id = 2; wrong2.position = {0, 0, 20};

    container.items = {wrong1, wrong2};

    // WRONG_1 (needed first) sits behind WRONG_2 (needed later) -> WRONG_1 is blocked.
    // Accessible count = 1 (WRONG_2) of 2 total -> 50%.
    CHECK(container.calculate_accessibility() == doctest::Approx(50.0));
}

TEST_CASE("FIFO/LIFO/NONE placement order") {
    Container container("C1", 100, 100, 100);
    Item item_fifo("FIFO_1", 20, 20, 20); item_fifo.strategy = LoadingStrategy::FIFO;
    Item item_lifo("LIFO_1", 20, 20, 20); item_lifo.strategy = LoadingStrategy::LIFO;
    Item item_none("NONE_1", 20, 20, 20); item_none.strategy = LoadingStrategy::NONE;

    Level2Engine engine(container, 0.5);
    auto unpacked = engine.pack({item_fifo, item_lifo, item_none});

    REQUIRE(unpacked.empty());

    auto find = [&](const std::string& id) -> const Item& {
        for (const auto& it : container.items) if (it.id == id) return it;
        throw std::runtime_error("not found");
    };

    CHECK(find("FIFO_1").position.z == doctest::Approx(0.0));
    CHECK(find("LIFO_1").position.z > 0.0);
    CHECK(find("LIFO_1").position.z >= 50.0);
}
