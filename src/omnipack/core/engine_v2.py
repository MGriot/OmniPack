import numpy as np
from typing import List, Tuple, Set
from .models import Item, Container, Rotation, LoadingStrategy, PackingVersus
from .ep import ExtremePoint, generate_extreme_points, get_valid_ep
from .accelerated import evaluate_positions_parallel

class Level2Engine:
    def __init__(self, container: Container, stability_factor=1.0, versus: PackingVersus = PackingVersus.LONGITUDINAL, random_disturbance=0.0):
        self.container = container
        self.stability_factor = stability_factor
        self.versus = versus
        self.random_disturbance = random_disturbance
        self.extreme_points: Set[ExtremePoint] = set()
        self.extreme_points.add(ExtremePoint(0, 0, 0))
        self.extreme_points.add(ExtremePoint(0, 0, container.depth))

    def pack(self, items: List[Item], grasp_k: int = 1, **kwargs) -> List[Item]:
        def sort_key(x):
            s_map = {LoadingStrategy.LIFO: 0, LoadingStrategy.FIFO: 1, LoadingStrategy.NONE: 2}
            vol_disturb = (np.random.random() - 0.5) * self.random_disturbance * x.volume()
            return (s_map.get(x.strategy, 2), -x.stop_id, -(x.volume() + vol_disturb))

        sorted_items = sorted(items, key=sort_key)
        unpacked_items = []

        for item in sorted_items:
            eps_list = list(self.extreme_points)
            eps_array = np.array([[p.x, p.y, p.z] for p in eps_list], dtype=np.float64)
            
            rots_dims = np.zeros((6, 4), dtype=np.float64)
            orig_rot = item.rotation
            for r_idx in range(6):
                item.rotation = Rotation(r_idx)
                w, h, d = item.get_dimension()
                rots_dims[r_idx] = [w, h, d, float(item.shape_type)]
            item.rotation = orig_rot

            existing_pos = np.zeros((len(self.container.items), 3), dtype=np.float64)
            existing_dim = np.zeros((len(self.container.items), 4), dtype=np.float64)
            existing_weights = np.zeros(len(self.container.items), dtype=np.float64)
            existing_max_support = np.zeros(len(self.container.items), dtype=np.float64)
            
            for idx, ex in enumerate(self.container.items):
                existing_pos[idx] = ex.position
                ew, eh, ed = ex.get_dimension()
                existing_dim[idx] = [ew, eh, ed, float(ex.shape_type)]
                existing_weights[idx] = ex.weight
                existing_max_support[idx] = ex.max_stack_weight
            
            container_dim = np.array([self.container.width, self.container.height, self.container.depth], dtype=np.float64)

            valid_mask = evaluate_positions_parallel(
                eps_array, rots_dims, existing_pos, existing_dim, 
                existing_weights, existing_max_support, container_dim, 
                float(self.container.shape_type), item.weight,
                stability_factor=self.stability_factor,
                strategy=int(item.strategy)
            )
            
            candidates = []
            for i in range(len(eps_list)):
                for j in range(6):
                    if Rotation(j) not in item.allowed_rotations: continue
                    if valid_mask[i, j]:
                        ep = eps_list[i]
                        item.rotation = Rotation(j)
                        bw, bh, bd = item.get_dimension()
                        z_val = (self.container.depth - (ep.z + bd)) if item.strategy == LoadingStrategy.LIFO else ep.z
                        balance_penalty = abs(ep.x + bw/2 - self.container.width/2) / self.container.width

                        if self.versus == PackingVersus.LATERAL: score = (z_val, ep.y, ep.x, balance_penalty)
                        elif self.versus == PackingVersus.FLOOR_FIRST: score = (ep.y, z_val, ep.x, balance_penalty)
                        elif self.versus == PackingVersus.WALL_BUILDING: score = (ep.x, z_val, ep.y)
                        else: score = (ep.x, ep.y, z_val, balance_penalty)

                        if self.random_disturbance > 0:
                            jitter = (np.random.random() - 0.5) * self.random_disturbance
                            score = tuple(s + jitter for s in score)
                        candidates.append((i, j, score, None))

                    if item.strategy == LoadingStrategy.LIFO:
                        ep = eps_list[i]
                        item.rotation = Rotation(j)
                        bw, bh, bd = item.get_dimension()
                        target_z = ep.z - bd
                        if target_z >= -0.001:
                            temp_ep = np.array([[ep.x, ep.y, target_z]], dtype=np.float64)
                            back_mask = evaluate_positions_parallel(
                                temp_ep, np.array([rots_dims[j]], dtype=np.float64),
                                existing_pos, existing_dim, existing_weights,
                                existing_max_support, container_dim,
                                float(self.container.shape_type), item.weight,
                                stability_factor=self.stability_factor,
                                strategy=int(item.strategy)
                            )
                            if back_mask[0, 0]:
                                z_val = (self.container.depth - (target_z + bd))
                                balance_penalty = abs(ep.x + bw/2 - self.container.width/2) / self.container.width
                                if self.versus == PackingVersus.LATERAL: score = (z_val, ep.y, ep.x, balance_penalty)
                                elif self.versus == PackingVersus.FLOOR_FIRST: score = (ep.y, z_val, ep.x, balance_penalty)
                                elif self.versus == PackingVersus.WALL_BUILDING: score = (ep.x, z_val, ep.y)
                                else: score = (z_val, ep.y, ep.x, balance_penalty)

                                if self.random_disturbance > 0:
                                    jitter = (np.random.random() - 0.5) * self.random_disturbance
                                    score = tuple(s + jitter for s in score)
                                candidates.append((i, j, score, target_z))

            if candidates:
                candidates.sort(key=lambda x: x[2])
                actual_k = min(grasp_k, len(candidates))
                chosen = candidates[0] if actual_k <= 1 else candidates[np.random.randint(0, actual_k)]
                ep_idx, rot_idx, _, t_z = chosen
                ep = eps_list[ep_idx]
                item.position = (ep.x, ep.y, t_z) if t_z is not None else ep.to_tuple()
                item.rotation = Rotation(rot_idx)
                self.container.items.append(item)
                self.extreme_points.remove(ep)
                for nep in generate_extreme_points(self.container, item): self.extreme_points.add(nep)
            else:
                unpacked_items.append(item)
        return unpacked_items
