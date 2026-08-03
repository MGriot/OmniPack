#ifndef OMNIPACK_CATALOG_H
#define OMNIPACK_CATALOG_H
/* Direct port of the catalog CRUD in src/omnipack/logic.py:
   get_catalog_data / save_to_catalog_data / delete_from_catalog_data.
   Catalog entries are opaque JSON objects; only the "name" field is
   inspected, for upsert/delete matching. */

#include "omnipack/json_value.h"

typedef struct {
    char path[512];
} Catalog;

void catalog_init(Catalog* c, const char* path);

/* Returns {"items": [...]} - caller owns the result (json_free it). Returns
   an empty catalog if the file doesn't exist or fails to parse, matching
   logic.py's behavior. */
JsonValue* catalog_get_all(const Catalog* c);

/* `entry` must have a "name" string field. Catalog clones it internally;
   caller still owns and must free `entry`. */
void catalog_save(const Catalog* c, const JsonValue* entry);

void catalog_remove(const Catalog* c, const char* name);

#endif /* OMNIPACK_CATALOG_H */
