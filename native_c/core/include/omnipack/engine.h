#ifndef OMNIPACK_ENGINE_H
#define OMNIPACK_ENGINE_H
/* Plain-C port of native/core/include/omnipack/engine.hpp
   (itself a port of src/omnipack/core/engine.py and engine_v2.py) */

#include "omnipack/extreme_points.h"
#include "omnipack/models.h"
#include "omnipack/prng.h"

/* Simple Best-Fit Decreasing, single-rotation-at-a-time greedy placement.
   No RNG/threading needed - suitable as a low-power/mobile fallback mode. */
typedef struct {
    Container* container; /* borrowed reference, not owned */
    double stability_factor;
    PackingVersus versus;
    ExtremePointSet extreme_points;
} Level1Engine;

void level1_engine_init(Level1Engine* e, Container* container, double stability_factor, PackingVersus versus);
void level1_engine_free(Level1Engine* e);
/* Packs `count` items from `items` into e->container. Items that fail to
   place are appended (as copies) into `unpacked_out` (must be initialized). */
void level1_engine_pack(Level1Engine* e, const Item* items, size_t count, ItemArray* unpacked_out);

/* Extreme-point + GRASP heuristic packer (matches Level2Engine). */
typedef struct {
    Container* container; /* borrowed reference, not owned */
    double stability_factor;
    PackingVersus versus;
    double random_disturbance;
    Prng rng;
    ExtremePointSet extreme_points;
} Level2Engine;

void level2_engine_init(Level2Engine* e, Container* container, double stability_factor,
                         PackingVersus versus, double random_disturbance);
void level2_engine_free(Level2Engine* e);
/* Test-only hook mirroring direct access to the Python/C++ engine's
   `extreme_points` set. */
void level2_engine_seed_extreme_point(Level2Engine* e, double x, double y, double z);
void level2_engine_pack(Level2Engine* e, const Item* items, size_t count, int grasp_k,
                         ItemArray* unpacked_out);

#endif /* OMNIPACK_ENGINE_H */
