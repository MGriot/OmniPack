from typing import List, Tuple, Set
from .models import Item, Container

class ExtremePoint:
    def __init__(self, x: float, y: float, z: float):
        self.x = x
        self.y = y
        self.z = z

    def to_tuple(self) -> Tuple[float, float, float]:
        return (self.x, self.y, self.z)

    def __eq__(self, other):
        if not isinstance(other, ExtremePoint): return False
        return abs(self.x - other.x) < 0.001 and abs(self.y - other.y) < 0.001 and abs(self.z - other.z) < 0.001

    def __hash__(self):
        return hash((round(self.x, 3), round(self.y, 3), round(self.z, 3)))

def generate_extreme_points(container: Container, item: Item) -> List[ExtremePoint]:
    """
    Enhanced EP Generation from all parts of a compound item.
    """
    parts = item.get_rotated_parts() # List of ((rx, ry, rz), (rw, rh, rd))
    ix, iy, iz = item.position
    existing = container.items
    
    potential_pts = []
    for (px, py, pz), (pw, ph, pd) in parts:
        # Global coordinates of this part
        gx, gy, gz = ix + px, iy + py, iz + pz
        
        # 3 Forward points from EACH part
        potential_pts.append([gx + pw, gy, gz])
        potential_pts.append([gx, gy + ph, gz])
        potential_pts.append([gx, gy, gz + pd])
        # 1 Backward point from EACH part (for LIFO discovery)
        potential_pts.append([gx, gy, gz])
    
    # Project each point to find its "stablest" coordinate
    final_pts = []
    # Include existing items expanded into boxes
    expanded_existing = []
    for other in existing:
        for (opx, opy, opz), (opw, oph, opd) in other.get_rotated_parts():
            expanded_existing.append(((other.position[0] + opx, other.position[1] + opy, other.position[2] + opz), (opw, oph, opd)))

    for p in potential_pts:
        px, py, pz = p
        
        # Project PX
        max_x = 0.0
        for (ox, oy, oz), (ow, oh, od) in expanded_existing:
            if ox + ow <= px + 0.001 and (oy < py + 0.001 and oy + oh > py - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_x = max(max_x, ox + ow)
        
        # Project PY
        max_y = 0.0
        for (ox, oy, oz), (ow, oh, od) in expanded_existing:
            if oy + oh <= py + 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_y = max(max_y, oy + oh)

        # Project PZ (Forward)
        max_z = 0.0
        for (ox, oy, oz), (ow, oh, od) in expanded_existing:
            if oz + od <= pz + 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oy < py + 0.001 and oy + oh > py - 0.001):
                max_z = max(max_z, oz + od)
        
        # Project PZ (Backward)
        min_z = container.depth
        for (ox, oy, oz), (ow, oh, od) in expanded_existing:
            if oz >= pz - 0.001 and (ox < px + 0.001 and ox + ow > px - 0.001) and (oy < py + 0.001 and oy + oh > py - 0.001):
                min_z = min(min_z, oz)

        final_pts.append(ExtremePoint(px, py, pz))
        if px > max_x: final_pts.append(ExtremePoint(max_x, py, pz))
        if py > max_y: final_pts.append(ExtremePoint(px, max_y, pz))
        if pz > max_z: final_pts.append(ExtremePoint(px, py, max_z))
        if pz < min_z: final_pts.append(ExtremePoint(px, py, min_z))

    valid_pts = []
    seen = set()
    for p in final_pts:
        if p.x > container.width or p.y > container.height or p.z > container.depth: continue
        if p.x < 0 or p.y < 0 or p.z < 0: continue
        p_round = (round(p.x, 3), round(p.y, 3), round(p.z, 3))
        if p_round not in seen:
            valid_pts.append(p)
            seen.add(p_round)
            
    return valid_pts

def get_valid_ep(container: Container, item: Item, eps: Set[ExtremePoint]) -> List[ExtremePoint]:
    """Rigorous check for valid placement for compound items and containers."""
    valid_eps = []
    expanded_existing = []
    for other in container.items:
        for (opx, opy, opz), (opw, oph, opd) in other.get_rotated_parts():
            expanded_existing.append(((other.position[0] + opx, other.position[1] + opy, other.position[2] + opz), (opw, oph, opd)))

    for ep in eps:
        item.position = ep.to_tuple()
        parts = item.get_rotated_parts()
        
        item_valid = True
        for (px, py, pz), (pw, ph, pd) in parts:
            ax, ay, az = ep.x + px, ep.y + py, ep.z + pz
            
            # Boundary check against container parts
            in_container = False
            for cp in container.parts:
                if (ax >= cp.dx - 0.001 and ax + pw <= cp.dx + cp.width + 0.001 and
                    ay >= cp.dy - 0.001 and ay + ph <= cp.dy + cp.height + 0.001 and
                    az >= cp.dz - 0.001 and az + pd <= cp.dz + cp.depth + 0.001):
                    in_container = True
                    break
            
            if not in_container:
                item_valid = False; break
                
            # Collision check
            collision = False
            for (ox, oy, oz), (ow, oh, od) in expanded_existing:
                if (ax < ox + ow - 0.001 and ax + pw > ox + 0.001 and
                    ay < oy + oh - 0.001 and ay + ph > oy + 0.001 and
                    az < oz + od - 0.001 and az + pd > oz + 0.001):
                    collision = True; break
            if collision:
                item_valid = False; break
        
        if item_valid:
            valid_eps.append(ep)
                
    return valid_eps
