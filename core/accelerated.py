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
    # Small epsilon for float precision
    return (rx1 - 0.001) <= px <= (rx2 + 0.001) and (ry1 - 0.001) <= py <= (ry2 + 0.001)

@njit(cache=True)
def calculate_cumulative_loads(existing_pos, existing_dim, existing_weights):
    """
    Physics-accurate load propagation.
    Ensures 100% of the weight of an upper item is transferred to the items 
    immediately supporting it, distributed by contact area.
    """
    n = len(existing_pos)
    # Start with the intrinsic weight of each item
    total_loads = np.copy(existing_weights)
    if n == 0: return total_loads

    # Propagation must happen from top to bottom (Z descending)
    z_coords = existing_pos[:, 2]
    indices = np.argsort(z_coords)[::-1]

    for i in range(n):
        curr_idx = indices[i]
        jx, jy, jz = existing_pos[curr_idx]
        jw, jh, jd = existing_dim[curr_idx]
        
        # Find direct supports (items whose top matches our bottom)
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
        
        # Transfer 100% of the CURRENT total load of item J (its weight + what's on it)
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
    # Pre-calculate loads of the current stable state
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            iw, ih, id_ = item_dims[j]
            
            # 1. Boundary & Overlap check
            if (ex + iw <= cw and ey + ih <= ch and ez + id_ <= cd):
                collision = False
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    if (ex < ox + ow and ex + iw > ox and
                        ey < oy + oh and ey + ih > oy and
                        ez < oz + od and ez + id_ > oz):
                        collision = True
                        break
                if collision: continue

                # 2. Physics: Floor check
                if ez < 0.001:
                    results[i, j] = True
                    continue

                # 3. Physics: Support & Center of Gravity (Barycentric Rules)
                cog_x = ex + iw / 2.0
                cog_y = ey + ih / 2.0
                
                total_support_area = 0.0
                is_cog_supported = False
                support_indices = []
                support_overlaps = []
                
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    
                    if abs(ez - (oz + od)) < 0.001:
                        area = get_overlap_area(ex, ey, ex + iw, ey + ih, ox, oy, ox + ow, oy + oh)
                        if area > 0:
                            total_support_area += area
                            support_indices.append(k)
                            support_overlaps.append(area)
                            # Barycentric rule: CoG projection must be within the supporting item's footprint
                            if is_point_in_rect(cog_x, cog_y, ox, oy, ox + ow, oy + oh):
                                is_cog_supported = True

                # Item must be balanced (CoG supported)
                if not is_cog_supported:
                    continue
                
                # 4. Physics: Structural Load Propagation
                # Check if the NEW item crushes ANY item in the chain below it
                can_sustain = True
                for m in range(len(support_indices)):
                    idx_k = support_indices[m]
                    area_k = support_overlaps[m]
                    
                    # Fraction of 100% of the new item's weight
                    added_load = item_weight * (area_k / total_support_area)
                    
                    # The item idx_k is already sustaining (current_loads[idx_k] - existing_weights[idx_k])
                    # If we add our portion, does it exceed idx_k's capacity?
                    if (current_loads[idx_k] - existing_weights[idx_k] + added_load) > existing_max_weights[idx_k]:
                        can_sustain = False
                        break
                
                if can_sustain:
                    results[i, j] = True

    return results
