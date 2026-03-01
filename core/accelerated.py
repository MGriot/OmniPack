import numpy as np
from numba import njit, prange

@njit(cache=True)
def check_collision_jit(new_item_pos, new_item_dim, existing_items_pos, existing_items_dim):
    """
    JIT-accelerated AABB collision detection.
    new_item_pos: (x, y, z)
    new_item_dim: (w, h, d)
    existing_items_pos: Nx3 array
    existing_items_dim: Nx3 array
    """
    nx, ny, nz = new_item_pos
    nw, nh, nd = new_item_dim
    
    for i in range(len(existing_items_pos)):
        ox, oy, oz = existing_items_pos[i]
        ow, oh, od = existing_items_dim[i]
        
        # AABB Collision logic
        if (nx < ox + ow and nx + nw > ox and
            ny < oy + oh and ny + nh > oy and
            nz < oz + od and nz + nd > oz):
            return True
    return False

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, container_dim):
    """
    Parallel evaluation of multiple possible placements (EPs x Rotations).
    Returns indices of valid placements.
    """
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    
    # Store results as a flattened 2D array: (ep_idx, rot_idx)
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    
    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            iw, ih, id = item_dims[j]
            
            # Boundary check
            if (ex + iw <= cw and ey + ih <= ch and ez + id <= cd):
                # Collision check
                collision = False
                for k in range(len(existing_pos)):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    if (ex < ox + ow and ex + iw > ox and
                        ey < oy + oh and ey + ih > oy and
                        ez < oz + od and ez + id > oz):
                        collision = True
                        break
                
                if not collision:
                    results[i, j] = True
                    
    return results
