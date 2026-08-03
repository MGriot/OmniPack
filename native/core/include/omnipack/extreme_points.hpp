#pragma once
// Direct C++ port of src/omnipack/core/ep.py

#include <algorithm>
#include <cmath>
#include <unordered_set>
#include <vector>

#include "omnipack/models.hpp"

namespace omnipack {

struct ExtremePoint {
    double x = 0.0, y = 0.0, z = 0.0;

    ExtremePoint() = default;
    ExtremePoint(double x_, double y_, double z_) : x(x_), y(y_), z(z_) {}

    Vec3 to_vec3() const { return {x, y, z}; }

    bool operator==(const ExtremePoint& other) const {
        return std::abs(x - other.x) < 0.001 &&
               std::abs(y - other.y) < 0.001 &&
               std::abs(z - other.z) < 0.001;
    }
};

struct ExtremePointHash {
    size_t operator()(const ExtremePoint& p) const {
        // Matches ep.py: hash((round(x,3), round(y,3), round(z,3)))
        auto rnd = [](double v) { return std::llround(v * 1000.0); };
        size_t h1 = std::hash<long long>{}(rnd(p.x));
        size_t h2 = std::hash<long long>{}(rnd(p.y));
        size_t h3 = std::hash<long long>{}(rnd(p.z));
        return h1 ^ (h2 * 0x9e3779b97f4a7c15ULL) ^ (h3 * 0xc2b2ae3d27d4eb4fULL);
    }
};

using ExtremePointSet = std::unordered_set<ExtremePoint, ExtremePointHash>;

inline std::vector<ExtremePoint> generate_extreme_points(const Container& container, const Item& item) {
    const double ix = item.position.x, iy = item.position.y, iz = item.position.z;
    const auto idim = item.get_dimension();
    const double iw = idim[0], ih = idim[1], id = idim[2];
    const auto& existing = container.items;

    struct Pt { double x, y, z; };
    std::vector<Pt> potential_pts = {
        {ix, iy, iz + id},      // Z first
        {ix, iy + ih, iz},      // Y second
        {ix + iw, iy, iz},      // X third
        {ix, iy, iz},           // LIFO / backward point
    };

    std::vector<ExtremePoint> final_pts;

    for (const auto& p : potential_pts) {
        const double px = p.x, py = p.y, pz = p.z;

        double max_x = 0.0;
        for (const auto& other : existing) {
            const auto od = other.get_dimension();
            const double ox = other.position.x, oy = other.position.y, oz = other.position.z;
            const double ow = od[0], oh = od[1], odd = od[2];
            if (ox + ow <= px + 0.001 && (oy < py + 0.001 && oy + oh > py - 0.001) &&
                (oz < pz + 0.001 && oz + odd > pz - 0.001)) {
                max_x = std::max(max_x, ox + ow);
            }
        }

        double max_y = 0.0;
        for (const auto& other : existing) {
            const auto od = other.get_dimension();
            const double ox = other.position.x, oy = other.position.y, oz = other.position.z;
            const double ow = od[0], oh = od[1], odd = od[2];
            if (oy + oh <= py + 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oz < pz + 0.001 && oz + odd > pz - 0.001)) {
                max_y = std::max(max_y, oy + oh);
            }
        }

        double max_z = 0.0;
        for (const auto& other : existing) {
            const auto od = other.get_dimension();
            const double ox = other.position.x, oy = other.position.y, oz = other.position.z;
            const double ow = od[0], oh = od[1], odd = od[2];
            if (oz + odd <= pz + 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oy < py + 0.001 && oy + oh > py - 0.001)) {
                max_z = std::max(max_z, oz + odd);
            }
        }

        double min_z = container.depth;
        for (const auto& other : existing) {
            const auto od = other.get_dimension();
            const double ox = other.position.x, oy = other.position.y, oz = other.position.z;
            const double ow = od[0], oh = od[1];
            if (oz >= pz - 0.001 && (ox < px + 0.001 && ox + ow > px - 0.001) &&
                (oy < py + 0.001 && oy + oh > py - 0.001)) {
                min_z = std::min(min_z, oz);
            }
        }

        final_pts.push_back({px, py, pz});
        if (px > max_x) final_pts.push_back({max_x, py, pz});
        if (py > max_y) final_pts.push_back({px, max_y, pz});
        if (pz > max_z) final_pts.push_back({px, py, max_z});
        if (pz < min_z) final_pts.push_back({px, py, min_z});
    }

    std::vector<ExtremePoint> valid_pts;
    ExtremePointSet seen;
    for (const auto& p : final_pts) {
        if (p.x > container.width || p.y > container.height || p.z > container.depth) continue;
        if (p.x < 0.0 || p.y < 0.0 || p.z < 0.0) continue;
        if (seen.find(p) == seen.end()) {
            valid_pts.push_back(p);
            seen.insert(p);
        }
    }
    return valid_pts;
}

// Pure-Python-fallback-equivalent validity check (naive box collision, no numba/threads).
inline std::vector<ExtremePoint> get_valid_ep(const Container& container, const Item& item,
                                               const ExtremePointSet& eps) {
    std::vector<ExtremePoint> valid_eps;
    const auto& existing = container.items;
    const double cw = container.width, ch = container.height, cd = container.depth;

    for (const auto& ep : eps) {
        const double ax = ep.x, ay = ep.y, az = ep.z;
        const auto adim = item.get_dimension();
        const double aw = adim[0], ah = adim[1], ad = adim[2];
        const int ast = static_cast<int>(item.shape_type);

        if (!(ax >= -0.001 && ax + aw <= cw + 0.001 &&
              ay >= -0.001 && ay + ah <= ch + 0.001 &&
              az >= -0.001 && az + ad <= cd + 0.001)) {
            continue;
        }

        bool collision = false;
        for (const auto& other : existing) {
            const double ox = other.position.x, oy = other.position.y, oz = other.position.z;
            const auto odim = other.get_dimension();
            const double ow = odim[0], oh = odim[1], od = odim[2];
            const int ost = static_cast<int>(other.shape_type);

            const bool aabb_overlap = !(ax >= ox + ow - 0.001 || ax + aw <= ox + 0.001 ||
                                          ay >= oy + oh - 0.001 || ay + ah <= oy + 0.001 ||
                                          az >= oz + od - 0.001 || az + ad <= oz + 0.001);
            if (aabb_overlap) {
                if (ast == 0 && ost == 0) { collision = true; break; }
                if (ast == 1 && ost == 1) { // Sphere-Sphere
                    double r1 = aw / 2.0, r2 = ow / 2.0;
                    double dist_sq = std::pow(ax + r1 - (ox + r2), 2) +
                                       std::pow(ay + r1 - (oy + r2), 2) +
                                       std::pow(az + r1 - (oz + r2), 2);
                    if (dist_sq < std::pow(r1 + r2, 2) - 0.001) { collision = true; break; }
                } else {
                    collision = true; break; // Conservative (matches ep.py)
                }
            }
        }

        if (!collision) valid_eps.push_back(ep);
    }
    return valid_eps;
}

} // namespace omnipack
