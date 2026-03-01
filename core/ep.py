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
    Enhanced EP Generation (Projected Extreme Points).
    Instead of just 3 points, we discover all stable corners by projecting
    item boundaries against existing geometry.
    """
    w, h, d = item.get_dimension()
    x, y, z = item.position
    existing = container.items
    
    # 1. Start with the 3 canonical points
    potential_pts = [
        [x + w, y, z],
        [x, y + h, z],
        [x, y, z + d]
    ]
    
    # 2. Project each point to find its "stablest" coordinate (nearest support)
    # This prevents the drift where the engine ignores floor gaps.
    final_pts = []
    for p in potential_pts:
        px, py, pz = p
        
        # Project PX: find highest x-bound below/beside it
        max_x = 0.0
        for other in existing:
            ow, oh, od = other.get_dimension()
            ox, oy, oz = other.position
            # If 'other' is behind the point in X and overlaps in Y, Z
            if ox + ow <= px and (oy < py + 0.001 and oy + oh > py - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_x = max(max_x, ox + ow)
        
        # Project PY
        max_y = 0.0
        for other in existing:
            ow, oh, od = other.get_dimension()
            ox, oy, oz = other.position
            if oy + oh <= py and (ox < px + 0.001 and ox + ow > px - 0.001) and (oz < pz + 0.001 and oz + od > pz - 0.001):
                max_y = max(max_y, oy + oh)

        # Project PZ
        max_z = 0.0
        for other in existing:
            ow, oh, od = other.get_dimension()
            ox, oy, oz = other.position
            if oz + od <= pz and (ox < px + 0.001 and ox + ow > px - 0.001) and (oy < py + 0.001 and oy + oh > py - 0.001):
                max_z = max(max_z, oz + od)

        # Add the projected versions to ensure we find "tucked" spots
        final_pts.append(ExtremePoint(px, py, pz))
        if px > max_x: final_pts.append(ExtremePoint(max_x, py, pz))
        if py > max_y: final_pts.append(ExtremePoint(px, max_y, pz))
        if pz > max_z: final_pts.append(ExtremePoint(px, py, max_z))

    # Filter out duplicates and points outside container
    valid_pts = []
    seen = set()
    for p in final_pts:
        if p.x >= container.width or p.y >= container.height or p.z >= container.depth: continue
        p_round = (round(p.x, 3), round(p.y, 3), round(p.z, 3))
        if p_round not in seen:
            valid_pts.append(p)
            seen.add(p_round)
            
    return valid_pts

def get_valid_ep(container: Container, item: Item, eps: Set[ExtremePoint]) -> List[ExtremePoint]:
    """Rigorous check for valid placement including collision and boundaries."""
    valid_eps = []
    w, h, d = item.get_dimension()
    
    for ep in eps:
        # 1. Boundary check
        if (ep.x + w > container.width + 0.001 or 
            ep.y + h > container.height + 0.001 or 
            ep.z + d > container.depth + 0.001):
            continue
            
        # 2. Collision check
        collision = False
        for other in container.items:
            ow, oh, od = other.get_dimension()
            ox, oy, oz = other.position
            
            # AABB intersection with epsilon
            if (ep.x < ox + ow - 0.001 and ep.x + w > ox + 0.001 and
                ep.y < oy + oh - 0.001 and ep.y + h > oy + 0.001 and
                ep.z < oz + od - 0.001 and ep.z + d > oz + 0.001):
                collision = True
                break
        
        if not collision:
            valid_eps.append(ep)
                
    return valid_eps
