#include "omnipack/catalog.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void catalog_init(Catalog* c, const char* path) {
    strncpy(c->path, path, sizeof(c->path) - 1);
    c->path[sizeof(c->path) - 1] = '\0';
}

JsonValue* catalog_get_all(const Catalog* c) {
    FILE* f = fopen(c->path, "rb");
    if (!f) {
        JsonValue* result = json_new_object();
        json_object_set(result, "items", json_new_array());
        return result;
    }

    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* buf = (char*)malloc((size_t)size + 1);
    size_t read_n = fread(buf, 1, (size_t)size, f);
    buf[read_n] = '\0';
    fclose(f);

    JsonValue* parsed = json_parse(buf);
    free(buf);

    if (!parsed || parsed->type != JSON_OBJECT || !json_object_has(parsed, "items")) {
        json_free(parsed);
        JsonValue* result = json_new_object();
        json_object_set(result, "items", json_new_array());
        return result;
    }
    return parsed;
}

static void write_catalog(const Catalog* c, const JsonValue* catalog) {
    char* text = json_dump(catalog, 4);
    FILE* f = fopen(c->path, "wb");
    if (f) {
        fwrite(text, 1, strlen(text), f);
        fclose(f);
    }
    free(text);
}

void catalog_save(const Catalog* c, const JsonValue* entry) {
    JsonValue* catalog = catalog_get_all(c);
    JsonValue* items = json_object_get(catalog, "items");
    JsonValue* name_val = json_object_get(entry, "name");
    const char* name = (name_val && name_val->type == JSON_STRING) ? name_val->string : "";

    int replaced = 0;
    for (size_t i = 0; i < json_array_size(items); ++i) {
        JsonValue* existing = json_array_get(items, i);
        JsonValue* existing_name = json_object_get(existing, "name");
        if (existing_name && existing_name->type == JSON_STRING && strcmp(existing_name->string, name) == 0) {
            JsonValue* cloned = json_clone(entry);
            json_free(items->array_items[i]);
            items->array_items[i] = cloned;
            replaced = 1;
            break;
        }
    }
    if (!replaced) json_array_push(items, json_clone(entry));

    write_catalog(c, catalog);
    json_free(catalog);
}

void catalog_remove(const Catalog* c, const char* name) {
    JsonValue* catalog = catalog_get_all(c);
    JsonValue* items = json_object_get(catalog, "items");

    for (size_t i = 0; i < json_array_size(items);) {
        JsonValue* existing = json_array_get(items, i);
        JsonValue* existing_name = json_object_get(existing, "name");
        if (existing_name && existing_name->type == JSON_STRING && strcmp(existing_name->string, name) == 0) {
            json_array_remove(items, i);
        } else {
            ++i;
        }
    }

    write_catalog(c, catalog);
    json_free(catalog);
}
