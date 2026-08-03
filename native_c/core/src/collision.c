#include "omnipack/collision.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

double get_overlap_area(double ax1, double ay1, double ax2, double ay2,
                         double bx1, double by1, double bx2, double by2) {
    double ix1 = ax1 > bx1 ? ax1 : bx1;
    double iy1 = ay1 > by1 ? ay1 : by1;
    double ix2 = ax2 < bx2 ? ax2 : bx2;
    double iy2 = ay2 < by2 ? ay2 : by2;
    if (ix1 < ix2 && iy1 < iy2) return (ix2 - ix1) * (iy2 - iy1);
    return 0.0;
}

typedef struct { double z; size_t idx; } ZIndex;

static int cmp_zindex_desc(const void* a, const void* b) {
    const ZIndex* za = (const ZIndex*)a;
    const ZIndex* zb = (const ZIndex*)b;
    if (za->z > zb->z) return -1;
    if (za->z < zb->z) return 1;
    return 0;
}

void calculate_cumulative_loads(const ExistingItem* existing, size_t n, double* out_loads) {
    for (size_t i = 0; i < n; ++i) out_loads[i] = existing[i].weight;
    if (n == 0) return;

    ZIndex* order = (ZIndex*)malloc(n * sizeof(ZIndex));
    for (size_t i = 0; i < n; ++i) { order[i].z = existing[i].pos.z; order[i].idx = i; }
    qsort(order, n, sizeof(ZIndex), cmp_zindex_desc);

    size_t* supports = (size_t*)malloc(n * sizeof(size_t));
    double* areas = (double*)malloc(n * sizeof(double));

    for (size_t i = 0; i < n; ++i) {
        size_t idx_j = order[i].idx;
        double jx = existing[idx_j].pos.x, jy = existing[idx_j].pos.y, jz = existing[idx_j].pos.z;
        double jw = existing[idx_j].w, jh = existing[idx_j].h;

        size_t support_count = 0;
        double total_contact = 0.0;

        for (size_t k = 0; k < n; ++k) {
            if (k == idx_j) continue;
            double kx = existing[k].pos.x, ky = existing[k].pos.y, kz = existing[k].pos.z;
            double kw = existing[k].w, kh = existing[k].h, kd = existing[k].d;
            if (fabs((kz + kd) - jz) < 0.001) {
                double area = get_overlap_area(jx, jy, jx + jw, jy + jh, kx, ky, kx + kw, ky + kh);
                if (area > 0.0) {
                    supports[support_count] = k;
                    areas[support_count] = area;
                    support_count++;
                    total_contact += area;
                }
            }
        }

        if (total_contact > 0.0) {
            for (size_t m = 0; m < support_count; ++m) {
                size_t idx_k = supports[m];
                double ratio = areas[m] / total_contact;
                out_loads[idx_k] += out_loads[idx_j] * ratio;
            }
        }
    }

    free(order);
    free(supports);
    free(areas);
}

int check_collision(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                     double ox, double oy, double oz, double ow, double oh, double od, int ost) {
    if (ax >= ox + ow - 0.001 || ax + aw <= ox + 0.001 ||
        ay >= oy + oh - 0.001 || ay + ah <= oy + 0.001 ||
        az >= oz + od - 0.001 || az + ad <= oz + 0.001) {
        return 0;
    }

    if (ast == 1 && ost == 1) { /* Sphere-Sphere */
        double r1 = aw / 2.0, r2 = ow / 2.0;
        double dist_sq = pow(ax + r1 - (ox + r2), 2) + pow(ay + r1 - (oy + r2), 2) +
                          pow(az + r1 - (oz + r2), 2);
        return dist_sq < pow(r1 + r2, 2) - 0.001;
    }
    if (ast == 1 && ost == 0) { /* Sphere-Box */
        double r = aw / 2.0;
        double cx = ax + r, cy = ay + r, cz = az + r;
        double clx = fmax(ox, fmin(cx, ox + ow));
        double cly = fmax(oy, fmin(cy, oy + oh));
        double clz = fmax(oz, fmin(cz, oz + od));
        double dist_sq = pow(cx - clx, 2) + pow(cy - cly, 2) + pow(cz - clz, 2);
        return dist_sq < r * r - 0.001;
    }
    if (ast == 0 && ost == 1) { /* Box-Sphere */
        double r = ow / 2.0;
        double cx = ox + r, cy = oy + r, cz = oz + r;
        double clx = fmax(ax, fmin(cx, ax + aw));
        double cly = fmax(ay, fmin(cy, ay + ah));
        double clz = fmax(az, fmin(cz, az + ad));
        double dist_sq = pow(cx - clx, 2) + pow(cy - cly, 2) + pow(cz - clz, 2);
        return dist_sq < r * r - 0.001;
    }
    return 1;
}

