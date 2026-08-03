#pragma once
// Direct C++ port of src/omnipack/core/engine.py (Level1Engine) and
// src/omnipack/core/engine_v2.py (Level2Engine).

#include <algorithm>
#include <array>
#include <optional>
#include <random>
#include <tuple>
#include <vector>

#include "omnipack/collision.hpp"
#include "omnipack/extreme_points.hpp"
#include "omnipack/models.hpp"

namespace omnipack {

// Simple Best-Fit Decreasing, single-rotation-at-a-time greedy placement.
// No JIT/threading needed - suitable as a low-power/mobile fallback mode.
class Level1Engine {
public:
    explicit Level1Engine(Container& container, double stability_factor = 1.0,
                           PackingVersus versus = PackingVersus::LONGITUDINAL)
        : container_(container), stability_factor_(stability_factor), versus_(versus) {
        extreme_points_.insert(ExtremePoint(0, 0, 0));
    }

    std::vector<Item> pack(std::vector<Item> items) {
        std::sort(items.begin(), items.end(),
                  [](const Item& a, const Item& b) { return a.volume() > b.volume(); });

        std::vector<Item> unpacked;

        for (Item item : items) {
            std::optional<ExtremePoint> best_ep;
            std::optional<Rotation> best_rot;

            for (const auto& ep : extreme_points_) {
                for (Rotation rot : item.allowed_rotations) {
                    item.rotation = rot;
                    auto dim = item.get_dimension();
                    double w = dim[0], h = dim[1], d = dim[2];

                    if (ep.x + w <= container_.width && ep.y + h <= container_.height &&
                        ep.z + d <= container_.depth) {
                        bool collision = false;
                        for (const auto& other : container_.items) {
                            auto odim = other.get_dimension();
                            double ow = odim[0], oh = odim[1], od = odim[2];
                            double ox = other.position.x, oy = other.position.y, oz = other.position.z;
                            if (!(ep.x + w <= ox || ep.x >= ox + ow ||
                                  ep.y + h <= oy || ep.y >= oy + oh ||
                                  ep.z + d <= oz || ep.z >= oz + od)) {
                                collision = true; break;
                            }
                        }
                        if (!collision) { best_ep = ep; best_rot = rot; break; }
                    }
                }
                if (best_ep) break;
            }

            if (best_ep) {
                item.position = best_ep->to_vec3();
                item.rotation = *best_rot;
                container_.items.push_back(item);
                extreme_points_.erase(*best_ep);
                for (const auto& nep : generate_extreme_points(container_, item)) extreme_points_.insert(nep);
            } else {
                unpacked.push_back(item);
            }
        }
        return unpacked;
    }

private:
    Container& container_;
    double stability_factor_;
    PackingVersus versus_;
    ExtremePointSet extreme_points_;
};

// Extreme-point + GRASP heuristic packer (matches Level2Engine).
class Level2Engine {
public:
    explicit Level2Engine(Container& container, double stability_factor = 1.0,
                           PackingVersus versus = PackingVersus::LONGITUDINAL,
                           double random_disturbance = 0.0)
        : container_(container), stability_factor_(stability_factor), versus_(versus),
          random_disturbance_(random_disturbance), rng_(std::random_device{}()) {
        extreme_points_.insert(ExtremePoint(0, 0, 0));
        extreme_points_.insert(ExtremePoint(0, 0, container.depth));
    }

    // Test-only hook mirroring direct access to the Python engine's `extreme_points` set.
    void seed_extreme_point(double x, double y, double z) { extreme_points_.insert(ExtremePoint(x, y, z)); }

    std::vector<Item> pack(std::vector<Item> items, int grasp_k = 1) {
        std::uniform_real_distribution<double> unit(0.0, 1.0);

        std::vector<std::tuple<int, int, double, size_t>> keyed; // (strategy_rank, -stop_id, -(vol+jitter), orig_idx)
        keyed.reserve(items.size());
        for (size_t i = 0; i < items.size(); ++i) {
            int rank = strategy_rank(items[i].strategy);
            double vol_disturb = (unit(rng_) - 0.5) * random_disturbance_ * items[i].volume();
            keyed.emplace_back(rank, -items[i].stop_id, -(items[i].volume() + vol_disturb), i);
        }
        std::sort(keyed.begin(), keyed.end(), [](const auto& a, const auto& b) {
            if (std::get<0>(a) != std::get<0>(b)) return std::get<0>(a) < std::get<0>(b);
            if (std::get<1>(a) != std::get<1>(b)) return std::get<1>(a) < std::get<1>(b);
            return std::get<2>(a) < std::get<2>(b);
        });

        std::vector<Item> unpacked;

        for (const auto& k : keyed) {
            Item item = items[std::get<3>(k)];
            place_one(item, grasp_k, unit);
            if (!last_placed_) unpacked.push_back(item);
        }
        return unpacked;
    }

private:
    static int strategy_rank(LoadingStrategy s) {
        switch (s) {
            case LoadingStrategy::LIFO: return 0;
            case LoadingStrategy::FIFO: return 1;
            default: return 2;
        }
    }

