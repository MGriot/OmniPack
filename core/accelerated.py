import numpy as np
from numba import njit, prange

@njit(cache=True)
def get_overlap_area(ax1, ay1, ax2, ay2, bx1, by1, bx2, by2):
    ix1 = max(ax1, bx1)
    iy1 = max(ay1, by1)
    ix2 = min(ax2, bx2)
    iy2 = min(ay2, by2)
    if ix1 < ix2 and iy1 < iy2:
        return (ix2 - ix1) * (iy2 - iy1)
    return 0.0

@njit(cache=True)
def calculate_cumulative_loads(existing_pos, existing_dim, existing_weights):
    n = len(existing_pos)
    loads = np.copy(existing_weights)
    if n == 0: return loads
    z_coords = existing_pos[:, 2]
    indices = np.argsort(z_coords)[::-1]
    for i in range(n):
        idx_j = indices[i]
        jx, jy, jz = existing_pos[idx_j]
        jw, jh, jd = existing_dim[idx_j]
        supports = []
        areas = []
        total_contact = 0.0
        for k in range(n):
            if k == idx_j: continue
            kx, ky, kz = existing_pos[k]
            kw, kh, kd = existing_dim[k]
            if abs((kz + kd) - jz) < 0.001:
                area = get_overlap_area(jx, jy, jx + jw, jy + jh, kx, ky, kx + kw, ky + kh)
                if area > 0:
                    supports.append(k)
                    areas.append(area)
                    total_contact += area
        if total_contact > 0:
            for m in range(len(supports)):
                idx_k = supports[m]
                ratio = areas[m] / total_contact
                loads[idx_k] += loads[idx_j] * ratio
    return loads

@njit(cache=True)
def is_part_in_container(px, py, pz, pw, ph, pd, cont_parts_data):
    """Checks if an item part is fully contained within the union of container parts."""
    # Note: Union containment is hard. For simplicity, we check if it's within AT LEAST ONE part.
    # More advanced: Check if the part is completely covered by the union.
    # For now, we assume container parts are large enough.
    for i in range(len(cont_parts_data)):
        cx, cy, cz, cw, ch, cd = cont_parts_data[i]
        if (px >= cx - 0.001 and px + pw <= cx + cw + 0.001 and
            py >= cy - 0.001 and py + ph <= cy + ch + 0.001 and
            pz >= cz - 0.001 and pz + pd <= cz + cd + 0.001):
            return True
    return False

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_parts_data, item_parts_counts, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight, stability_factor=1.0, strategy=0, cont_parts_data=None):
    num_eps = len(eps)
    num_rots = len(item_parts_data)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    n_existing = len(existing_pos)
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    # Pre-check: exists any floor position?
    any_floor_valid = False
    if stability_factor > 0.7:
        for i in range(num_eps):
            if eps[i, 2] < 0.001:
                for j in range(num_rots):
                    all_parts_valid = True
                    for p_idx in range(item_parts_counts[j]):
                        px, py, pz, pw, ph, pd = item_parts_data[j, p_idx]
                        ax, ay, az = eps[i, 0] + px, eps[i, 1] + py, eps[i, 2] + pz
                        
                        # Boundary check (Container Parts)
                        if cont_parts_data is not None:
                            if not is_part_in_container(ax, ay, az, pw, ph, pd, cont_parts_data):
                                all_parts_valid = False; break
                        else:
                            if not (ax + pw <= cw + 0.001 and ay + ph <= ch + 0.001 and az + pd <= cd + 0.001):
                                all_parts_valid = False; break
                        
                        collision = False
                        for k in range(n_existing):
                            if (ax < existing_pos[k, 0] + existing_dim[k, 0] - 0.001 and ax + pw > existing_pos[k, 0] + 0.001 and
                                ay < existing_pos[k, 1] + existing_dim[k, 1] - 0.001 and ay + ph > existing_pos[k, 1] + 0.001 and
                                az < existing_pos[k, 2] + existing_dim[k, 2] - 0.001 and az + pd > existing_pos[k, 2] + 0.001):
                                collision = True; break
                        if collision:
                            all_parts_valid = False; break
                    
                    if all_parts_valid:
                        any_floor_valid = True; break
            if any_floor_valid: break

    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        
        if stability_factor > 0.9 and any_floor_valid and ez > 0.001:
            continue

        for j in range(num_rots):
            n_parts = item_parts_counts[j]
            item_valid = True
            for p_idx in range(n_parts):
                px, py, pz, pw, ph, pd = item_parts_data[j, p_idx]
                ax, ay, az = ex + px, ey + py, ez + pz
                
                # Boundary check (Container Parts)
                if cont_parts_data is not None:
                    if not is_part_in_container(ax, ay, az, pw, ph, pd, cont_parts_data):
                        item_valid = False; break
                else:
                    if not (ax + pw <= cw + 0.001 and ay + ph <= ch + 0.001 and az + pd <= cd + 0.001):
                        item_valid = False; break
                
                collision = False
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    if (ax < ox + ow - 0.001 and ax + pw > ox + 0.001 and
                        ay < oy + oh - 0.001 and ay + ph > oy + 0.001 and
                        az < oz + od - 0.001 and az + pd > oz + 0.001):
                        collision = True; break
                if collision:
                    item_valid = False; break
            
            if not item_valid: continue

            if ez < 0.001:
                results[i, j] = True
            elif strategy == 2 and (ez > 0):
                is_at_front = False
                for p_idx in range(n_parts):
                    px, py, pz, pw, ph, pd = item_parts_data[j, p_idx]
                    if abs((ez + pz + pd) - cd) < 0.001:
                        is_at_front = True; break
                if is_at_front:
                    results[i, j] = True
                    continue
            
            if not results[i, j]:
                total_item_footprint = 0.0
                total_contact_area = 0.0
                support_indices = []
                support_areas = []
                
                for p_idx in range(n_parts):
                    px, py, pz, pw, ph, pd = item_parts_data[j, p_idx]
                    ax, ay, az = ex + px, ey + py, ez + pz
                    total_item_footprint += pw * ph
                    
                    for k in range(n_existing):
                        ox, oy, oz = existing_pos[k]
                        ow, oh, od = existing_dim[k]
                        if abs(az - (oz + od)) < 0.001:
                            area = get_overlap_area(ax, ay, ax + pw, ay + ph, ox, oy, ox + ow, oy + oh)
                            if area > 0:
                                total_contact_area += area
                                support_indices.append(k)
                                support_areas.append(area)
                
                if total_contact_area > (total_item_footprint * 0.5):
                    can_sustain = True
                    for m in range(len(support_indices)):
                        idx_k = support_indices[m]
                        added_load = item_weight * (support_areas[m] / total_contact_area)
                        if (current_loads[idx_k] - existing_weights[idx_k] + added_load) > existing_max_weights[idx_k] + 0.001:
                            can_sustain = False; break
                    if can_sustain:
                        results[i, j] = True
    return results
