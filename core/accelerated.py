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
def is_point_in_rect(px, py, rx1, ry1, rx2, ry2):
    return (rx1 - 0.001) <= px <= (rx2 + 0.001) and (ry1 - 0.001) <= py <= (ry2 + 0.001)

@njit(cache=True)
def calculate_cumulative_loads(existing_pos, existing_dim, existing_weights):
    n = len(existing_pos)
    total_loads = np.copy(existing_weights)
    if n == 0: return total_loads
    z_coords = existing_pos[:, 2]
    indices = np.argsort(z_coords)[::-1]
    for i in range(n):
        curr_idx = indices[i]
        jx, jy, jz = existing_pos[curr_idx]
        jw, jh, jd = existing_dim[curr_idx]
        supports = []
        areas = []
        total_contact = 0.0
        for k in range(n):
            if k == curr_idx: continue
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
                support_idx = supports[m]
                ratio = areas[m] / total_contact
                total_loads[support_idx] += total_loads[curr_idx] * ratio
    return total_loads

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight):
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    n_existing = len(existing_pos)
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    # Pre-check: Is there ANY valid spot on the floor (Z=0)?
    # We must do this to enforce "Floor-First" logic.
    any_floor_valid = False
    floor_valid_mask = np.zeros((num_eps, num_rots), dtype=np.bool_)

    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        # Only check floor EPs in this pass
        if ez < 0.001:
            for j in range(num_rots):
                iw, ih, id_ = item_dims[j]
                if (ex + iw <= cw and ey + ih <= ch and ez + id_ <= cd):
                    collision = False
                    for k in range(n_existing):
                        ox, oy, oz = existing_pos[k]
                        ow, oh, od = existing_dim[k]
                        if (ex < ox + ow - 0.001 and ex + iw > ox + 0.001 and
                            ey < oy + oh - 0.001 and ey + ih > oy + 0.001 and
                            ez < oz + od - 0.001 and ez + id_ > oz + 0.001):
                            collision = True
                            break
                    if not collision:
                        floor_valid_mask[i, j] = True
                        # Using an atomic-like check for parallel safety
                        # (Not strictly necessary for bool but good practice)
    
    # Check if we found floor spots
    for i in range(num_eps):
        for j in range(num_rots):
            if floor_valid_mask[i, j]:
                any_floor_valid = True
                break
        if any_floor_valid: break

    # Final Pass
    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            # RULE: If ANY floor spot exists, we block all stacking spots for this item.
            if any_floor_valid and ez >= 0.001:
                results[i, j] = False
                continue
            
            # Standard logic for the rest
            iw, ih, id_ = item_dims[j]
            if (ex + iw <= cw and ey + ih <= ch and ez + id_ <= cd):
                collision = False
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    if (ex < ox + ow - 0.001 and ex + iw > ox + 0.001 and
                        ey < oy + oh - 0.001 and ey + ih > oy + 0.001 and
                        ez < oz + od - 0.001 and ez + id_ > oz + 0.001):
                        collision = True
                        break
                if collision: continue

                if ez < 0.001:
                    results[i, j] = True
                else:
                    # Physics check for stacked items
                    cog_x, cog_y = ex + iw/2.0, ey + ih/2.0
                    total_support_area = 0.0
                    is_cog_supported = False
                    support_indices, support_overlaps = [], []
                    for k in range(n_existing):
                        ox, oy, oz = existing_pos[k]
                        ow, oh, od = existing_dim[k]
                        if abs(ez - (oz + od)) < 0.001:
                            area = get_overlap_area(ex, ey, ex + iw, ey + ih, ox, oy, ox + ow, oy + oh)
                            if area > 0:
                                total_support_area += area
                                support_indices.append(k)
                                support_overlaps.append(area)
                                if is_point_in_rect(cog_x, cog_y, ox, oy, ox + ow, oy + oh):
                                    is_cog_supported = True
                    
                    if not is_cog_supported: continue
                    
                    can_sustain = True
                    for m in range(len(support_indices)):
                        idx_k = support_indices[m]
                        added_load = item_weight * (support_overlaps[m] / total_support_area)
                        if (current_loads[idx_k] - existing_weights[idx_k] + added_load) > existing_max_weights[idx_k]:
                            can_sustain = False
                            break
                    if can_sustain:
                        results[i, j] = True
    return results
