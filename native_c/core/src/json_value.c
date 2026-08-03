#include "omnipack/json_value.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---------- construction ---------- */

static JsonValue* json_new(JsonType t) {
    JsonValue* v = (JsonValue*)calloc(1, sizeof(JsonValue));
    v->type = t;
    return v;
}

JsonValue* json_new_null(void) { return json_new(JSON_NULL); }

JsonValue* json_new_bool(int b) {
    JsonValue* v = json_new(JSON_BOOL);
    v->boolean = b ? 1 : 0;
    return v;
}

JsonValue* json_new_number(double n) {
    JsonValue* v = json_new(JSON_NUMBER);
    v->number = n;
    return v;
}

JsonValue* json_new_string(const char* s) {
    JsonValue* v = json_new(JSON_STRING);
    v->string = (char*)malloc(strlen(s) + 1);
    strcpy(v->string, s);
    return v;
}

JsonValue* json_new_array(void) { return json_new(JSON_ARRAY); }
JsonValue* json_new_object(void) { return json_new(JSON_OBJECT); }

void json_free(JsonValue* v) {
    if (!v) return;
    free(v->string);
    for (size_t i = 0; i < v->array_len; ++i) json_free(v->array_items[i]);
    free(v->array_items);
    for (size_t i = 0; i < v->member_len; ++i) {
        free(v->members[i].key);
        json_free(v->members[i].value);
    }
    free(v->members);
    free(v);
}

JsonValue* json_clone(const JsonValue* v) {
    if (!v) return NULL;
    switch (v->type) {
        case JSON_NULL: return json_new_null();
        case JSON_BOOL: return json_new_bool(v->boolean);
        case JSON_NUMBER: return json_new_number(v->number);
        case JSON_STRING: return json_new_string(v->string);
        case JSON_ARRAY: {
            JsonValue* out = json_new_array();
            for (size_t i = 0; i < v->array_len; ++i) json_array_push(out, json_clone(v->array_items[i]));
            return out;
        }
        case JSON_OBJECT: {
            JsonValue* out = json_new_object();
            for (size_t i = 0; i < v->member_len; ++i) json_object_set(out, v->members[i].key, json_clone(v->members[i].value));
            return out;
        }
    }
    return json_new_null();
}

/* ---------- array/object operations ---------- */

void json_array_push(JsonValue* arr, JsonValue* item) {
    if (arr->array_len >= arr->array_cap) {
        size_t new_cap = arr->array_cap ? arr->array_cap * 2 : 8;
        arr->array_items = (JsonValue**)realloc(arr->array_items, new_cap * sizeof(JsonValue*));
        arr->array_cap = new_cap;
    }
    arr->array_items[arr->array_len++] = item;
}

JsonValue* json_array_get(const JsonValue* arr, size_t idx) {
    if (idx >= arr->array_len) return NULL;
    return arr->array_items[idx];
}

size_t json_array_size(const JsonValue* arr) { return arr->array_len; }

void json_array_remove(JsonValue* arr, size_t idx) {
    if (idx >= arr->array_len) return;
    json_free(arr->array_items[idx]);
    for (size_t i = idx; i + 1 < arr->array_len; ++i) arr->array_items[i] = arr->array_items[i + 1];
    arr->array_len--;
}

void json_object_set(JsonValue* obj, const char* key, JsonValue* value) {
    for (size_t i = 0; i < obj->member_len; ++i) {
        if (strcmp(obj->members[i].key, key) == 0) {
            json_free(obj->members[i].value);
            obj->members[i].value = value;
            return;
        }
    }
    if (obj->member_len >= obj->member_cap) {
        size_t new_cap = obj->member_cap ? obj->member_cap * 2 : 8;
        obj->members = (JsonMember*)realloc(obj->members, new_cap * sizeof(JsonMember));
        obj->member_cap = new_cap;
    }
    obj->members[obj->member_len].key = (char*)malloc(strlen(key) + 1);
    strcpy(obj->members[obj->member_len].key, key);
    obj->members[obj->member_len].value = value;
    obj->member_len++;
}

JsonValue* json_object_get(const JsonValue* obj, const char* key) {
    for (size_t i = 0; i < obj->member_len; ++i) {
        if (strcmp(obj->members[i].key, key) == 0) return obj->members[i].value;
    }
    return NULL;
}

