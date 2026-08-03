#ifndef OMNIPACK_JSON_VALUE_H
#define OMNIPACK_JSON_VALUE_H
/* Minimal JSON value type: just enough to parse/dump the catalog file format
   (an object with an "items" array of arbitrary, opaque item dicts keyed by
   "name"). Not a general-purpose JSON library. Manual memory management
   (malloc/free, explicit json_free) since plain C has no destructors. */

#include <stddef.h>

typedef enum { JSON_NULL, JSON_BOOL, JSON_NUMBER, JSON_STRING, JSON_ARRAY, JSON_OBJECT } JsonType;

typedef struct JsonValue JsonValue;

typedef struct {
    char* key;   /* owned */
    JsonValue* value; /* owned */
} JsonMember;

struct JsonValue {
    JsonType type;
    int boolean;
    double number;
    char* string; /* owned, JSON_STRING only */

    JsonValue** array_items; /* owned array of owned pointers, JSON_ARRAY only */
    size_t array_len, array_cap;

    JsonMember* members; /* owned array, JSON_OBJECT only */
    size_t member_len, member_cap;
};

JsonValue* json_new_null(void);
JsonValue* json_new_bool(int b);
JsonValue* json_new_number(double n);
JsonValue* json_new_string(const char* s);
JsonValue* json_new_array(void);
JsonValue* json_new_object(void);

/* Recursively frees v and everything it owns. Safe to call with NULL. */
void json_free(JsonValue* v);
/* Deep copy. */
JsonValue* json_clone(const JsonValue* v);

/* Takes ownership of `item`. */
void json_array_push(JsonValue* arr, JsonValue* item);
JsonValue* json_array_get(const JsonValue* arr, size_t idx); /* borrowed pointer */
size_t json_array_size(const JsonValue* arr);
/* Removes and frees the element at idx, shifting later elements down. */
void json_array_remove(JsonValue* arr, size_t idx);

/* Upsert. Takes ownership of `value`; frees any previous value at `key`. */
void json_object_set(JsonValue* obj, const char* key, JsonValue* value);
JsonValue* json_object_get(const JsonValue* obj, const char* key); /* borrowed pointer, NULL if absent */
int json_object_has(const JsonValue* obj, const char* key);

/* Returns NULL on parse error. Caller owns the returned value (json_free it). */
JsonValue* json_parse(const char* text);
/* Caller must free() the returned string. */
char* json_dump(const JsonValue* v, int indent);

#endif /* OMNIPACK_JSON_VALUE_H */
