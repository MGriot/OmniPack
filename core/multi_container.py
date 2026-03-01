from typing import List, Tuple, Dict
from .models import Item, Container
from .engine_v2 import Level2Engine
import copy

class MultiContainerEngine:
    """
    Handles packing into multiple containers if items don't fit in the first one.
    Strategies:
    - 'minimize_out': Maximize items in the first container, then use a second for the rest.
    - 'optimal_balance': Balance items between containers to minimize 'criticity' (overflow).
    """
    def __init__(self, base_container: Container):
        self.base_container = base_container

    def pack_all(self, items: List[Item], strategy: str = "minimize_out") -> List[Container]:
        containers = []
        remaining_items = copy.deepcopy(items)
        
        # 1. Fill the first (primary) container
        c1 = Container(
            self.base_container.id + "_Primary",
            self.base_container.width,
            self.base_container.height,
            self.base_container.depth
        )
        engine1 = Level2Engine(c1)
        unpacked_after_c1 = engine1.pack(remaining_items)
        containers.append(c1)
        
        if not unpacked_after_c1:
            return containers

        # 2. Strategy Logic for the overflow
        if strategy == "minimize_out":
            # Just pack what's left into a second container of the same size
            c2 = Container(
                self.base_container.id + "_Overflow",
                self.base_container.width,
                self.base_container.height,
                self.base_container.depth
            )
            engine2 = Level2Engine(c2)
            engine2.pack(unpacked_after_c1)
            containers.append(c2)
            
        elif strategy == "optimal_balance":
            # Suggest a specific space size for the remaining items 
            # to minimize unused volume in the second container
            max_w = max((i.width for i in unpacked_after_c1), default=0)
            max_h = max((i.height for i in unpacked_after_c1), default=0)
            max_d = max((i.depth for i in unpacked_after_c1), default=0)
            
            # Simple heuristic for suggested secondary space:
            # Fit to the bounding box of remaining items or volume-based
            total_vol_needed = sum(i.volume() for i in unpacked_after_c1)
            suggested_w = max(max_w, self.base_container.width)
            suggested_h = max(max_h, total_vol_needed / (suggested_w * self.base_container.depth))
            suggested_d = self.base_container.depth
            
            c2 = Container(
                "Suggested_Secondary_Space",
                suggested_w,
                suggested_h,
                suggested_d
            )
            engine2 = Level2Engine(c2)
            engine2.pack(unpacked_after_c1)
            containers.append(c2)

        return containers