int is_in_container(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                     double cw, double ch, double cd, int cst) {
    if (ax >= -0.001 && ax + aw <= cw + 0.001 &&
        ay >= -0.001 && ay + ah <= ch + 0.001 &&
        az >= -0.001 && az + ad <= cd + 0.001) {
        if (ast == 1 && cst == 1) { /* Item sphere in container sphere */
            double r_item = aw / 2.0, r_cont = cw / 2.0;
            double dist = sqrt(pow(ax + r_item - r_cont, 2) + pow(ay + r_item - r_cont, 2) +
                                 pow(az + r_item - r_cont, 2));
            return dist + r_item <= r_cont + 0.001;
        }
        return 1;
    }
    return 0;
}

void evaluate_positions(const Vec3* eps, size_t num_eps,
                         const RotDim* item_dims, size_t num_rots,
                         const ExistingItem* existing, size_t n_existing,
                         double cw, double ch, double cd, int container_shape,
                         double item_weight, int strategy,
                         unsigned char* results_out) {
    memset(results_out, 0, num_eps * num_rots);

    double* current_loads = (double*)malloc((n_existing > 0 ? n_existing : 1) * sizeof(double));
    calculate_cumulative_loads(existing, n_existing, current_loads);

    size_t* support_indices = (size_t*)malloc((n_existing > 0 ? n_existing : 1) * sizeof(size_t));
    double* support_areas = (double*)malloc((n_existing > 0 ? n_existing : 1) * sizeof(double));

    for (size_t i = 0; i < num_eps; ++i) {
        double ax = eps[i].x, ay = eps[i].y, az = eps[i].z;
        for (size_t j = 0; j < num_rots; ++j) {
            double aw = item_dims[j].w, ah = item_dims[j].h, ad = item_dims[j].d;
            int ast = item_dims[j].shape;

            if (!is_in_container(ax, ay, az, aw, ah, ad, ast, cw, ch, cd, container_shape)) continue;

            int collision = 0;
            for (size_t k = 0; k < n_existing; ++k) {
                if (check_collision(ax, ay, az, aw, ah, ad, ast,
                                     existing[k].pos.x, existing[k].pos.y, existing[k].pos.z,
                                     existing[k].w, existing[k].h, existing[k].d, existing[k].shape)) {
                    collision = 1; break;
                }
            }
            if (collision) continue;

            int valid = 0;
            if (az < 0.001) {
                valid = 1;
            } else if (strategy == 2 && (az + ad >= cd - 0.001)) { /* LIFO door support */
                valid = 1;
            }

            if (!valid) {
                double total_contact_area = 0.0;
                size_t support_count = 0;
                for (size_t k = 0; k < n_existing; ++k) {
                    if (fabs(az - (existing[k].pos.z + existing[k].d)) < 0.001) {
                        double area = get_overlap_area(ax, ay, ax + aw, ay + ah,
                                                        existing[k].pos.x, existing[k].pos.y,
                                                        existing[k].pos.x + existing[k].w,
                                                        existing[k].pos.y + existing[k].h);
                        if (area > 0.0) {
                            total_contact_area += area;
                            support_indices[support_count] = k;
                            support_areas[support_count] = area;
                            support_count++;
                        }
                    }
                }
                if (total_contact_area > (aw * ah * 0.5)) {
                    int can_sustain = 1;
                    for (size_t m = 0; m < support_count; ++m) {
                        size_t idx_k = support_indices[m];
                        double added_load = item_weight * (support_areas[m] / total_contact_area);
                        if ((current_loads[idx_k] - existing[idx_k].weight + added_load) >
                            existing[idx_k].max_stack_weight + 0.001) {
                            can_sustain = 0; break;
                        }
                    }
                    if (can_sustain) valid = 1;
                }
            }

            results_out[i * num_rots + j] = valid ? 1 : 0;
        }
    }

    free(current_loads);
    free(support_indices);
    free(support_areas);
}
