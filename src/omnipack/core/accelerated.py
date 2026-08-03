import numpy as np
import logging

try:
    from numba import njit, prange
except ImportError:
    logging.warning("Numba not found. Falling back to pure Python (slower).")
    def njit(*args, **kwargs):
        def decorator(func):
            return func
        return decorator
    prange = range

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
        jw, jh, jd = existing_dim[idx_j, :3]
        supports = []
        areas = []
        total_contact = 0.0
        for k in range(n):
            if k == idx_j: continue
            kx, ky, kz = existing_pos[k]
            kw, kh, kd = existing_dim[k, :3]
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
    # 1. Bounding Box check (Early Exit)
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

    return True

@njit(cache=True)
def is_in_container(ax, ay, az, aw, ah, ad, ast, cw, ch, cd, cst):
    # Bounding box inclusion
    if (ax >= -0.001 and ax + aw <= cw + 0.001 and
        ay >= -0.001 and ay + ah <= ch + 0.001 and
        az >= -0.001 and az + ad <= cd + 0.001):

        if ast == 1.0 and cst == 1.0: # Item Sphere in Container Sphere
            r_item = aw / 2.0
            r_cont = cw / 2.0
            dist = np.sqrt((ax + r_item - r_cont)**2 + (ay + r_item - r_cont)**2 + (az + r_item - r_cont)**2)
            return dist + r_item <= r_cont + 0.001
        return True
    return False

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, container_shape, item_weight, stability_factor=1.0, strategy=0):
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    n_existing = len(existing_pos)
    current_loads = calculate_cumulative_loads(existing_pos, existing_dim, existing_weights)

    for i in prange(num_eps):
        ax, ay, az = eps[i]

        for j in range(num_rots):
            aw, ah, ad, ast = item_dims[j]

            if not is_in_container(ax, ay, az, aw, ah, ad, ast, cw, ch, cd, container_shape):
                continue

            collision = False
            for k in range(n_existing):
                ox, oy, oz = existing_pos[k]
                ow, oh, od, ost = existing_dim[k]
                if check_collision(ax, ay, az, aw, ah, ad, ast, ox, oy, oz, ow, oh, od, ost):
                    collision = True; break
            if collision: continue

            if az < 0.001:
                results[i, j] = True
            elif strategy == 2 and (az + ad >= cd - 0.001): # LIFO Door support
                results[i, j] = True

            if not results[i, j]:
                total_contact_area = 0.0
                support_indices = []
                support_areas = []

                for k in range(n_existing):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od, ost = existing_dim[k]
                    if abs(az - (oz + od)) < 0.001:
                        area = get_overlap_area(ax, ay, ax + aw, ay + ah, ox, oy, ox + ow, oy + oh)
                        if area > 0:
                            total_contact_area += area
                            support_indices.append(k)
                            support_areas.append(area)

                if total_contact_area > (aw * ah * 0.5):
                    can_sustain = True
                    for m in range(len(support_indices)):
                        idx_k = support_indices[m]
                        added_load = item_weight * (support_areas[m] / total_contact_area)
                        if (current_loads[idx_k] - existing_weights[idx_k] + added_load) > existing_max_weights[idx_k] + 0.001:
                            can_sustain = False; break
                    if can_sustain:
                        results[i, j] = True
    return results
