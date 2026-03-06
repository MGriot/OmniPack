import numpy as np
from typing import List, Tuple, Set
from .models import Item, Container, Rotation, LoadingStrategy, PackingVersus
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep
from .accelerated import evaluate_positions_parallel

class Level2Engine:
    """
    PC/Workstation Engine: Uses Numba JIT and Parallel loops.
    Supports Non-Parallelepiped shapes for BOTH items and containers.
    """
    def __init__(self, container: Container, stability_factor=1.0, versus: PackingVersus = PackingVersus.LONGITUDINAL):
        self.container = container
        self.stability_factor = stability_factor
        self.versus = versus

        self.extreme_points: Set[ExtremePoint] = set()
        # Seed from ALL container parts to ensure we find all "entry" points
        for part in container.parts:
            # Back entry (Z = offset Z)
            self.extreme_points.add(ExtremePoint(part.dx, part.dy, part.dz))
            # Front entry (Z = offset Z + depth)
            self.extreme_points.add(ExtremePoint(part.dx, part.dy, part.dz + part.depth))

        # Ensure at least (0,0,0) is there if no parts were somehow defined (post_init should handle it though)
        if not self.extreme_points:
            self.extreme_points.add(ExtremePoint(0, 0, 0))
            self.extreme_points.add(ExtremePoint(0, 0, container.depth))


    def pack(self, items: List[Item], grasp_k: int = 1, **kwargs) -> List[Item]:
        def sort_key(x):
            s_map = {LoadingStrategy.LIFO: 0, LoadingStrategy.FIFO: 1, LoadingStrategy.NONE: 2}
            return (s_map.get(x.strategy, 2), -x.stop_id, -x.volume())

        sorted_items = sorted(items, key=sort_key)
        unpacked_items = []

        # Prepare container parts data
        cont_parts_list = self.container.parts
        cont_parts_data = np.zeros((len(cont_parts_list), 6), dtype=np.float64)
        for i, cp in enumerate(cont_parts_list):
            cont_parts_data[i] = [cp.dx, cp.dy, cp.dz, cp.width, cp.height, cp.depth]

        for item in sorted_items:
            eps_list = list(self.extreme_points)
            eps_array = np.array([[p.x, p.y, p.z] for p in eps_list], dtype=np.float64)
            
            # Prepare parts for all 6 rotations
            max_parts = max(len(it.parts) for it in sorted_items)
            rots_parts_data = np.zeros((6, max_parts, 6), dtype=np.float64)
            rots_parts_counts = np.zeros(6, dtype=np.int32)
            
            orig_rot = item.rotation
            for r_idx in range(6):
                item.rotation = Rotation(r_idx)
                p_list = item.get_rotated_parts()
                rots_parts_counts[r_idx] = len(p_list)
                for p_idx, ((px, py, pz), (pw, ph, pd)) in enumerate(p_list):
                    rots_parts_data[r_idx, p_idx] = [px, py, pz, pw, ph, pd]
            item.rotation = orig_rot

            # Expand existing items into boxes
            expanded_pos = []
            expanded_dim = []
            expanded_weights = []
            expanded_max_support = []
            
            for ex_item in self.container.items:
                ex_parts = ex_item.get_rotated_parts()
                total_vol = ex_item.volume()
                for (px, py, pz), (pw, ph, pd) in ex_parts:
                    expanded_pos.append([ex_item.position[0] + px, ex_item.position[1] + py, ex_item.position[2] + pz])
                    expanded_dim.append([pw, ph, pd])
                    part_vol = pw * ph * pd
                    expanded_weights.append(ex_item.weight * (part_vol / total_vol) if total_vol > 0 else 0)
                    expanded_max_support.append(ex_item.max_stack_weight)
            
            existing_pos = np.array(expanded_pos, dtype=np.float64).reshape(-1, 3) if expanded_pos else np.zeros((0, 3))
            existing_dim = np.array(expanded_dim, dtype=np.float64).reshape(-1, 3) if expanded_dim else np.zeros((0, 3))
            existing_weights = np.array(expanded_weights, dtype=np.float64)
            existing_max_support = np.array(expanded_max_support, dtype=np.float64)
            
            container_dim = np.array([self.container.width, self.container.height, self.container.depth], dtype=np.float64)

            # Parallel verification
            valid_mask = evaluate_positions_parallel(
                eps_array, rots_parts_data, rots_parts_counts, existing_pos, existing_dim, 
                existing_weights, existing_max_support, container_dim, item.weight,
                stability_factor=self.stability_factor,
                strategy=int(item.strategy),
                cont_parts_data=cont_parts_data
            )
            
            candidates = []
            for i in range(len(eps_list)):
                for j in range(6):
                    if Rotation(j) not in item.allowed_rotations:
                        continue
                        
                    if valid_mask[i, j]:
                        ep = eps_list[i]
                        item.rotation = Rotation(j)
                        bw, bh, bd = item.get_dimension()
                        
                        if item.strategy == LoadingStrategy.LIFO:
                            z_val = (self.container.depth - (ep.z + bd))
                        else:
                            z_val = ep.z
                        
                        if self.versus == PackingVersus.LATERAL:
                            score = (ep.x, ep.y, z_val)
                        elif self.versus == PackingVersus.FLOOR_FIRST:
                            score = (ep.y, z_val, ep.x)
                        else: # LONGITUDINAL
                            score = (z_val, ep.y, ep.x)
                            
                        candidates.append((i, j, score, None))
                    
                    # Special case for LIFO: Test if we can place the item ENDING at this EP
                    if item.strategy == LoadingStrategy.LIFO:
                        ep = eps_list[i]
                        item.rotation = Rotation(j)
                        bw, bh, bd = item.get_dimension()
                        target_z = ep.z - bd
                        if target_z >= -0.001:
                            temp_ep = np.array([[ep.x, ep.y, target_z]], dtype=np.float64)
                            back_mask = evaluate_positions_parallel(
                                temp_ep, np.array([rots_parts_data[j]], dtype=np.float64), 
                                np.array([rots_parts_counts[j]], dtype=np.int32),
                                existing_pos, existing_dim, existing_weights, 
                                existing_max_support, container_dim, item.weight,
                                stability_factor=self.stability_factor,
                                strategy=int(item.strategy),
                                cont_parts_data=cont_parts_data
                            )
                            if back_mask[0, 0]:
                                z_val = (self.container.depth - (target_z + bd))
                                if self.versus == PackingVersus.LATERAL:
                                    score = (ep.x, z_val, ep.y)
                                elif self.versus == PackingVersus.FLOOR_FIRST:
                                    score = (ep.y, ep.x, z_val)
                                else: # LONGITUDINAL
                                    score = (z_val, ep.y, ep.x)
                                candidates.append((i, j, score, target_z))
            
            if candidates:
                candidates.sort(key=lambda x: x[2])
                actual_k = min(grasp_k, len(candidates))
                choice_idx = np.random.randint(0, actual_k) if actual_k > 1 else 0
                chosen = candidates[choice_idx]
                
                ep_idx, rot_idx, _, t_z = chosen
                ep = eps_list[ep_idx]
                
                if t_z is not None:
                    item.position = (ep.x, ep.y, t_z)
                else:
                    item.position = ep.to_tuple()
                
                item.rotation = Rotation(rot_idx)
                self.container.items.append(item)
                
                self.extreme_points.remove(ep)
                new_eps = generate_extreme_points(self.container, item)
                for nep in new_eps:
                    self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
                
        return unpacked_items
