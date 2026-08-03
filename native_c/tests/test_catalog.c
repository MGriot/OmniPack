/* Ports native/tests/test_catalog.cpp */
#include <stdio.h>
#include <string.h>

#include "omnipack/catalog.h"
#include "test.h"

TEST_CASE(test_catalog_lifecycle) {
    const char* path = "omnipack_test_catalog_c.json";
    remove(path);

    Catalog cat;
    catalog_init(&cat, path);

    JsonValue* empty = catalog_get_all(&cat);
    CHECK_TRUE(json_array_size(json_object_get(empty, "items")) == 0);
    json_free(empty);

    JsonValue* entry = json_new_object();
    json_object_set(entry, "name", json_new_string("Test Configuration"));
    JsonValue* container = json_new_object();
    json_object_set(container, "id", json_new_string("c1"));
    json_object_set(container, "width", json_new_number(100));
    json_object_set(entry, "container", container);

    catalog_save(&cat, entry);

    JsonValue* after_save = catalog_get_all(&cat);
    JsonValue* items = json_object_get(after_save, "items");
    CHECK_TRUE(json_array_size(items) == 1);
    CHECK_TRUE(strcmp(json_object_get(json_array_get(items, 0), "name")->string, "Test Configuration") == 0);
    json_free(after_save);

    /* Update same entry (upsert by name) */
    json_object_set(json_object_get(entry, "container"), "width", json_new_number(200));
    catalog_save(&cat, entry);

    JsonValue* after_update = catalog_get_all(&cat);
    JsonValue* items2 = json_object_get(after_update, "items");
    CHECK_TRUE(json_array_size(items2) == 1);
    JsonValue* updated_width = json_object_get(json_object_get(json_array_get(items2, 0), "container"), "width");
    CHECK_DOUBLE_EQ(updated_width->number, 200.0);
    json_free(after_update);

    catalog_remove(&cat, "Test Configuration");
    JsonValue* after_delete = catalog_get_all(&cat);
    CHECK_TRUE(json_array_size(json_object_get(after_delete, "items")) == 0);
    json_free(after_delete);

    json_free(entry);
    remove(path);
}

void run_catalog_tests(void) {
    RUN_TEST(test_catalog_lifecycle);
}
