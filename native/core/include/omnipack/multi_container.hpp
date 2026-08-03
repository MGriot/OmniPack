#pragma once
// Direct C++ port of src/omnipack/core/multi_container.py

#include <string>
#include <unordered_set>
#include <vector>

#include "omnipack/engine.hpp"
#include "omnipack/models.hpp"

namespace omnipack {

class MultiContainerEngine {
public:
    explicit MultiContainerEngine(Container base_container,
                                   PackingVersus versus = PackingVersus::LONGITUDINAL)
        : base_container_(std::move(base_container)), versus_(versus) {}

    std::vector<Container> pack_all(std::vector<Item> items, const std::string& mode = "level2",
                                     double stability_factor = 1.0, int grasp_k = 1,
                                     double random_disturbance = 0.0) {
        std::vector<Item> remaining = std::move(items);
        std::vector<Container> packed_containers;
        int container_count = 0;

        while (!remaining.empty()) {
            ++container_count;
            Container current(base_container_.id + "_" + std::to_string(container_count),
                               base_container_.width, base_container_.height, base_container_.depth,
                               base_container_.max_weight, base_container_.shape_type);

            std::vector<Item> unpacked;
            if (mode == "level1") {
                Level1Engine engine(current, stability_factor, versus_);
                unpacked = engine.pack(remaining);
            } else {
                Level2Engine engine(current, stability_factor, versus_, random_disturbance);
                unpacked = engine.pack(remaining, grasp_k);
            }

            if (current.items.empty()) break;
            packed_containers.push_back(current);

            std::unordered_set<std::string> packed_ids;
            for (const auto& it : current.items) packed_ids.insert(it.id);
            std::vector<Item> next_remaining;
            for (auto& it : remaining) {
                if (packed_ids.find(it.id) == packed_ids.end()) next_remaining.push_back(it);
            }
            remaining = std::move(next_remaining);
        }

        return packed_containers;
    }

private:
    Container base_container_;
    PackingVersus versus_;
};

} // namespace omnipack
