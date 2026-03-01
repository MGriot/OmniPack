import numpy as np
from numba import njit, prange

@njit(cache=True)
def check_collision_jit(new_item_pos, new_item_dim, existing_items_pos, existing_items_dim):
    nx, ny, nz = new_item_pos
    nw, nh, nd = new_item_dim
    for i in range(len(existing_items_pos)):
        ox, oy, oz = existing_items_pos[i]
        ow, oh, od = existing_items_dim[i]
        if (nx < ox + ow and nx + nw > ox and
            ny < oy + oh and ny + nh > oy and
            nz < oz + od and nz + nd > oz):
            return True
    return False

@njit(parallel=True, cache=True)
def evaluate_positions_parallel(eps, item_dims, existing_pos, existing_dim, existing_weights, existing_max_weights, container_dim, item_weight):
    """
    Parallel evaluation with collision and stacking weight checks.
    existing_weights: Array of weights of items already in container.
    existing_max_weights: Array of max support weight for each existing item.
    """
    num_eps = len(eps)
    num_rots = len(item_dims)
    cw, ch, cd = container_dim
    results = np.zeros((num_eps, num_rots), dtype=np.bool_)
    
    for i in prange(num_eps):
        ex, ey, ez = eps[i]
        for j in range(num_rots):
            iw, ih, id = item_dims[j]
            
            if (ex + iw <= cw and ey + ih <= ch and ez + id <= cd):
                collision = False
                # Check if we are placing ON TOP of something
                # and if that something can support our weight.
                
                for k in range(len(existing_pos)):
                    ox, oy, oz = existing_pos[k]
                    ow, oh, od = existing_dim[k]
                    
                    # 1. Standard Collision
                    if (ex < ox + ow and ex + iw > ox and
                        ey < oy + oh and ey + ih > oy and
                        ez < oz + od and ez + id > oz):
                        collision = True
                        break
                    
                    # 2. Stacking check: If our base (Z=ez) is exactly on top of another item's top (Z=oz+od)
                    # AND we overlap in X and Y
                    if (abs(ez - (oz + od)) < 0.001): # Precisely on top
                        if (ex < ox + ow and ex + iw > ox and
                            ey < oy + oh and ey + ih > oy):
                            # We are putting weight on item K
                            if item_weight > existing_max_weights[k]:
                                collision = True # Treat as 'invalid' placement
                                break
                
                if not collision:
                    results[i, j] = True
                    
    return results
