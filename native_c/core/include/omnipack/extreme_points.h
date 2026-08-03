#ifndef OMNIPACK_EXTREME_POINTS_H
#define OMNIPACK_EXTREME_POINTS_H
/* Plain-C port of native/core/include/omnipack/extreme_points.hpp
   (itself a port of src/omnipack/core/ep.py) */

#include <stddef.h>

#include "omnipack/models.h"

typedef struct {
    double x, y, z;
} ExtremePoint;

int extreme_point_eq(ExtremePoint a, ExtremePoint b);

/* A dynamic array acting as a set: linear insert-if-absent/remove.
   Containers stay small enough (dozens to low hundreds of items) that O(n)
   dedup is simpler and safer in C than hand-rolling a hash table. */
typedef struct {
    ExtremePoint* data;
    size_t len;
    size_t cap;
} ExtremePointSet;

void eps_init(ExtremePointSet* s);
void eps_free(ExtremePointSet* s);
int eps_contains(const ExtremePointSet* s, ExtremePoint p);
/* No-op if an equal point (within tolerance) is already present. */
void eps_insert(ExtremePointSet* s, ExtremePoint p);
/* No-op if not present. */
void eps_remove(ExtremePointSet* s, ExtremePoint p);

/* Same layout as ExtremePointSet, used for plain (non-deduped) output lists.
   Use eps_init/eps_free on it too; ep_list_push just skips the dedup check. */
typedef ExtremePointSet ExtremePointList;

void ep_list_push(ExtremePointList* l, ExtremePoint p);

/* Appends the item's forward + backward candidate extreme points (clipped to
   the container bounds and deduped) into `out` (must already be initialized;
   not cleared first, so callers can accumulate across multiple items). */
void generate_extreme_points(const Container* container, const Item* item, ExtremePointList* out);

/* Pure-fallback validity check (naive box/sphere collision, no threading):
   filters `eps` down to points where `item` fits in `container` without
   colliding with any existing item. Appends survivors into `out`. */
void get_valid_ep(const Container* container, const Item* item,
                   const ExtremePoint* eps, size_t eps_count, ExtremePointList* out);

#endif /* OMNIPACK_EXTREME_POINTS_H */