    void place_one(Item item, int grasp_k, std::uniform_real_distribution<double>& unit) {
        last_placed_ = false;

        std::vector<ExtremePoint> eps_list(extreme_points_.begin(), extreme_points_.end());
        std::vector<Vec3> eps_vec(eps_list.size());
        for (size_t i = 0; i < eps_list.size(); ++i) eps_vec[i] = eps_list[i].to_vec3();

        std::array<RotDim, 6> rots_dims;
        Rotation orig_rot = item.rotation;
        for (int r = 0; r < 6; ++r) {
            item.rotation = static_cast<Rotation>(r);
            auto dim = item.get_dimension();
            rots_dims[r] = {dim[0], dim[1], dim[2], static_cast<int>(item.shape_type)};
        }
        item.rotation = orig_rot;

        std::vector<ExistingItem> existing(container_.items.size());
        for (size_t idx = 0; idx < container_.items.size(); ++idx) {
            const auto& ex = container_.items[idx];
            auto dim = ex.get_dimension();
            existing[idx] = {ex.position, dim[0], dim[1], dim[2],
                              static_cast<int>(ex.shape_type), ex.weight, ex.max_stack_weight};
        }

        std::vector<RotDim> rots_vec(rots_dims.begin(), rots_dims.end());
        std::vector<uint8_t> valid_mask = evaluate_positions(
            eps_vec, rots_vec, existing, container_.width, container_.height, container_.depth,
            static_cast<int>(container_.shape_type), item.weight, static_cast<int>(item.strategy));

        struct Candidate { size_t ep_idx; int rot_idx; std::array<double, 4> score; std::optional<double> target_z; };
        std::vector<Candidate> candidates;

        auto allowed = [&](Rotation r) {
            return std::find(item.allowed_rotations.begin(), item.allowed_rotations.end(), r) !=
                   item.allowed_rotations.end();
        };

        for (size_t i = 0; i < eps_list.size(); ++i) {
            for (int j = 0; j < 6; ++j) {
                Rotation rot = static_cast<Rotation>(j);
                if (!allowed(rot)) continue;

                if (valid_mask[i * 6 + j]) {
                    const auto& ep = eps_list[i];
                    item.rotation = rot;
                    auto dim = item.get_dimension();
                    double bw = dim[0], bd = dim[2];
                    double z_val = (item.strategy == LoadingStrategy::LIFO)
                                       ? (container_.depth - (ep.z + bd)) : ep.z;
                    double balance_penalty = std::abs(ep.x + bw / 2.0 - container_.width / 2.0) / container_.width;

                    std::array<double, 4> score{};
                    if (versus_ == PackingVersus::LATERAL) score = {z_val, ep.y, ep.x, balance_penalty};
                    else if (versus_ == PackingVersus::FLOOR_FIRST) score = {ep.y, z_val, ep.x, balance_penalty};
                    else if (versus_ == PackingVersus::WALL_BUILDING) score = {ep.x, z_val, ep.y, 0.0};
                    else score = {ep.x, ep.y, z_val, balance_penalty};

                    if (random_disturbance_ > 0.0) {
                        double jitter = (unit(rng_) - 0.5) * random_disturbance_;
                        for (auto& s : score) s += jitter;
                    }
                    candidates.push_back({i, j, score, std::nullopt});
                }

                if (item.strategy == LoadingStrategy::LIFO) {
                    const auto& ep = eps_list[i];
                    item.rotation = rot;
                    auto dim = item.get_dimension();
                    double bw = dim[0], bd = dim[2];
                    double target_z = ep.z - bd;
                    if (target_z >= -0.001) {
                        std::vector<Vec3> temp_ep = {{ep.x, ep.y, target_z}};
                        std::vector<RotDim> single_rot = {rots_dims[j]};
                        auto back_mask = evaluate_positions(temp_ep, single_rot, existing,
                                                             container_.width, container_.height, container_.depth,
                                                             static_cast<int>(container_.shape_type), item.weight,
                                                             static_cast<int>(item.strategy));
                        if (back_mask[0]) {
                            double z_val = container_.depth - (target_z + bd);
                            double balance_penalty = std::abs(ep.x + bw / 2.0 - container_.width / 2.0) / container_.width;

                            std::array<double, 4> score{};
                            if (versus_ == PackingVersus::LATERAL) score = {z_val, ep.y, ep.x, balance_penalty};
                            else if (versus_ == PackingVersus::FLOOR_FIRST) score = {ep.y, z_val, ep.x, balance_penalty};
                            else if (versus_ == PackingVersus::WALL_BUILDING) score = {ep.x, z_val, ep.y, 0.0};
                            else score = {z_val, ep.y, ep.x, balance_penalty};

                            if (random_disturbance_ > 0.0) {
                                double jitter = (unit(rng_) - 0.5) * random_disturbance_;
                                for (auto& s : score) s += jitter;
                            }
                            candidates.push_back({i, j, score, target_z});
                        }
                    }
                }
            }
        }

        if (candidates.empty()) return;

        std::sort(candidates.begin(), candidates.end(),
                  [](const Candidate& a, const Candidate& b) { return a.score < b.score; });
        size_t actual_k = std::min(static_cast<size_t>(std::max(grasp_k, 1)), candidates.size());
        size_t chosen_idx = 0;
        if (actual_k > 1) {
            std::uniform_int_distribution<size_t> pick(0, actual_k - 1);
            chosen_idx = pick(rng_);
        }
        const Candidate& chosen = candidates[chosen_idx];
        const auto& ep = eps_list[chosen.ep_idx];

        Item placed = item;
        placed.position = chosen.target_z ? Vec3{ep.x, ep.y, *chosen.target_z} : ep.to_vec3();
        placed.rotation = static_cast<Rotation>(chosen.rot_idx);
        container_.items.push_back(placed);
        extreme_points_.erase(ep);
        for (const auto& nep : generate_extreme_points(container_, placed)) extreme_points_.insert(nep);
        last_placed_ = true;
    }

    Container& container_;
    double stability_factor_;
    PackingVersus versus_;
    double random_disturbance_;
    std::mt19937 rng_;
    ExtremePointSet extreme_points_;
    bool last_placed_ = false;
};

} // namespace omnipack
