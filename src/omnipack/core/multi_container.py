from typing import List, Tuple
from .models import Item, Container, PackingVersus
from .engine import Level1Engine
from .engine_v2 import Level2Engine
import copy

class MultiContainerEngine:
    def __init__(self, base_container: Container, versus: PackingVersus = PackingVersus.LONGITUDINAL):
        self.base_container = base_container
        self.versus = versus

    def _get_engine(self, container: Container, mode="level2", stability_factor=1.0, random_disturbance=0.0):
        if mode == "level1":
            return Level1Engine(container, stability_factor=stability_factor, versus=self.versus)
        return Level2Engine(container, stability_factor=stability_factor, versus=self.versus, random_disturbance=random_disturbance)

    def pack_all(self, items: List[Item], strategy="minimize_out", mode="level2", stability_factor=1.0, grasp_k=1, random_disturbance=0.0) -> List[Container]:
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

            engine = self._get_engine(current_container, mode=mode, stability_factor=stability_factor, random_disturbance=random_disturbance)
            unpacked = engine.pack(remaining_items, grasp_k=grasp_k)
            packed_items = current_container.items

            packed_ids = {it.id for it in packed_items}
            remaining_items = [it for it in remaining_items if it.id not in packed_ids]
            
            if not packed_items: break
            packed_containers.append(current_container)

        return packed_containers
