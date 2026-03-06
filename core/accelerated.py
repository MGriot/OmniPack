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
def check_collision(ax, ay, az, aw, ah, ad, ast, ox, oy, oz, ow, oh, od, ost):
    """
    Generalized collision detection for different shapes.
    ast/ost are ShapeType values: BOX=0, SPHERE=1.
    """
    # 1. Bounding Box check (Early Exit / Box-Box)
    if (ax >= ox + ow - 0.001 or ax + aw <= ox + 0.001 or
        ay >= oy + oh - 0.001 or ay + ah <= oy + 0.001 or
        az >= oz + od - 0.001 or az + ad <= oz + 0.001):
        return False
        
    # 2. Detailed check for Spheres
    if ast == 1.0 and ost == 1.0: # Sphere-Sphere
        r1 = aw / 2.0
        r2 = ow / 2.0
        dist_sq = (ax + r1 - (ox + r2))**2 + (ay + r1 - (oy + r2))**2 + (az + r1 - (oz + r2))**2
        return dist_sq < (r1 + r2)**2 - 0.001
        
    if ast == 1.0 and ost == 0.0: # Sphere-Box
        r = aw / 2.0
        cx, cy, cz = ax + r, ay + r, az + r
        clx = max(ox, min(cx, ox + ow))
        cly = max(oy, min(cy, oy + oh))
        clz = max(oz, min(cz, oz + od))
        dist_sq = (cx - clx)**2 + (cy - cly)**2 + (cz - clz)**2
        return dist_sq < r**2 - 0.001

    if ast == 0.0 and ost == 1.0: # Box-Sphere
        r = ow / 2.0
        cx, cy, cz = ox + r, oy + r, oz + r
        clx = max(ax, min(cx, ax + aw))
        cly = max(ay, min(cy, ay + ah))
        clz = max(az, min(cz, az + ad))
        dist_sq = (cx - clx)**2 + (cy - cly)**2 + (cz - clz)**2
        return dist_sq < r**2 - 0.001
        
    # Default: Box-Box (collision if it passed early exit)
    return True

@njit(cache=True)
def is_part_in_container(px, py, pz, pw, ph, pd, pst, cont_parts_data):
    """Checks if an item part is fully contained within the union of container parts."""
    for i in range(len(cont_parts_data)):
        cx, cy, cz, cw, ch, cd, cst = cont_parts_data[i]
        
        # Simple bounding box inclusion
        if (px >= cx - 0.001 and px + pw <= cx + cw + 0.001 and
            py >= cy - 0.001 and py + ph <= cy + ch + 0.001 and
            pz >= cz - 0.001 and pz + pd <= cz + cd + 0.001):
            
            # If container is sphere and item is sphere, check radius inclusion
            if pst == 1.0 and cst == 1.0:
                r_item = pw / 2.0
                r_cont = cw / 2.0
                dist = np.sqrt((px + r_item - (cx + r_cont))**2 + 
                               (py + r_item - (cy + r_cont))**2 + 
                               (pz + r_item - (cz + r_cont))**2)
                if dist + r_item <= r_cont + 0.001:
                    return True
            else:
                return True
    return False

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_parts_data, item_parts_counts, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight, stability_factor=1.0, strategy=0, cont_parts_data=None):
    num_eps = len(eps)
    num_rots = len(item_parts_data)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    n_existing = len(existing_pos)
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim[:, :3], existing_weights)

    # Pre-check: exists any floor position?
    any_floor_valid = False
    if stability_factor > 0.7:
        for i in range(num_eps):
            if eps[i, 2] < 0.001:
                for j in range(num_rots):
                    all_parts_valid = True
                    for p_idx in range(item_parts_counts[j]):
                        px, py, pz, pw, ph, pd, pst = item_parts_data[j, p_idx]
                        ax, ay, az = eps[i, 0] + px, eps[i, 1] + py, eps[i, 2] + pz
                        
                        # Boundary check (Container Parts)
                        if cont_parts_data is not None:
                            if not is_part_in_container(ax, ay, az, pw, ph, pd, pst, cont_parts_data):
                                all_parts_valid = False; break
                        else:
                            if not (ax + pw <= cw + 0.001 and ay + ph <= ch + 0.001 and az + pd <= cd + 0.001):
                                all_parts_valid = False; break
                        
                        collision = False
                        for k in range(n_existing):
                            ox, oy, oz = existing_pos[k]
                            ow, oh, od, ost = existing_dim[k]
                            if check_collision(ax, ay, az, pw, ph, pd, pst, ox, oy, oz, ow, oh, od, ost):
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
                px, py, pz, pw, ph, pd, pst = item_parts_data[j, p_idx]
                ax, ay, az = ex + px, ey + py, ez + pz
                
                # Boundary check (Container Parts)
                if cont_parts_data is not None:
                    if not is_part_in_container(ax, ay, az, pw, ph, pd, pst, cont_parts_data):
                        item_valid = False; break
                else:
                    if not (ax + pw <= cw + 0.001 and ay + ph <= ch + 0.001 and az + pd <= cd + 0.001):
                        item_valid = False; break
                
                collision = False
                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od, ost = existing_dim[k]
                    if check_collision(ax, ay, az, pw, ph, pd, pst, ox, oy, oz, ow, oh, od, ost):
                        collision = True; break
                if collision:
                    item_valid = False; break
            
            if not item_valid: continue

            if ez < 0.001:
                results[i, j] = True
            elif strategy == 2 and (ez > 0):
                is_at_front = False
                for p_idx in range(n_parts):
                    px, py, pz, pw, ph, pd, pst = item_parts_data[j, p_idx]
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
                    px, py, pz, pw, ph, pd, pst = item_parts_data[j, p_idx]
                    ax, ay, az = ex + px, ey + py, ez + pz
                    total_item_footprint += pw * ph
                    
                    for k in range(n_existing):
                        ox, oy, oz = existing_pos[k]
                        ow, oh, od, ost = existing_dim[k]
                        if abs(az - (oz + od)) < 0.001:
                            # Use bounding box for footprint area overlap
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
