// Ports tests/test_catalog_api.py's catalog lifecycle (save/get/update/delete)
#include <filesystem>

#include "doctest.h"
#include "omnipack/catalog.hpp"

using namespace omnipack;

TEST_CASE("catalog save/get/update/delete lifecycle") {
    auto path = std::filesystem::temp_directory_path() / "omnipack_test_catalog.json";
    std::filesystem::remove(path);

    Catalog catalog(path);

    // 1. Empty catalog
    auto empty = catalog.get_all();
    REQUIRE(empty.at("items").array().empty());

    // 2. Save entry
    JsonValue entry = JsonValue::make_object();
    entry["name"] = JsonValue::make_string("Test Configuration");
    JsonValue container = JsonValue::make_object();
    container["id"] = JsonValue::make_string("c1");
    container["width"] = JsonValue::make_number(100);
    entry["container"] = container;

    catalog.save(entry);

    auto after_save = catalog.get_all();
    REQUIRE(after_save.at("items").array().size() == 1);
    CHECK(after_save.at("items").array()[0].at("name").as_string() == "Test Configuration");

    // 3. Update same entry (upsert by name)
    entry["container"].object()["width"] = JsonValue::make_number(200);
    catalog.save(entry);

    auto after_update = catalog.get_all();
    REQUIRE(after_update.at("items").array().size() == 1);
    CHECK(after_update.at("items").array()[0].at("container").at("width").as_number() == doctest::Approx(200));

    // 4. Delete
    catalog.remove("Test Configuration");
    auto after_delete = catalog.get_all();
    CHECK(after_delete.at("items").array().empty());

    std::filesystem::remove(path);
}
