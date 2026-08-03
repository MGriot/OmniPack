#ifndef OMNIPACK_PRNG_H
#define OMNIPACK_PRNG_H
/* Small embedded xorshift32 PRNG, carried per-engine-instance so packing runs
   don't depend on global RNG state. Portable across MSVC/MinGW/Android bionic
   (unlike rand_r, which isn't available everywhere). */

#include <stdint.h>

typedef struct {
    uint32_t state;
} Prng;

void prng_seed(Prng* p, uint32_t seed);
/* Seeds from wall-clock time + a stack-address perturbation - "good enough"
   entropy for a packing heuristic's tie-breaking, not for cryptographic use. */
void prng_seed_auto(Prng* p);
uint32_t prng_next_u32(Prng* p);
/* Uniform double in [0, 1). */
double prng_next_double(Prng* p);
/* Uniform integer in [lo, hi] inclusive. */
int prng_next_range(Prng* p, int lo, int hi);

#endif /* OMNIPACK_PRNG_H */
