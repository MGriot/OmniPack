#include "omnipack/multi_container.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "omnipack/engine.h"

void container_array_init(ContainerArray* arr) {
    arr->data = NULL;
    arr->len = 0;
    arr->cap = 0;
}

void container_array_push(ContainerArray* arr, Container c) {
    if (arr->len >= arr->cap) {
        size_t new_cap = arr->cap ? arr->cap * 2 : 4;
        arr->data = (Container*)realloc(arr->data, new_cap * sizeof(Container));
        arr->cap = new_cap;
    }
    arr->data[arr->len++] = c;
}

void container_array_free(ContainerArray* arr) {
    for (size_t i = 0; i < arr->len; ++i) container_free(&arr->data[i]);
    free(arr->data);
    arr->data = NULL;
    arr->len = 0;
    arr->cap = 0;
}

void multi_container_engine_init(MultiContainerEngine* e, const char* id, double w, double h, double d,
                                  double max_weight, ShapeType shape, PackingVersus versus) {
    memset(e, 0, sizeof(*e));
    strncpy(e->id, id, OMNIPACK_ID_MAX - 1);
    e->width = w; e->height = h; e->depth = d;
    e->max_weight = max_weight;
    e->shape_type = shape;
    e->versus = versus;
}

void multi_container_engine_pack_all(const MultiContainerEngine* e, const Item* items, size_t count,
                                      const char* mode, double stability_factor, int grasp_k,
                                      double random_disturbance, ContainerArray* out) {
    /* `remaining` mirrors multi_container.py's deep-copied working list:
       items not yet packed, in their original (unpositioned) state. */
    ItemArray remaining;
    item_array_copy_from(&remaining, items, count);

    int container_count = 0;
    int use_level1 = mode && strcmp(mode, "level1") == 0;

    while (remaining.len > 0) {
        container_count++;
        char cid[OMNIPACK_ID_MAX];
        snprintf(cid, sizeof(cid), "%s_%d", e->id, container_count);

        Container current;
        container_init_full(&current, cid, e->width, e->height, e->depth, e->max_weight, e->shape_type);

        ItemArray unpacked;
        item_array_init(&unpacked);

        if (use_level1) {
            Level1Engine engine;
            level1_engine_init(&engine, &current, stability_factor, e->versus);
            level1_engine_pack(&engine, remaining.data, remaining.len, &unpacked);
            level1_engine_free(&engine);
        } else {
            Level2Engine engine;
            level2_engine_init(&engine, &current, stability_factor, e->versus, random_disturbance);
            level2_engine_pack(&engine, remaining.data, remaining.len, grasp_k, &unpacked);
            level2_engine_free(&engine);
        }

        if (current.items.len == 0) {
            container_free(&current);
            item_array_free(&unpacked);
            break;
        }

        container_array_push(out, current);
        item_array_free(&remaining);
        remaining = unpacked; /* items still needing a container, unpositioned */
    }

    item_array_free(&remaining);
}
