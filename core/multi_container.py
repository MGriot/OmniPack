from typing import List, Tuple, Dict, Any
from .models import Item, Container
from .engine import Level1Engine
from .engine_v2 import Level2Engine
from .genetic import GeneticOptimizer
from .mcts import MonteCarloOptimizer
import copy

class MultiContainerEngine:
    def __init__(self, base_container: Container):
        self.base_container = base_container

    def _get_engine(self, container: Container, mode: str):
        if mode == "level1":
            return Level1Engine(container)
        return Level2Engine(container)

    def pack_all(self, items: List[Item], strategy: str = "minimize_out", mode: str = "level2") -> List[Container]:
        containers = []
        remaining_items = copy.deepcopy(items)
        
        # 1. Primary Container
        c1 = Container(
            self.base_container.id + "_Primary",
            self.base_container.width,
            self.base_container.height,
            self.base_container.depth
        )
        
        if mode == "genetic":
            optimizer = GeneticOptimizer(c1, remaining_items, population_size=10, generations=5)
            packed, _ = optimizer.evolve()
            c1.items = packed
        elif mode == "mcts":
            optimizer = MonteCarloOptimizer(c1, remaining_items, time_limit=3.0)
            packed = optimizer.search()
            c1.items = packed
        else:
            engine = self._get_engine(c1, mode)
            unpacked = engine.pack(remaining_items)
            # Find what was packed for consistent handling
            # In Level 1/2 engine is in-place, so we don't need to reassign
        
        packed_ids = {it.id for it in c1.items}
        unpacked = [it for it in remaining_items if it.id not in packed_ids]
        
        containers.append(c1)
        if not unpacked:
            return containers

        # 2. Strategy Logic for overflow
        if strategy == "minimize_out":
            c2 = Container(self.base_container.id + "_Overflow", self.base_container.width, self.base_container.height, self.base_container.depth)
            engine2 = self._get_engine(c2, mode if mode in ["level1", "level2"] else "level2")
            engine2.pack(unpacked)
            containers.append(c2)
            
        elif strategy == "optimal_balance":
            max_w = max((i.width for i in unpacked), default=self.base_container.width)
            total_vol = sum(i.volume() for i in unpacked)
            suggested_h = max(max((i.height for i in unpacked), default=0), total_vol / (max_w * self.base_container.depth))
            
            c2 = Container("Suggested_Space", max_w, suggested_h, self.base_container.depth)
            engine2 = self._get_engine(c2, "level2")
            engine2.pack(unpacked)
            containers.append(c2)

        return containers
