from typing import List, Tuple
from .models import Item, Container, PackingVersus
from .engine import Level1Engine
from .engine_v2 import Level2Engine
from .genetic import GeneticOptimizer
from .mcts import MonteCarloOptimizer
import copy

class MultiContainerEngine:
    """
    Handles packing items into multiple containers if they don't fit in one.
    """
    def __init__(self, base_container: Container, versus: PackingVersus = PackingVersus.LONGITUDINAL):
        self.base_container = base_container
        self.versus = versus

    def _get_engine(self, container: Container, mode="level2", stability_factor=1.0):
        if mode == "level1":
            return Level1Engine(container, stability_factor=stability_factor, versus=self.versus)
        return Level2Engine(container, stability_factor=stability_factor, versus=self.versus)

    def pack_all(self, items: List[Item], strategy="minimize_out", mode="level2", stability_factor=1.0, grasp_k=1) -> List[Container]:
        """
        Main entry point for multi-container packing.
        Returns a list of Container objects with items assigned.
        """
        remaining_items = copy.deepcopy(items)
        packed_containers = []
        container_count = 0

        while remaining_items:
            container_count += 1
            current_container = Container(
                id=f"{self.base_container.id}_{container_count}",
                width=self.base_container.width,
                height=self.base_container.height,
                depth=self.base_container.depth,
                max_weight=self.base_container.max_weight,
                shape_type=self.base_container.shape_type
            )

            # Choose engine / optimizer
            if mode == "genetic":
                optimizer = GeneticOptimizer(current_container, remaining_items, stability_factor=stability_factor)
                packed_items, _ = optimizer.evolve()
            elif mode == "mcts":
                optimizer = MonteCarloOptimizer(current_container, remaining_items, stability_factor=stability_factor)
                packed_items = optimizer.search()
            else:
                engine = self._get_engine(current_container, mode=mode, stability_factor=stability_factor)
                unpacked = engine.pack(remaining_items, grasp_k=grasp_k)
                packed_items = current_container.items

            packed_ids = {it.id for it in packed_items}
            remaining_items = [it for it in remaining_items if it.id not in packed_ids]
            
            if not packed_items:
                # If we couldn't pack even one item, prevent infinite loop
                break
                
            packed_containers.append(current_container)
            
            # If strategy is just to fill one and stop, we'd exit here, but "pack_all" implies multiple
            if strategy == "suggest_extra" and container_count >= 1:
                # In this mode, maybe we adjust the size of the next container? 
                # For now, we keep it simple.
                pass

        return packed_containers
