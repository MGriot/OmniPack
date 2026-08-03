#include "omnipack/extreme_points.h"

#include <math.h>
#include <stdlib.h>

int extreme_point_eq(ExtremePoint a, ExtremePoint b) {
    return fabs(a.x - b.x) < 0.001 && fabs(a.y - b.y) < 0.001 && fabs(a.z - b.z) < 0.001;
}

static void eps_reserve(ExtremePointSet* s, size_t min_cap) {
    if (s->cap >= min_cap) return;
    size_t new_cap = s->cap == 0 ? 8 : s->cap * 2;
    if (new_cap < min_cap) new_cap = min_cap;
    s->data = (ExtremePoint*)realloc(s->data, new_cap * sizeof(ExtremePoint));
    s->cap = new_cap;
}

void eps_init(ExtremePointSet* s) {
    s->data = NULL;
    s->len = 0;
    s->cap = 0;
}

void eps_free(ExtremePointSet* s) {
    free(s->data);
    s->data = NULL;
    s->len = 0;
    s->cap = 0;
}

int eps_contains(const ExtremePointSet* s, ExtremePoint p) {
    for (size_t i = 0; i < s->len; ++i) {
        if (extreme_point_eq(s->data[i], p)) return 1;
    }
    return 0;
}

void eps_insert(ExtremePointSet* s, ExtremePoint p) {
    if (eps_contains(s, p)) return;
    eps_reserve(s, s->len + 1);
    s->data[s->len++] = p;
}

void eps_remove(ExtremePointSet* s, ExtremePoint p) {
    for (size_t i = 0; i < s->len; ++i) {
        if (extreme_point_eq(s->data[i], p)) {
            s->data[i] = s->data[s->len - 1];
            s->len--;
            return;
        }
    }
}

void ep_list_push(ExtremePointList* l, ExtremePoint p) {
    eps_reserve(l, l->len + 1);
    l->data[l->len++] = p;
}

void generate_extreme_points(const Container* container, const Item* item, ExtremePointList* out) {
    double ix = item->position.x, iy = item->position.y, iz = item->position.z;
    double idim[3];
    item_get_dimension(item, idim);
    double iw = idim[0], ih = idim[1], id = idim[2];
    const ItemArray* existing = &container->items;

    ExtremePoint potential_pts[4] = {
        {ix, iy, iz + id}, /* Z first */
        {ix, iy + ih, iz}, /* Y second */
        {ix + iw, iy, iz}, /* X third */
        {ix, iy, iz},      /* LIFO / backward point */
    };

    /* Upper bound: 5 candidates per potential point. */
    ExtremePoint final_pts[20];
    size_t final_count = 0;

    for (int pi = 0; pi < 4; ++pi) {
        double px = potential_pts[pi].x, py = potential_pts[pi].y, pz = potential_pts[pi].z;

        double max_x = 0.0;
        for (size_t k = 0; k < existing->len; ++k) {
            const Item* o = &existing->data[k];
            double odim[3]; item_get_dimension(o, odim);
            double ox = o->position.x, oy = o->position.y, oz = o->position.z;
            double ow = odim[0], oh = odim[1], odd = odim[2];
            if (ox + ow <= px + 0.001 && (oy < py + 0.001 && oy + oh > py - 0.001) &&
                (oz < pz + 0.001 && oz + odd > pz - 0.001)) {
                if (ox + ow > max_x) max_x = ox + ow;
            }
        }

        double max_y = 0.0;
        for (size_t k = 0; k < existing->len; ++k) {
            const Item* o = &existing->data[k];
            double odim[3]; item_get_dimension(o, odim);
            double ox = o->position.x, oy = o->position.y, oz = o->position.z;
            double ow = odim[0], oh = odim[1], odd = odim[2];
            if (oy + oh <= py + 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oz < pz + 0.001 && oz + odd > pz - 0.001)) {
                if (oy + oh > max_y) max_y = oy + oh;
            }
        }

        double max_z = 0.0;
        for (size_t k = 0; k < existing->len; ++k) {
            const Item* o = &existing->data[k];
            double odim[3]; item_get_dimension(o, odim);
            double ox = o->position.x, oy = o->position.y, oz = o->position.z;
            double ow = odim[0], oh = odim[1], odd = odim[2];
            if (oz + odd <= pz + 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oy < py + 0.001 && oy + oh > py - 0.001)) {
                if (oz + odd > max_z) max_z = oz + odd;
            }
        }

        double min_z = container->depth;
        for (size_t k = 0; k < existing->len; ++k) {
            const Item* o = &existing->data[k];
            double odim[3]; item_get_dimension(o, odim);
            double ox = o->position.x, oy = o->position.y, oz = o->position.z;
            double ow = odim[0], oh = odim[1];
            if (oz >= pz - 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oy < py + 0.001 && oy + oh > py - 0.001)) {
                if (oz < min_z) min_z = oz;
            }
        }

        final_pts[final_count++] = (ExtremePoint){px, py, pz};
        if (px > max_x) final_pts[final_count++] = (ExtremePoint){max_x, py, pz};
        if (py > max_y) final_pts[final_count++] = (ExtremePoint){px, max_y, pz};
        if (pz > max_z) final_pts[final_count++] = (ExtremePoint){px, py, max_z};
        if (pz < min_z) final_pts[final_count++] = (ExtremePoint){px, py, min_z};
    }

    for (size_t i = 0; i < final_count; ++i) {
        ExtremePoint p = final_pts[i];
        if (p.x > container->width || p.y > container->height || p.z > container->depth) continue;
        if (p.x < 0.0 || p.y < 0.0 || p.z < 0.0) continue;

        int already_in_out = 0;
        for (size_t j = 0; j < out->len; ++j) {
            if (extreme_point_eq(out->data[j], p)) { already_in_out = 1; break; }
        }
        if (!already_in_out) ep_list_push(out, p);
    }
}

