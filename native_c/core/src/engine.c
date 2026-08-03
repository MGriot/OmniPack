#include "omnipack/engine.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#include "omnipack/collision.h"

/* ---------- Level1Engine (simple best-fit decreasing) ---------- */

void level1_engine_init(Level1Engine* e, Container* container, double stability_factor, PackingVersus versus) {
    e->container = container;
    e->stability_factor = stability_factor;
    e->versus = versus;
    eps_init(&e->extreme_points);
    ExtremePoint origin = {0, 0, 0};
    eps_insert(&e->extreme_points, origin);
}

void level1_engine_free(Level1Engine* e) {
    eps_free(&e->extreme_points);
}

static int cmp_item_by_volume_desc(const void* a, const void* b) {
    double va = item_volume((const Item*)a);
    double vb = item_volume((const Item*)b);
    if (va > vb) return -1;
    if (va < vb) return 1;
    return 0;
}

void level1_engine_pack(Level1Engine* e, const Item* items, size_t count, ItemArray* unpacked_out) {
    Item* sorted = (Item*)malloc((count > 0 ? count : 1) * sizeof(Item));
    memcpy(sorted, items, count * sizeof(Item));
    qsort(sorted, count, sizeof(Item), cmp_item_by_volume_desc);

    for (size_t idx = 0; idx < count; ++idx) {
        Item item = sorted[idx];
        int found = 0;
        ExtremePoint best_ep = {0, 0, 0};
        Rotation best_rot = ROT_W_H_D;

        for (size_t epi = 0; epi < e->extreme_points.len && !found; ++epi) {
            ExtremePoint ep = e->extreme_points.data[epi];
            for (int ri = 0; ri < item.allowed_rotations_count; ++ri) {
                Rotation rot = item.allowed_rotations[ri];
                item.rotation = rot;
                double dim[3];
                item_get_dimension(&item, dim);
                double w = dim[0], h = dim[1], d = dim[2];

                if (ep.x + w <= e->container->width && ep.y + h <= e->container->height &&
                    ep.z + d <= e->container->depth) {
                    int collision = 0;
                    for (size_t oi = 0; oi < e->container->items.len; ++oi) {
                        const Item* other = &e->container->items.data[oi];
                        double odim[3];
                        item_get_dimension(other, odim);
                        double ow = odim[0], oh = odim[1], od = odim[2];
                        double ox = other->position.x, oy = other->position.y, oz = other->position.z;
                        if (!(ep.x + w <= ox || ep.x >= ox + ow ||
                              ep.y + h <= oy || ep.y >= oy + oh ||
                              ep.z + d <= oz || ep.z >= oz + od)) {
                            collision = 1;
                            break;
                        }
                    }
                    if (!collision) {
                        best_ep = ep;
                        best_rot = rot;
                        found = 1;
                        break;
                    }
                }
            }
        }

        if (found) {
            item.position.x = best_ep.x; item.position.y = best_ep.y; item.position.z = best_ep.z;
            item.rotation = best_rot;
            item_array_push(&e->container->items, item);
            eps_remove(&e->extreme_points, best_ep);
            generate_extreme_points(e->container, &item, &e->extreme_points);
        } else {
            item_array_push(unpacked_out, item);
        }
    }

    free(sorted);
}

/* ---------- Level2Engine (extreme-point + GRASP heuristic) ---------- */

void level2_engine_init(Level2Engine* e, Container* container, double stability_factor,
                         PackingVersus versus, double random_disturbance) {
    e->container = container;
    e->stability_factor = stability_factor;
    e->versus = versus;
    e->random_disturbance = random_disturbance;
    prng_seed_auto(&e->rng);
    eps_init(&e->extreme_points);
    ExtremePoint origin = {0, 0, 0};
    ExtremePoint back = {0, 0, container->depth};
    eps_insert(&e->extreme_points, origin);
    eps_insert(&e->extreme_points, back);
}

void level2_engine_free(Level2Engine* e) {
    eps_free(&e->extreme_points);
}

void level2_engine_seed_extreme_point(Level2Engine* e, double x, double y, double z) {
    ExtremePoint p = {x, y, z};
    eps_insert(&e->extreme_points, p);
}

static int strategy_rank(LoadingStrategy s) {
    switch (s) {
        case STRAT_LIFO: return 0;
        case STRAT_FIFO: return 1;
        default: return 2;
    }
}

typedef struct {
    int rank;
    int stop_id; /* compared ascending on -stop_id, i.e. descending stop_id */
    double neg_vol_key;
    size_t orig_idx;
} SortKey;

static int cmp_sort_key(const void* pa, const void* pb) {
    const SortKey* a = (const SortKey*)pa;
    const SortKey* b = (const SortKey*)pb;
    if (a->rank != b->rank) return a->rank < b->rank ? -1 : 1;
    int neg_a = -a->stop_id, neg_b = -b->stop_id;
    if (neg_a != neg_b) return neg_a < neg_b ? -1 : 1;
    if (a->neg_vol_key < b->neg_vol_key) return -1;
    if (a->neg_vol_key > b->neg_vol_key) return 1;
    return 0;
}

