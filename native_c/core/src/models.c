#include "omnipack/models.h"

#include <stdlib.h>
#include <string.h>
#include <math.h>

static void item_array_reserve(ItemArray* arr, size_t min_cap) {
    if (arr->cap >= min_cap) return;
    size_t new_cap = arr->cap == 0 ? 8 : arr->cap * 2;
    if (new_cap < min_cap) new_cap = min_cap;
    arr->data = (Item*)realloc(arr->data, new_cap * sizeof(Item));
    arr->cap = new_cap;
}

void item_array_init(ItemArray* arr) {
    arr->data = NULL;
    arr->len = 0;
    arr->cap = 0;
}

void item_array_push(ItemArray* arr, Item item) {
    item_array_reserve(arr, arr->len + 1);
    arr->data[arr->len++] = item;
}

void item_array_free(ItemArray* arr) {
    free(arr->data);
    arr->data = NULL;
    arr->len = 0;
    arr->cap = 0;
}

void item_array_copy_from(ItemArray* dst, const Item* src, size_t count) {
    item_array_init(dst);
    item_array_reserve(dst, count);
    memcpy(dst->data, src, count * sizeof(Item));
    dst->len = count;
}

Item item_make(const char* id, double w, double h, double d) {
    Item it;
    memset(&it, 0, sizeof(it));
    strncpy(it.id, id, OMNIPACK_ID_MAX - 1);
    it.width = w;
    it.height = h;
    it.depth = d;
    it.weight = 0.0;
    it.max_stack_weight = 1000000.0;
    it.strategy = STRAT_NONE;
    it.stop_id = 0;
    it.shape_type = SHAPE_BOX;
    it.allowed_rotations_count = 6;
    for (int i = 0; i < 6; ++i) it.allowed_rotations[i] = (Rotation)i;
    it.position.x = it.position.y = it.position.z = 0.0;
    it.rotation = ROT_W_H_D;
    return it;
}

double item_volume(const Item* it) {
    return it->width * it->height * it->depth;
}

void item_get_dimension(const Item* it, double out[3]) {
    double w = it->width, h = it->height, d = it->depth;
    switch (it->rotation) {
        case ROT_W_H_D: out[0] = w; out[1] = h; out[2] = d; break;
        case ROT_H_W_D: out[0] = h; out[1] = w; out[2] = d; break;
        case ROT_H_D_W: out[0] = h; out[1] = d; out[2] = w; break;
        case ROT_D_H_W: out[0] = d; out[1] = h; out[2] = w; break;
        case ROT_D_W_H: out[0] = d; out[1] = w; out[2] = h; break;
        case ROT_W_D_H: out[0] = w; out[1] = d; out[2] = h; break;
        default: out[0] = w; out[1] = h; out[2] = d; break;
    }
}

int item_allows_rotation(const Item* it, Rotation r) {
    for (int i = 0; i < it->allowed_rotations_count; ++i) {
        if (it->allowed_rotations[i] == r) return 1;
    }
    return 0;
}

void container_init(Container* c, const char* id, double w, double h, double d) {
    container_init_full(c, id, w, h, d, 1000000.0, SHAPE_BOX);
}

void container_init_full(Container* c, const char* id, double w, double h, double d,
                          double max_weight, ShapeType shape) {
    memset(c, 0, sizeof(*c));
    strncpy(c->id, id, OMNIPACK_ID_MAX - 1);
    c->width = w;
    c->height = h;
    c->depth = d;
    c->max_weight = max_weight;
    c->shape_type = shape;
    item_array_init(&c->items);
}

void container_free(Container* c) {
    item_array_free(&c->items);
}

double container_volume(const Container* c) {
    return c->width * c->height * c->depth;
}

double container_remaining_volume(const Container* c) {
    double used = 0.0;
    for (size_t i = 0; i < c->items.len; ++i) used += item_volume(&c->items.data[i]);
    return container_volume(c) - used;
}

double container_volume_utilization(const Container* c) {
    if (c->items.len == 0) return 0.0;
    double used = 0.0;
    for (size_t i = 0; i < c->items.len; ++i) used += item_volume(&c->items.data[i]);
    return (used / container_volume(c)) * 100.0;
}

ContainerStats container_calculate_stats(const Container* c) {
    ContainerStats stats;
    memset(&stats, 0, sizeof(stats));
    stats.stability_score = 100.0;
    stats.item_count = (int)c->items.len;

    double used_vol = 0.0, total_weight = 0.0;
    for (size_t i = 0; i < c->items.len; ++i) {
        used_vol += item_volume(&c->items.data[i]);
        total_weight += c->items.data[i].weight;
    }

    double com_x = 0.0, com_y = 0.0, com_z = 0.0;
    if (total_weight > 0.0) {
        for (size_t i = 0; i < c->items.len; ++i) {
            const Item* it = &c->items.data[i];
            double dim[3];
            item_get_dimension(it, dim);
            com_x += (it->position.x + dim[0] / 2.0) * it->weight;
            com_y += (it->position.y + dim[1] / 2.0) * it->weight;
            com_z += (it->position.z + dim[2] / 2.0) * it->weight;
        }
        com_x /= total_weight; com_y /= total_weight; com_z /= total_weight;
    }

    double vol = container_volume(c);
    stats.volume_utilization = vol > 0.0 ? (used_vol / vol) * 100.0 : 0.0;
    stats.weight_utilization = c->max_weight > 0.0 ? (total_weight / c->max_weight) * 100.0 : 0.0;
    stats.total_weight = total_weight;
    stats.center_of_mass.x = com_x;
    stats.center_of_mass.y = com_y;
    stats.center_of_mass.z = com_z;
    return stats;
}

typedef struct {
    const Item* item;
} ItemPtr;

static int cmp_item_ptr_by_z_desc(const void* a, const void* b) {
    const Item* ia = *(const Item* const*)a;
    const Item* ib = *(const Item* const*)b;
    if (ia->position.z > ib->position.z) return -1;
    if (ia->position.z < ib->position.z) return 1;
    return 0;
}

double container_calculate_accessibility(const Container* c) {
    size_t n = c->items.len;
    if (n == 0) return 100.0;

    const Item** sorted_by_z = (const Item**)malloc(n * sizeof(const Item*));
    for (size_t i = 0; i < n; ++i) sorted_by_z[i] = &c->items.data[i];
    qsort(sorted_by_z, n, sizeof(const Item*), cmp_item_ptr_by_z_desc);

    /* Track which items are blocked by pointer identity (index into c->items.data). */
    int* blocked = (int*)calloc(n, sizeof(int));

    for (size_t i = 0; i < n; ++i) {
        const Item* item = sorted_by_z[i];
        for (size_t j = i + 1; j < n; ++j) {
            const Item* other = sorted_by_z[j];
            if (other->stop_id < item->stop_id) {
                double idim[3], odim[3];
                item_get_dimension(item, idim);
                item_get_dimension(other, odim);
                int overlap_x = item->position.x < other->position.x + odim[0] &&
                                item->position.x + idim[0] > other->position.x;
                int overlap_y = item->position.y < other->position.y + odim[1] &&
                                item->position.y + idim[1] > other->position.y;
                if (overlap_x && overlap_y) {
                    size_t other_idx = (size_t)(other - c->items.data);
                    blocked[other_idx] = 1;
                }
            }
        }
    }

    size_t blocked_count = 0;
    for (size_t i = 0; i < n; ++i) blocked_count += (size_t)blocked[i];

    free(sorted_by_z);
    free(blocked);

    return ((double)(n - blocked_count) / (double)n) * 100.0;
}
