import numpy as np
from numba import njit, prange

@njit(cache=True)
def get_overlap_area(ax1, ay1, ax2, ay2, bx1, by1, bx2, by2):
    """Calculates the intersection area of two rectangles."""
    ix1 = max(ax1, bx1)
    iy1 = max(ay1, by1)
    ix2 = min(ax2, bx2)
    iy2 = min(ay2, by2)
    if ix1 < ix2 and iy1 < iy2:
        return (ix2 - ix1) * (iy2 - iy1)
    return 0.0

@njit(cache=True)
def is_point_in_rect(px, py, rx1, ry1, rx2, ry2):
    return rx1 <= px <= rx2 and ry1 <= py <= ry2

@njit(cache=True)
def calculate_cumulative_loads(existing_pos, existing_dim, existing_weights):
    """
    Calculates cumulative loads using Z-descending propagation.
    Distributes 100% of weight to supports proportional to contact area.
    """
    n = len(existing_pos)
    loads = np.copy(existing_weights)
    if n == 0: return loads

    z_coords = existing_pos[:, 2]
    indices = np.argsort(z_coords)[::-1] # Process from top to bottom

    for i in range(n):
        idx_j = indices[i]
        jx, jy, jz = existing_pos[idx_j]
        jw, jh, jd = existing_dim[idx_j]
        
        # Find supports below item J
        support_indices = []
        support_areas = []
        total_support_area = 0.0
        
        for k in range(n):
            if k == idx_j: continue
            kx, ky, kz = existing_pos[k]
            kw, kh, kd = existing_dim[k]
            
            # If item K is directly below J
            if abs((kz + kd) - jz) < 0.001:
                area = get_overlap_area(jx, jy, jx + jw, jy + jh, kx, ky, kx + kw, ky + kh)
                if area > 0:
                    support_indices.append(k)
                    support_areas.append(area)
                    total_support_area += area
        
        # Distribute J's total load (own weight + load from above)
        if total_support_area > 0:
            for m in range(len(support_indices)):
                k_idx = support_indices[m]
                # Physics: Force transfer is proportional to contact area (simplified)
                # Ensure 100% of weight is transferred
                ratio = support_areas[m] / total_support_area
                loads[k_idx] += loads[idx_j] * ratio

    return loads

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight):
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    
    n_existing = len(existing_pos)
    # Pre-calculate base loads on all items
    base_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            iw, ih, id_ = item_dims[j]
            
            # Boundary Check
            if (ex + iw <= cw and ey + ih <= ch and ez + id_ <= cd):
                collision = False
                
                # 1. Collision Check
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    if (ex < ox + ow and ex + iw > ox and
                        ey < oy + oh and ey + ih > oy and
                        ez < oz + od and ez + id_ > oz):
                        collision = True
                        break
                if collision: continue

                # If on floor, valid (assuming floor has infinite support)
                if ez < 0.001:
                    results[i, j] = True
                    continue

                # 2. Physics Check: Stability & Weight
                # Identify supports
                cog_x = ex + iw / 2.0
                cog_y = ey + ih / 2.0
                
                supports = []
                support_overlaps = []
                total_contact_area = 0.0
                is_cog_supported = False
                
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    
                    # Check if K is directly below
                    if abs(ez - (oz + od)) < 0.001:
                        overlap = get_overlap_area(ex, ey, ex + iw, ey + ih, ox, oy, ox + ow, oy + oh)
                        if overlap > 0:
                            supports.append(k)
                            support_overlaps.append(overlap)
                            total_contact_area += overlap
                            
                            # Stability: Check if CoG is inside this support
                            # (Conservative check: CoG must be supported by at least one item directly)
                            if is_point_in_rect(cog_x, cog_y, ox, oy, ox + ow, oy + oh):
                                is_cog_supported = True

                # Tipping Rule: Center of Gravity must be supported
                if not is_cog_supported:
                    continue
                
                # Weight Distribution Rule
                # Distribute 100% of new item weight to supports
                can_take_load = True
                if total_contact_area > 0:
                    for m in range(len(supports)):
                        k_idx = supports[m]
                        overlap = support_overlaps[m]
                        
                        # Load fraction
                        load_fraction = overlap / total_contact_area
                        added_load = item_weight * load_fraction
                        
                        # Current load on K (total accumulated - its own weight)
                        current_load_on_k = base_loads[k_idx] - existing_weights[k_idx]
                        
                        if (current_load_on_k + added_load) > existing_max_weights[k_idx]:
                            can_take_load = False
                            break
                else:
                    # Floating in air (should be caught by ez check, but safety first)
                    can_take_load = False

                if can_take_load:
                    results[i, j] = True

    return results
