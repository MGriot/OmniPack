#pragma once
// Direct C++ port of src/omnipack/core/models.py

#include <algorithm>
#include <array>
#include <cmath>
#include <optional>
#include <string>
#include <unordered_set>
#include <vector>

namespace omnipack {

enum class Rotation { W_H_D = 0, H_W_D = 1, H_D_W = 2, D_H_W = 3, D_W_H = 4, W_D_H = 5 };
enum class LoadingStrategy { NONE = 0, FIFO = 1, LIFO = 2 };
enum class PackingVersus { LONGITUDINAL = 0, LATERAL = 1, FLOOR_FIRST = 2, WALL_BUILDING = 3, CORNER_FIRST = 4 };
enum class ShapeType { BOX = 0, SPHERE = 1, CYLINDER = 2, TETRAHEDRON = 3 };

inline std::vector<Rotation> all_rotations() {
    return {Rotation::W_H_D, Rotation::H_W_D, Rotation::H_D_W,
            Rotation::D_H_W, Rotation::D_W_H, Rotation::W_D_H};
}

struct Vec3 {
    double x = 0.0, y = 0.0, z = 0.0;
};

struct Item {
    std::string id;
    double width = 0.0, height = 0.0, depth = 0.0;
    double weight = 0.0;
    double max_stack_weight = 1'000'000.0;
    std::optional<std::string> group_id;
    LoadingStrategy strategy = LoadingStrategy::NONE;
    int stop_id = 0;
    std::vector<Rotation> allowed_rotations = all_rotations();
    ShapeType shape_type = ShapeType::BOX;

    // Internal state, set by the engine while packing.
    Vec3 position{0.0, 0.0, 0.0};
    Rotation rotation = Rotation::W_H_D;

    Item() = default;
    Item(std::string id_, double w, double h, double d,
         double weight_ = 0.0, double max_stack_weight_ = 1'000'000.0)
        : id(std::move(id_)), width(w), height(h), depth(d),
          weight(weight_), max_stack_weight(max_stack_weight_) {}

    double volume() const { return width * height * depth; }

    std::array<double, 3> get_dimension() const {
        const double w = width, h = height, d = depth;
        switch (rotation) {
            case Rotation::W_H_D: return {w, h, d};
            case Rotation::H_W_D: return {h, w, d};
            case Rotation::H_D_W: return {h, d, w};
            case Rotation::D_H_W: return {d, h, w};
            case Rotation::D_W_H: return {d, w, h};
            case Rotation::W_D_H: return {w, d, h};
        }
        return {w, h, d};
    }
};

struct ContainerStats {
    double volume_utilization = 0.0;
    double weight_utilization = 0.0;
    double total_weight = 0.0;
    Vec3 center_of_mass{0.0, 0.0, 0.0};
    double stability_score = 100.0; // Placeholder, matches models.py
    int item_count = 0;
};

struct Container {
    std::string id;
    double width = 0.0, height = 0.0, depth = 0.0;
    double max_weight = 1'000'000.0;
    ShapeType shape_type = ShapeType::BOX;
    std::vector<Item> items;

    Container() = default;
    Container(std::string id_, double w, double h, double d,
               double max_weight_ = 1'000'000.0, ShapeType shape = ShapeType::BOX)
        : id(std::move(id_)), width(w), height(h), depth(d),
          max_weight(max_weight_), shape_type(shape) {}

    double volume() const { return width * height * depth; }

    double remaining_volume() const {
        double used = 0.0;
        for (const auto& it : items) used += it.volume();
        return volume() - used;
    }

    double volume_utilization() const {
        if (items.empty()) return 0.0;
        double used = 0.0;
        for (const auto& it : items) used += it.volume();
        return (used / volume()) * 100.0;
    }

    ContainerStats calculate_stats() const {
        ContainerStats stats;
        stats.item_count = static_cast<int>(items.size());
        double used_vol = 0.0, total_weight = 0.0;
        for (const auto& it : items) { used_vol += it.volume(); total_weight += it.weight; }

        double com_x = 0.0, com_y = 0.0, com_z = 0.0;
        if (total_weight > 0.0) {
            for (const auto& it : items) {
                auto dim = it.get_dimension();
                com_x += (it.position.x + dim[0] / 2.0) * it.weight;
                com_y += (it.position.y + dim[1] / 2.0) * it.weight;
                com_z += (it.position.z + dim[2] / 2.0) * it.weight;
            }
            com_x /= total_weight; com_y /= total_weight; com_z /= total_weight;
        }

        stats.volume_utilization = volume() > 0.0 ? (used_vol / volume()) * 100.0 : 0.0;
        stats.weight_utilization = max_weight > 0.0 ? (total_weight / max_weight) * 100.0 : 0.0;
        stats.total_weight = total_weight;
        stats.center_of_mass = {com_x, com_y, com_z};
        return stats;
    }

    // Stop-aware accessibility: an item is blocked if another item sitting
    // closer to the door (higher Z) needs a LATER stop (higher stop_id) and
    // overlaps it in X-Y - i.e. it must be moved before this item can come out.
    // Score is the percentage of items that are NOT blocked.
    double calculate_accessibility() const {
        if (items.empty()) return 100.0;

        std::vector<const Item*> sorted_by_z;
        sorted_by_z.reserve(items.size());
        for (const auto& it : items) sorted_by_z.push_back(&it);
        std::sort(sorted_by_z.begin(), sorted_by_z.end(),
                  [](const Item* a, const Item* b) { return a->position.z > b->position.z; });

        std::unordered_set<const Item*> blocked;
        for (size_t i = 0; i < sorted_by_z.size(); ++i) {
            const Item* item = sorted_by_z[i];
            for (size_t j = i + 1; j < sorted_by_z.size(); ++j) {
                const Item* other = sorted_by_z[j];
                if (other->stop_id < item->stop_id) {
                    auto idim = item->get_dimension();
                    auto odim = other->get_dimension();
                    bool overlap_x = item->position.x < other->position.x + odim[0] &&
                                      item->position.x + idim[0] > other->position.x;
                    bool overlap_y = item->position.y < other->position.y + odim[1] &&
                                      item->position.y + idim[1] > other->position.y;
                    if (overlap_x && overlap_y) blocked.insert(other);
                }
            }
        }

        return (static_cast<double>(items.size() - blocked.size()) / static_cast<double>(items.size())) * 100.0;
    }
};

} // namespace omnipack
