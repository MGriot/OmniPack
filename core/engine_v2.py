import numpy as np
from typing import List, Tuple, Set
from .models import Item, Container, Rotation
from .ep import ExtremePoint, generate_extreme_points
from .accelerated import evaluate_positions_parallel

class Level2Engine:
    """
    PC/Workstation Engine: Uses Numba JIT and Parallel loops.
    Optimized for floor utilization and space compactness.
    """
    def __init__(self, container: Container):
        self.container = container
        self.extreme_points: Set[ExtremePoint] = {ExtremePoint(0, 0, 0)}

    def pack(self, items: List[Item]) -> List[Item]:
        # Sort items by volume descending to place large items first
        sorted_items = sorted(items, key=lambda x: x.volume(), reverse=True)
        unpacked_items = []

        container_center_x = self.container.width / 2.0
        container_center_y = self.container.height / 2.0

        for item in sorted_items:
            eps_list = list(self.extreme_points)
            eps_array = np.array([[p.x, p.y, p.z] for p in eps_list], dtype=np.float64)
            
            rots = []
            orig_rot = item.rotation
            for rot in Rotation:
                item.rotation = rot
                rots.append(item.get_dimension())
            item.rotation = orig_rot
            rots_array = np.array(rots, dtype=np.float64)

            existing_pos = np.array([i.position for i in self.container.items], dtype=np.float64).reshape(-1, 3)
            existing_dim = np.array([i.get_dimension() for i in self.container.items], dtype=np.float64).reshape(-1, 3)
            existing_weights = np.array([i.weight for i in self.container.items], dtype=np.float64)
            existing_max_support = np.array([i.max_stack_weight for i in self.container.items], dtype=np.float64)
            
            container_dim = np.array([self.container.width, self.container.height, self.container.depth], dtype=np.float64)

            valid_mask = evaluate_positions_parallel(
                eps_array, 
                rots_array, 
                existing_pos, 
                existing_dim, 
                existing_weights,
                existing_max_support,
                container_dim,
                item.weight
            )
            
            best_fit = None # (EP_idx, Rot_idx, Score)
            
            for i in range(len(eps_list)):
                for j in range(6):
                    if valid_mask[i, j]:
                        ep = eps_list[i]
                        w, h, d = rots[j]
                        
                        # SCORING STRATEGY:
                        # 1. Heavily prioritize Lower Z (Gravity/Floor usage)
                        # 2. To avoid fragmentation, prioritize "tucking" into the back-left corner
                        # 3. Use barycentric distance as a tie-breaker for stability if Z and corner are equal
                        
                        # Primary: Z (minimize)
                        # Secondary: Y (minimize - tuck back)
                        # Tertiary: X (minimize - tuck left)
                        
                        score = (ep.z, ep.y, ep.x)
                        
                        if best_fit is None or score < best_fit[2]:
                            best_fit = (i, j, score)
            
            if best_fit:
                ep_idx, rot_idx, _ = best_fit
                ep = eps_list[ep_idx]
                item.position = ep.to_tuple()
                item.rotation = Rotation(rot_idx)
                self.container.items.append(item)
                
                # Update EPs and also add intermediate points to improve floor discovery
                new_eps = generate_extreme_points(self.container, item)
                self.extreme_points.remove(ep)
                for nep in new_eps: self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
                
        return unpacked_items