typedef struct {
    size_t ep_idx;
    int rot_idx;
    double score[4];
    int has_target_z;
    double target_z;
} Candidate;

static int cmp_candidate(const void* pa, const void* pb) {
    const Candidate* a = (const Candidate*)pa;
    const Candidate* b = (const Candidate*)pb;
    for (int i = 0; i < 4; ++i) {
        if (a->score[i] < b->score[i]) return -1;
        if (a->score[i] > b->score[i]) return 1;
    }
    return 0;
}

static void score_for_versus(PackingVersus versus, double ep_x, double ep_y, double z_val,
                              double balance_penalty, int is_back_candidate, double out[4]) {
    if (versus == VERSUS_LATERAL) { out[0] = z_val; out[1] = ep_y; out[2] = ep_x; out[3] = balance_penalty; }
    else if (versus == VERSUS_FLOOR_FIRST) { out[0] = ep_y; out[1] = z_val; out[2] = ep_x; out[3] = balance_penalty; }
    else if (versus == VERSUS_WALL_BUILDING) { out[0] = ep_x; out[1] = z_val; out[2] = ep_y; out[3] = 0.0; }
    else if (is_back_candidate) { out[0] = z_val; out[1] = ep_y; out[2] = ep_x; out[3] = balance_penalty; } /* LONGITUDINAL back-candidate */
    else { out[0] = ep_x; out[1] = ep_y; out[2] = z_val; out[3] = balance_penalty; } /* LONGITUDINAL normal candidate */
}

/* Places one item into e->container using the extreme-point + GRASP search.
   Returns 1 and writes the placed item (with position/rotation set) into
   *out_placed on success, 0 if no valid candidate was found. */
