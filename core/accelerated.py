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

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight):
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    n_existing = len(existing_pos)
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            iw, ih, id_ = item_dims[j]
            
            # 1. Physical fit (Boundary & Collision)
            if (ex + iw <= cw + 0.001 and ey + ih <= ch + 0.001 and ez + id_ <= cd + 0.001):
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

                # 2. Support check
                if ez < 0.001: # Floor
                    results[i, j] = True
                else:
                    total_contact = 0.0
                    support_indices = []
                    support_areas = []
                    for k in range(n_existing):
                        ox, oy, oz = existing_pos[k]
                        ow, oh, od = existing_dim[k]
                        if abs(ez - (oz + od)) < 0.001:
                            area = get_overlap_area(ex, ey, ex + iw, ey + ih, ox, oy, ox + ow, oy + oh)
                            if area > 0:
                                total_contact += area
                                support_indices.append(k)
                                support_areas.append(area)
                    
                    # Tipping Rule: Center of Gravity must be over supports
                    # Simple version: Area supported must be > 50% of footprint
                    if total_contact > (iw * ih * 0.5):
                        # Weight Rule: Supports must not exceed capacity
                        can_sustain = True
                        for m in range(len(support_indices)):
                            idx_k = support_indices[m]
                            added_load = item_weight * (support_areas[m] / total_contact)
                            # Load on K is (Total - Own Weight)
                            if (current_loads[idx_k] - existing_weights[idx_k] + added_load) > existing_max_weights[idx_k] + 0.001:
                                can_sustain = False
                                break
                        if can_sustain:
                            results[i, j] = True
    return results
