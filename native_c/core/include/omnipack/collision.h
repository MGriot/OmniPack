#ifndef OMNIPACK_COLLISION_H
#define OMNIPACK_COLLISION_H
/* Plain-C port of native/core/include/omnipack/collision.hpp
   (itself a port of src/omnipack/core/accelerated.py).
   Plain loops - no JIT needed, this already outperforms the Python/Numba
   baseline; it's a straight translation, not an optimization pass. */

#include <stddef.h>
#include <stdint.h>

#include "omnipack/models.h"

double get_overlap_area(double ax1, double ay1, double ax2, double ay2,
                         double bx1, double by1, double bx2, double by2);

typedef struct {
    Vec3 pos;
    double w, h, d;
    int shape;
    double weight;
    double max_stack_weight;
} ExistingItem;

/* Writes n cumulative loads into out_loads (caller-allocated, n entries). */
void calculate_cumulative_loads(const ExistingItem* existing, size_t n, double* out_loads);

int check_collision(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                     double ox, double oy, double oz, double ow, double oh, double od, int ost);

int is_in_container(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                     double cw, double ch, double cd, int cst);

typedef struct {
    double w, h, d;
    int shape;
} RotDim;

/* Writes a flat num_eps x num_rots grid into results_out (caller-allocated,
   num_eps*num_rots bytes): results_out[i * num_rots + j] is 1 if rotation j
   at extreme point i is a valid placement. */
void evaluate_positions(const Vec3* eps, size_t num_eps,
                         const RotDim* item_dims, size_t num_rots,
                         const ExistingItem* existing, size_t n_existing,
                         double cw, double ch, double cd, int container_shape,
                         double item_weight, int strategy,
                         unsigned char* results_out);

#endif /* OMNIPACK_COLLISION_H */
