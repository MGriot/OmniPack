#include "omnipack/prng.h"

#include <time.h>

void prng_seed(Prng* p, uint32_t seed) {
    /* xorshift32 requires a non-zero state. */
    p->state = seed ? seed : 0xA5A5A5A5u;
}

void prng_seed_auto(Prng* p) {
    /* A stack address alone doesn't vary across repeated calls at the same
       call depth (common in tight loops), so mix in a call counter too. */
    static uint32_t call_counter = 0;
    call_counter++;

    int stack_marker;
    uint32_t t = (uint32_t)time(NULL);
    uint32_t addr_entropy = (uint32_t)(size_t)&stack_marker;
    prng_seed(p, (t ^ (addr_entropy * 2654435761u)) + call_counter * 0x9E3779B9u);
}

uint32_t prng_next_u32(Prng* p) {
    uint32_t x = p->state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    p->state = x;
    return x;
}

double prng_next_double(Prng* p) {
    /* 24 bits of precision, in [0, 1). */
    return (double)(prng_next_u32(p) >> 8) / (double)(1u << 24);
}

int prng_next_range(Prng* p, int lo, int hi) {
    if (hi <= lo) return lo;
    uint32_t span = (uint32_t)(hi - lo + 1);
    return lo + (int)(prng_next_u32(p) % span);
}
