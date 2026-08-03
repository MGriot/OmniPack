// Exercises MultiContainerEngine's overflow-across-containers loop
// (src/omnipack/core/multi_container.py has no dedicated Python test file;
// this covers the same behavior exercised indirectly via tests/test_api.py).
#include "doctest.h"
#include "omnipack/models.hpp"
#include "omnipack/multi_container.hpp"

using namespace omnipack;

TEST_CASE("items that don't fit in one container overflow into a second") {
    Container base("C", 10, 10, 10); // volume 1000, holds exactly one 10x10x10 item
    std::vector<Item> items = {Item("i1", 10, 10, 10), Item("i2", 10, 10, 10)};

    MultiContainerEngine engine(base);
    auto containers = engine.pack_all(items);

    REQUIRE(containers.size() == 2);
    CHECK(containers[0].items.size() == 1);
    CHECK(containers[1].items.size() == 1);
    CHECK(containers[0].id == "C_1");
    CHECK(containers[1].id == "C_2");
}

TEST_CASE("all items fit in a single container") {
    Container base("C", 100, 100, 100);
    std::vector<Item> items = {Item("i1", 10, 10, 10), Item("i2", 10, 10, 10)};

    MultiContainerEngine engine(base);
    auto containers = engine.pack_all(items);

    REQUIRE(containers.size() == 1);
    CHECK(containers[0].items.size() == 2);
}
