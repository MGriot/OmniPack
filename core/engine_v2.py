import numpy as np
from typing import List, Tuple, Set
from .models import Item, Container, Rotation
from .ep import ExtremePoint, generate_extreme_points
from .accelerated import evaluate_positions_parallel

class Level2Engine:
    """
    PC/Workstation Engine: Uses Numba JIT and Parallel loops to scale performance.
    """
    def __init__(self, container: Container):
        self.container = container
        self.extreme_points: Set[ExtremePoint] = {ExtremePoint(0, 0, 0)}

    def pack(self, items: List[Item]) -> List[Item]:
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        for item in sorted_items:
            # Prepare data for JIT
            eps_list = list(self.extreme_points)
            eps_array = np.array([[p.x, p.y, p.z] for p in eps_list], dtype=np.float64)
            
            # All 6 rotation dimensions
            rots = []
            orig_rot = item.rotation
            for rot in Rotation:
                item.rotation = rot
                rots.append(item.get_dimension())
            item.rotation = orig_rot
            rots_array = np.array(rots, dtype=np.float64)

            # Existing items
            existing_pos = np.array([i.position for i in self.container.items], dtype=np.float64).reshape(-1, 3)
            existing_dim = np.array([i.get_dimension() for i in self.container.items], dtype=np.float64).reshape(-1, 3)
            
            container_dim = np.array([self.container.width, self.container.height, self.container.depth], dtype=np.float64)

            # Parallel evaluation
            valid_mask = evaluate_positions_parallel(eps_array, rots_array, existing_pos, existing_dim, container_dim)
            
            # Find best fit using the same score as Level 1
            best_fit = None # (EP_idx, Rot_idx, Score)
            
            for i in range(len(eps_list)):
                for j in range(6):
                    if valid_mask[i, j]:
                        ep = eps_list[i]
                        score = (ep.z, ep.y, ep.x)
                        if best_fit is None or score < best_fit[2]:
                            best_fit = (i, j, score)
            
            if best_fit:
                ep_idx, rot_idx, _ = best_fit
                ep = eps_list[ep_idx]
                item.position = ep.to_tuple()
                item.rotation = Rotation(rot_idx)
                
                self.container.items.append(item)
                
                # Update EPs
                new_eps = generate_extreme_points(self.container, item)
                self.extreme_points.remove(ep)
                for nep in new_eps:
                    self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
                
        return unpacked_items
