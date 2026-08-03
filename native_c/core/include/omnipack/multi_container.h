#ifndef OMNIPACK_MULTI_CONTAINER_H
#define OMNIPACK_MULTI_CONTAINER_H
/* Plain-C port of native/core/include/omnipack/multi_container.hpp
   (itself a port of src/omnipack/core/multi_container.py) */

#include "omnipack/models.h"

typedef struct {
    Container* data;
    size_t len;
    size_t cap;
} ContainerArray;

void container_array_init(ContainerArray* arr);
/* Takes ownership of `c` (its items array will be freed by container_array_free). */
void container_array_push(ContainerArray* arr, Container c);
void container_array_free(ContainerArray* arr);

typedef struct {
    char id[OMNIPACK_ID_MAX];
    double width, height, depth;
    double max_weight;
    ShapeType shape_type;
    PackingVersus versus;
} MultiContainerEngine;

void multi_container_engine_init(MultiContainerEngine* e, const char* id, double w, double h, double d,
                                  double max_weight, ShapeType shape, PackingVersus versus);

/* mode: "level1" or "level2" (anything else falls back to level2, matching
   multi_container.py's _get_engine default). Appends packed containers into
   `out` (must be initialized) until `items` is exhausted or a container
   packs nothing (infinite-loop guard, matches the Python original). */
void multi_container_engine_pack_all(const MultiContainerEngine* e, const Item* items, size_t count,
                                      const char* mode, double stability_factor, int grasp_k,
                                      double random_disturbance, ContainerArray* out);

#endif /* OMNIPACK_MULTI_CONTAINER_H */