int json_object_has(const JsonValue* obj, const char* key) {
    return json_object_get(obj, key) != NULL;
}

/* ---------- parsing ---------- */

typedef struct {
    const char* s;
    size_t pos;
    size_t len;
} Parser;

static void skip_ws(Parser* p) {
    while (p->pos < p->len && isspace((unsigned char)p->s[p->pos])) p->pos++;
}

static JsonValue* parse_value(Parser* p);

static char* parse_raw_string(Parser* p) {
    /* Expects p->s[p->pos] == '"'. Returns a malloc'd, NUL-terminated string. */
    p->pos++; /* opening quote */
    size_t cap = 32, len = 0;
    char* out = (char*)malloc(cap);
    while (p->pos < p->len && p->s[p->pos] != '"') {
        char c = p->s[p->pos];
        char to_add;
        if (c == '\\' && p->pos + 1 < p->len) {
            char next = p->s[p->pos + 1];
            switch (next) {
                case 'n': to_add = '\n'; break;
                case 't': to_add = '\t'; break;
                case 'r': to_add = '\r'; break;
                case '"': to_add = '"'; break;
                case '\\': to_add = '\\'; break;
                case '/': to_add = '/'; break;
                default: to_add = next; break;
            }
            p->pos += 2;
        } else {
            to_add = c;
            p->pos += 1;
        }
        if (len + 1 >= cap) { cap *= 2; out = (char*)realloc(out, cap); }
        out[len++] = to_add;
    }
    p->pos++; /* closing quote */
    out[len] = '\0';
    return out;
}

static JsonValue* parse_object(Parser* p) {
    JsonValue* v = json_new_object();
    p->pos++; /* '{' */
    skip_ws(p);
    if (p->pos < p->len && p->s[p->pos] == '}') { p->pos++; return v; }
    while (1) {
        skip_ws(p);
        char* key = parse_raw_string(p);
        skip_ws(p);
        if (p->pos >= p->len || p->s[p->pos] != ':') { free(key); json_free(v); return NULL; }
        p->pos++;
        JsonValue* val = parse_value(p);
        if (!val) { free(key); json_free(v); return NULL; }
        json_object_set(v, key, val);
        free(key);
        skip_ws(p);
        if (p->pos < p->len && p->s[p->pos] == ',') { p->pos++; continue; }
        if (p->pos < p->len && p->s[p->pos] == '}') { p->pos++; break; }
        json_free(v);
        return NULL;
    }
    return v;
}

static JsonValue* parse_array(Parser* p) {
    JsonValue* v = json_new_array();
    p->pos++; /* '[' */
    skip_ws(p);
    if (p->pos < p->len && p->s[p->pos] == ']') { p->pos++; return v; }
    while (1) {
        JsonValue* item = parse_value(p);
        if (!item) { json_free(v); return NULL; }
        json_array_push(v, item);
        skip_ws(p);
        if (p->pos < p->len && p->s[p->pos] == ',') { p->pos++; continue; }
        if (p->pos < p->len && p->s[p->pos] == ']') { p->pos++; break; }
        json_free(v);
        return NULL;
    }
    return v;
}

static JsonValue* parse_value(Parser* p) {
    skip_ws(p);
    if (p->pos >= p->len) return NULL;
    char c = p->s[p->pos];
    if (c == '{') return parse_object(p);
    if (c == '[') return parse_array(p);
    if (c == '"') { char* s = parse_raw_string(p); JsonValue* v = json_new_string(s); free(s); return v; }
    if (c == 't' && p->pos + 4 <= p->len && strncmp(p->s + p->pos, "true", 4) == 0) { p->pos += 4; return json_new_bool(1); }
    if (c == 'f' && p->pos + 5 <= p->len && strncmp(p->s + p->pos, "false", 5) == 0) { p->pos += 5; return json_new_bool(0); }
    if (c == 'n' && p->pos + 4 <= p->len && strncmp(p->s + p->pos, "null", 4) == 0) { p->pos += 4; return json_new_null(); }

    size_t start = p->pos;
    if (p->pos < p->len && (p->s[p->pos] == '-' || p->s[p->pos] == '+')) p->pos++;
    while (p->pos < p->len && (isdigit((unsigned char)p->s[p->pos]) || p->s[p->pos] == '.' ||
                                p->s[p->pos] == 'e' || p->s[p->pos] == 'E' ||
                                p->s[p->pos] == '+' || p->s[p->pos] == '-')) {
        p->pos++;
    }
    if (p->pos == start) return NULL; /* not a valid token */
    char* end = NULL;
    double n = strtod(p->s + start, &end);
    return json_new_number(n);
}

