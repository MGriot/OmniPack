#pragma once
// Direct C++ port of src/omnipack/core/accelerated.py.
// No JIT needed: native loops already outperform the Python/Numba baseline,
// this is a straight translation, not an optimization pass.

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <vector>

#include "omnipack/models.hpp"

namespace omnipack {

inline double get_overlap_area(double ax1, double ay1, double ax2, double ay2,
                                double bx1, double by1, double bx2, double by2) {
    double ix1 = std::max(ax1, bx1);
    double iy1 = std::max(ay1, by1);
    double ix2 = std::min(ax2, bx2);
    double iy2 = std::min(ay2, by2);
    if (ix1 < ix2 && iy1 < iy2) return (ix2 - ix1) * (iy2 - iy1);
    return 0.0;
}

// dims: {w, h, d} per existing item (no shape column needed here).
inline std::vector<double> calculate_cumulative_loads(const std::vector<Vec3>& pos,
                                                        const std::vector<std::array<double, 3>>& dims,
                                                        const std::vector<double>& weights) {
    size_t n = pos.size();
    std::vector<double> loads = weights;
    if (n == 0) return loads;

    std::vector<size_t> indices(n);
    for (size_t i = 0; i < n; ++i) indices[i] = i;
    std::sort(indices.begin(), indices.end(),
              [&](size_t a, size_t b) { return pos[a].z > pos[b].z; });

    for (size_t i = 0; i < n; ++i) {
        size_t idx_j = indices[i];
        double jx = pos[idx_j].x, jy = pos[idx_j].y, jz = pos[idx_j].z;
        double jw = dims[idx_j][0], jh = dims[idx_j][1];

        std::vector<size_t> supports;
        std::vector<double> areas;
        double total_contact = 0.0;

        for (size_t k = 0; k < n; ++k) {
            if (k == idx_j) continue;
            double kx = pos[k].x, ky = pos[k].y, kz = pos[k].z;
            double kw = dims[k][0], kh = dims[k][1], kd = dims[k][2];
            if (std::abs((kz + kd) - jz) < 0.001) {
                double area = get_overlap_area(jx, jy, jx + jw, jy + jh, kx, ky, kx + kw, ky + kh);
                if (area > 0.0) {
                    supports.push_back(k);
                    areas.push_back(area);
                    total_contact += area;
                }
            }
        }

        if (total_contact > 0.0) {
            for (size_t m = 0; m < supports.size(); ++m) {
                size_t idx_k = supports[m];
                double ratio = areas[m] / total_contact;
                loads[idx_k] += loads[idx_j] * ratio;
            }
        }
    }
    return loads;
}

inline bool check_collision(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                             double ox, double oy, double oz, double ow, double oh, double od, int ost) {
    // 1. Bounding box check (early exit)
    if (ax >= ox + ow - 0.001 || ax + aw <= ox + 0.001 ||
        ay >= oy + oh - 0.001 || ay + ah <= oy + 0.001 ||
        az >= oz + od - 0.001 || az + ad <= oz + 0.001) {
        return false;
    }

    // 2. Detailed check for spheres
    if (ast == 1 && ost == 1) { // Sphere-Sphere
        double r1 = aw / 2.0, r2 = ow / 2.0;
        double dist_sq = std::pow(ax + r1 - (ox + r2), 2) + std::pow(ay + r1 - (oy + r2), 2) +
                           std::pow(az + r1 - (oz + r2), 2);
        return dist_sq < std::pow(r1 + r2, 2) - 0.001;
    }
    if (ast == 1 && ost == 0) { // Sphere-Box
        double r = aw / 2.0;
        double cx = ax + r, cy = ay + r, cz = az + r;
        double clx = std::max(ox, std::min(cx, ox + ow));
        double cly = std::max(oy, std::min(cy, oy + oh));
        double clz = std::max(oz, std::min(cz, oz + od));
        double dist_sq = std::pow(cx - clx, 2) + std::pow(cy - cly, 2) + std::pow(cz - clz, 2);
        return dist_sq < r * r - 0.001;
    }
    if (ast == 0 && ost == 1) { // Box-Sphere
        double r = ow / 2.0;
        double cx = ox + r, cy = oy + r, cz = oz + r;
        double clx = std::max(ax, std::min(cx, ax + aw));
        double cly = std::max(ay, std::min(cy, ay + ah));
        double clz = std::max(az, std::min(cz, az + ad));
        double dist_sq = std::pow(cx - clx, 2) + std::pow(cy - cly, 2) + std::pow(cz - clz, 2);
        return dist_sq < r * r - 0.001;
    }
    return true;
}

inline bool is_in_container(double ax, double ay, double az, double aw, double ah, double ad, int ast,
                             double cw, double ch, double cd, int cst) {
    if (ax >= -0.001 && ax + aw <= cw + 0.001 &&
        ay >= -0.001 && ay + ah <= ch + 0.001 &&
        az >= -0.001 && az + ad <= cd + 0.001) {
        if (ast == 1 && cst == 1) { // Item sphere in container sphere
            double r_item = aw / 2.0, r_cont = cw / 2.0;
            double dist = std::sqrt(std::pow(ax + r_item - r_cont, 2) +
                                      std::pow(ay + r_item - r_cont, 2) +
                                      std::pow(az + r_item - r_cont, 2));
            return dist + r_item <= r_cont + 0.001;
        }
        return true;
    }
    return false;
}

struct RotDim { double w, h, d; int shape; };
struct ExistingItem { Vec3 pos; double w, h, d; int shape; double weight; double max_stack_weight; };

// Returns a flat num_eps x num_rots grid: valid[i * num_rots + j].
inline std::vector<uint8_t> evaluate_positions(const std::vector<Vec3>& eps,
                                                const std::vector<RotDim>& item_dims,
                                                const std::vector<ExistingItem>& existing,
                                                double cw, double ch, double cd, int container_shape,
                                                double item_weight, int strategy) {
    size_t num_eps = eps.size(), num_rots = item_dims.size(), n_existing = existing.size();
    std::vector<uint8_t> results(num_eps * num_rots, 0);

    std::vector<Vec3> ex_pos(n_existing);
    std::vector<std::array<double, 3>> ex_dim3(n_existing);
    std::vector<double> ex_weights(n_existing);
    for (size_t k = 0; k < n_existing; ++k) {
        ex_pos[k] = existing[k].pos;
        ex_dim3[k] = {existing[k].w, existing[k].h, existing[k].d};
        ex_weights[k] = existing[k].weight;
    }
    std::vector<double> current_loads = calculate_cumulative_loads(ex_pos, ex_dim3, ex_weights);

    for (size_t i = 0; i < num_eps; ++i) {
        double ax = eps[i].x, ay = eps[i].y, az = eps[i].z;
        for (size_t j = 0; j < num_rots; ++j) {
            double aw = item_dims[j].w, ah = item_dims[j].h, ad = item_dims[j].d;
            int ast = item_dims[j].shape;

            if (!is_in_container(ax, ay, az, aw, ah, ad, ast, cw, ch, cd, container_shape)) continue;

            bool collision = false;
            for (size_t k = 0; k < n_existing; ++k) {
                if (check_collision(ax, ay, az, aw, ah, ad, ast,
                                     existing[k].pos.x, existing[k].pos.y, existing[k].pos.z,
                                     existing[k].w, existing[k].h, existing[k].d, existing[k].shape)) {
                    collision = true; break;
                }
            }
            if (collision) continue;

            bool valid = false;
            if (az < 0.001) {
                valid = true;
            } else if (strategy == 2 && (az + ad >= cd - 0.001)) { // LIFO door support
                valid = true;
            }

            if (!valid) {
                double total_contact_area = 0.0;
                std::vector<size_t> support_indices;
                std::vector<double> support_areas;
                for (size_t k = 0; k < n_existing; ++k) {
                    if (std::abs(az - (existing[k].pos.z + existing[k].d)) < 0.001) {
                        double area = get_overlap_area(ax, ay, ax + aw, ay + ah,
                                                        existing[k].pos.x, existing[k].pos.y,
                                                        existing[k].pos.x + existing[k].w,
                                                        existing[k].pos.y + existing[k].h);
                        if (area > 0.0) {
                            total_contact_area += area;
                            support_indices.push_back(k);
                            support_areas.push_back(area);
                        }
                    }
                }
                if (total_contact_area > (aw * ah * 0.5)) {
                    bool can_sustain = true;
                    for (size_t m = 0; m < support_indices.size(); ++m) {
                        size_t idx_k = support_indices[m];
                        double added_load = item_weight * (support_areas[m] / total_contact_area);
                        if ((current_loads[idx_k] - ex_weights[idx_k] + added_load) >
                            existing[idx_k].max_stack_weight + 0.001) {
                            can_sustain = false; break;
                        }
                    }
                    if (can_sustain) valid = true;
                }
            }

            results[i * num_rots + j] = valid ? 1 : 0;
        }
    }
    return results;
}

} // namespace omnipack