void get_valid_ep(const Container* container, const Item* item,
                   const ExtremePoint* eps, size_t eps_count, ExtremePointList* out) {
    const ItemArray* existing = &container->items;
    double cw = container->width, ch = container->height, cd = container->depth;

    for (size_t e = 0; e < eps_count; ++e) {
        double ax = eps[e].x, ay = eps[e].y, az = eps[e].z;
        double adim[3]; item_get_dimension(item, adim);
        double aw = adim[0], ah = adim[1], ad = adim[2];
        int ast = (int)item->shape_type;

        if (!(ax >= -0.001 && ax + aw <= cw + 0.001 &&
              ay >= -0.001 && ay + ah <= ch + 0.001 &&
              az >= -0.001 && az + ad <= cd + 0.001)) {
            continue;
        }

        int collision = 0;
        for (size_t k = 0; k < existing->len; ++k) {
            const Item* o = &existing->data[k];
            double ox = o->position.x, oy = o->position.y, oz = o->position.z;
            double odim[3]; item_get_dimension(o, odim);
            double ow = odim[0], oh = odim[1], od = odim[2];
            int ost = (int)o->shape_type;

            int aabb_overlap = !(ax >= ox + ow - 0.001 || ax + aw <= ox + 0.001 ||
                                   ay >= oy + oh - 0.001 || ay + ah <= oy + 0.001 ||
                                   az >= oz + od - 0.001 || az + ad <= oz + 0.001);
            if (aabb_overlap) {
                if (ast == 0 && ost == 0) { collision = 1; break; }
                if (ast == 1 && ost == 1) { /* Sphere-Sphere */
                    double r1 = aw / 2.0, r2 = ow / 2.0;
                    double dist_sq = pow(ax + r1 - (ox + r2), 2) + pow(ay + r1 - (oy + r2), 2) +
                                      pow(az + r1 - (oz + r2), 2);
                    if (dist_sq < pow(r1 + r2, 2) - 0.001) { collision = 1; break; }
                } else {
                    collision = 1; break; /* Conservative, matches ep.py */
                }
            }
        }

        if (!collision) ep_list_push(out, eps[e]);
    }
}