static int place_one(Level2Engine* e, Item item, int grasp_k, Item* out_placed) {
    size_t num_eps = e->extreme_points.len;
    ExtremePoint* eps_list = (ExtremePoint*)malloc((num_eps > 0 ? num_eps : 1) * sizeof(ExtremePoint));
    memcpy(eps_list, e->extreme_points.data, num_eps * sizeof(ExtremePoint));
    Vec3* eps_vec = (Vec3*)malloc((num_eps > 0 ? num_eps : 1) * sizeof(Vec3));
    for (size_t i = 0; i < num_eps; ++i) { eps_vec[i].x = eps_list[i].x; eps_vec[i].y = eps_list[i].y; eps_vec[i].z = eps_list[i].z; }

    RotDim rots_dims[6];
    Rotation orig_rot = item.rotation;
    for (int r = 0; r < 6; ++r) {
        item.rotation = (Rotation)r;
        double dim[3];
        item_get_dimension(&item, dim);
        rots_dims[r].w = dim[0]; rots_dims[r].h = dim[1]; rots_dims[r].d = dim[2];
        rots_dims[r].shape = (int)item.shape_type;
    }
    item.rotation = orig_rot;

    size_t n_existing = e->container->items.len;
    ExistingItem* existing = (ExistingItem*)malloc((n_existing > 0 ? n_existing : 1) * sizeof(ExistingItem));
    for (size_t i = 0; i < n_existing; ++i) {
        const Item* ex = &e->container->items.data[i];
        double dim[3];
        item_get_dimension(ex, dim);
        existing[i].pos = ex->position;
        existing[i].w = dim[0]; existing[i].h = dim[1]; existing[i].d = dim[2];
        existing[i].shape = (int)ex->shape_type;
        existing[i].weight = ex->weight;
        existing[i].max_stack_weight = ex->max_stack_weight;
    }

    unsigned char* valid_mask = (unsigned char*)malloc((num_eps > 0 ? num_eps : 1) * 6);
    evaluate_positions(eps_vec, num_eps, rots_dims, 6, existing, n_existing,
                        e->container->width, e->container->height, e->container->depth,
                        (int)e->container->shape_type, item.weight, (int)item.strategy, valid_mask);

    Candidate* candidates = NULL;
    size_t cand_len = 0, cand_cap = 0;

    for (size_t i = 0; i < num_eps; ++i) {
        ExtremePoint ep = eps_list[i];
        for (int j = 0; j < 6; ++j) {
            Rotation rot = (Rotation)j;
            if (!item_allows_rotation(&item, rot)) continue;

            if (valid_mask[i * 6 + j]) {
                item.rotation = rot;
                double dim[3];
                item_get_dimension(&item, dim);
                double bw = dim[0], bd = dim[2];
                double z_val = (item.strategy == STRAT_LIFO) ? (e->container->depth - (ep.z + bd)) : ep.z;
                double balance_penalty = fabs(ep.x + bw / 2.0 - e->container->width / 2.0) / e->container->width;

                double score[4];
                score_for_versus(e->versus, ep.x, ep.y, z_val, balance_penalty, 0, score);
                if (e->random_disturbance > 0.0) {
                    double jitter = (prng_next_double(&e->rng) - 0.5) * e->random_disturbance;
                    for (int s = 0; s < 4; ++s) score[s] += jitter;
                }

                if (cand_len >= cand_cap) {
                    cand_cap = cand_cap ? cand_cap * 2 : 16;
                    candidates = (Candidate*)realloc(candidates, cand_cap * sizeof(Candidate));
                }
                Candidate* c = &candidates[cand_len++];
                c->ep_idx = i; c->rot_idx = j; memcpy(c->score, score, sizeof(score));
                c->has_target_z = 0; c->target_z = 0.0;
            }

            if (item.strategy == STRAT_LIFO) {
                item.rotation = rot;
                double dim[3];
                item_get_dimension(&item, dim);
                double bw = dim[0], bd = dim[2];
                double target_z = ep.z - bd;
                if (target_z >= -0.001) {
                    Vec3 temp_ep = {ep.x, ep.y, target_z};
                    RotDim single_rot = rots_dims[j];
                    unsigned char back_mask = 0;
                    evaluate_positions(&temp_ep, 1, &single_rot, 1, existing, n_existing,
                                        e->container->width, e->container->height, e->container->depth,
                                        (int)e->container->shape_type, item.weight, (int)item.strategy, &back_mask);
                    if (back_mask) {
                        double z_val = e->container->depth - (target_z + bd);
                        double balance_penalty = fabs(ep.x + bw / 2.0 - e->container->width / 2.0) / e->container->width;
                        double score[4];
                        score_for_versus(e->versus, ep.x, ep.y, z_val, balance_penalty, 1, score);
                        if (e->random_disturbance > 0.0) {
                            double jitter = (prng_next_double(&e->rng) - 0.5) * e->random_disturbance;
                            for (int s = 0; s < 4; ++s) score[s] += jitter;
                        }

                        if (cand_len >= cand_cap) {
                            cand_cap = cand_cap ? cand_cap * 2 : 16;
                            candidates = (Candidate*)realloc(candidates, cand_cap * sizeof(Candidate));
                        }
                        Candidate* c = &candidates[cand_len++];
                        c->ep_idx = i; c->rot_idx = j; memcpy(c->score, score, sizeof(score));
                        c->has_target_z = 1; c->target_z = target_z;
                    }
                }
            }
        }
    }

    int placed_ok = 0;
    if (cand_len > 0) {
        qsort(candidates, cand_len, sizeof(Candidate), cmp_candidate);
        size_t actual_k = grasp_k > 1 ? (size_t)grasp_k : 1;
        if (actual_k > cand_len) actual_k = cand_len;
        size_t chosen_idx = 0;
        if (actual_k > 1) chosen_idx = (size_t)prng_next_range(&e->rng, 0, (int)actual_k - 1);
        Candidate chosen = candidates[chosen_idx];
        ExtremePoint ep = eps_list[chosen.ep_idx];

        Item placed = item;
        if (chosen.has_target_z) { placed.position.x = ep.x; placed.position.y = ep.y; placed.position.z = chosen.target_z; }
        else { placed.position.x = ep.x; placed.position.y = ep.y; placed.position.z = ep.z; }
        placed.rotation = (Rotation)chosen.rot_idx;

        item_array_push(&e->container->items, placed);
        eps_remove(&e->extreme_points, ep);
        generate_extreme_points(e->container, &placed, &e->extreme_points);

        *out_placed = placed;
        placed_ok = 1;
    }

    free(eps_list);
    free(eps_vec);
    free(existing);
    free(valid_mask);
    free(candidates);

    return placed_ok;
}

void level2_engine_pack(Level2Engine* e, const Item* items, size_t count, int grasp_k, ItemArray* unpacked_out) {
    SortKey* keys = (SortKey*)malloc((count > 0 ? count : 1) * sizeof(SortKey));
    for (size_t i = 0; i < count; ++i) {
        keys[i].rank = strategy_rank(items[i].strategy);
        keys[i].stop_id = items[i].stop_id;
        double vol_disturb = (prng_next_double(&e->rng) - 0.5) * e->random_disturbance * item_volume(&items[i]);
        keys[i].neg_vol_key = -(item_volume(&items[i]) + vol_disturb);
        keys[i].orig_idx = i;
    }
    qsort(keys, count, sizeof(SortKey), cmp_sort_key);

    for (size_t i = 0; i < count; ++i) {
        Item item = items[keys[i].orig_idx];
        Item placed;
        int ok = place_one(e, item, grasp_k, &placed);
        if (!ok) item_array_push(unpacked_out, item);
    }

    free(keys);
}