JsonValue* json_parse(const char* text) {
    Parser p;
    p.s = text;
    p.pos = 0;
    p.len = strlen(text);
    JsonValue* v = parse_value(&p);
    return v;
}

/* ---------- dumping ---------- */

typedef struct {
    char* data;
    size_t len, cap;
} StrBuf;

static void sb_init(StrBuf* b) { b->data = (char*)malloc(64); b->data[0] = '\0'; b->len = 0; b->cap = 64; }

static void sb_reserve(StrBuf* b, size_t extra) {
    if (b->len + extra + 1 <= b->cap) return;
    size_t new_cap = b->cap * 2;
    while (new_cap < b->len + extra + 1) new_cap *= 2;
    b->data = (char*)realloc(b->data, new_cap);
    b->cap = new_cap;
}

static void sb_append(StrBuf* b, const char* s) {
    size_t n = strlen(s);
    sb_reserve(b, n);
    memcpy(b->data + b->len, s, n + 1);
    b->len += n;
}

static void sb_append_char(StrBuf* b, char c) {
    sb_reserve(b, 1);
    b->data[b->len++] = c;
    b->data[b->len] = '\0';
}

static void sb_append_padding(StrBuf* b, int count) {
    for (int i = 0; i < count; ++i) sb_append_char(b, ' ');
}

static void sb_append_escaped(StrBuf* b, const char* s) {
    sb_append_char(b, '"');
    for (const char* p = s; *p; ++p) {
        switch (*p) {
            case '"': sb_append(b, "\\\""); break;
            case '\\': sb_append(b, "\\\\"); break;
            case '\n': sb_append(b, "\\n"); break;
            case '\t': sb_append(b, "\\t"); break;
            case '\r': sb_append(b, "\\r"); break;
            default: sb_append_char(b, *p);
        }
    }
    sb_append_char(b, '"');
}

static void dump_value(const JsonValue* v, StrBuf* b, int indent, int depth) {
    char num_buf[64];
    switch (v->type) {
        case JSON_NULL: sb_append(b, "null"); break;
        case JSON_BOOL: sb_append(b, v->boolean ? "true" : "false"); break;
        case JSON_NUMBER:
            if (v->number == (double)(long long)v->number) snprintf(num_buf, sizeof(num_buf), "%lld", (long long)v->number);
            else snprintf(num_buf, sizeof(num_buf), "%g", v->number);
            sb_append(b, num_buf);
            break;
        case JSON_STRING: sb_append_escaped(b, v->string); break;
        case JSON_ARRAY:
            if (v->array_len == 0) { sb_append(b, "[]"); break; }
            sb_append(b, "[\n");
            for (size_t i = 0; i < v->array_len; ++i) {
                sb_append_padding(b, indent * (depth + 1));
                dump_value(v->array_items[i], b, indent, depth + 1);
                if (i + 1 < v->array_len) sb_append_char(b, ',');
                sb_append_char(b, '\n');
            }
            sb_append_padding(b, indent * depth);
            sb_append_char(b, ']');
            break;
        case JSON_OBJECT:
            if (v->member_len == 0) { sb_append(b, "{}"); break; }
            sb_append(b, "{\n");
            for (size_t i = 0; i < v->member_len; ++i) {
                sb_append_padding(b, indent * (depth + 1));
                sb_append_escaped(b, v->members[i].key);
                sb_append(b, ": ");
                dump_value(v->members[i].value, b, indent, depth + 1);
                if (i + 1 < v->member_len) sb_append_char(b, ',');
                sb_append_char(b, '\n');
            }
            sb_append_padding(b, indent * depth);
            sb_append_char(b, '}');
            break;
    }
}

char* json_dump(const JsonValue* v, int indent) {
    StrBuf b;
    sb_init(&b);
    dump_value(v, &b, indent, 0);
    return b.data; /* caller frees */
}
